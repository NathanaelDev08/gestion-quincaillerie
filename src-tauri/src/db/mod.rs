use crate::error::AppResult;
use sqlx::{
    sqlite::{SqliteConnection, SqlitePoolOptions},
    Pool, Sqlite, Transaction,
};
use tauri::{AppHandle, Manager};

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(app: &AppHandle) -> AppResult<DbPool> {
    let dir = app
        .path()
        .app_data_dir()
        .expect("app data dir");
    std::fs::create_dir_all(&dir).ok();
    let db_path = dir.join("gestion.db");
    let url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    run_migrations(&pool).await?;

    // Sauvegarde automatique quotidienne (conserve les 7 dernières)
    if let Err(e) = auto_backup(app, &pool).await {
        eprintln!("[DB] Sauvegarde auto ignorée: {}", e);
    }

    // Enregistrer le pool dans l'état Tauri (si absent)
    // Note: manage est fait dans main via .manage(pool.clone())
    Ok(pool)
}

/// Copie la base vers Documents/GestionQuincaillerie/backups une fois par jour.
async fn auto_backup(app: &AppHandle, pool: &DbPool) -> Result<(), String> {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let dst_dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("GestionQuincaillerie/backups");
    std::fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
    let already: bool = std::fs::read_dir(&dst_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with(&format!("auto-{today}")));
    if already {
        return Ok(());
    }
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    for suffix in ["gestion.db", "gestion.db-wal", "gestion.db-shm"] {
        let src = dir.join(suffix);
        if src.exists() {
            std::fs::copy(&src, dst_dir.join(format!("auto-{today}-{suffix}")))
                .map_err(|e| e.to_string())?;
        }
    }
    // Rotation : garder les 7 sauvegardes auto les plus récentes
    let mut autos: Vec<_> = std::fs::read_dir(&dst_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("auto-"))
        .collect();
    autos.sort_by_key(|e| e.file_name());
    while autos.len() > 7 * 3 {
        if let Some(old) = autos.first() {
            let _ = std::fs::remove_file(old.path());
        }
        autos.remove(0);
    }
    eprintln!("[DB] Sauvegarde auto du jour effectuée");
    Ok(())
}

/// Sessions rafraîchissables (jetons révocables) — migration v7, idempotente.
async fn run_migrations_v7(pool: &DbPool) -> AppResult<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions_refresh (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            expire_at TEXT NOT NULL,
            cree_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions_refresh(user_id)")
        .execute(pool)
        .await?;
    let _ = sqlx::query("DELETE FROM sessions_refresh WHERE expire_at < datetime('now')")
        .execute(pool)
        .await;
    Ok(())
}

async fn run_migrations(pool: &DbPool) -> AppResult<()> {
    let schema: &str = r#"
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            full_name TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'admin',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS clients (
            id TEXT PRIMARY KEY,
            nom TEXT NOT NULL,
            prenom TEXT,
            entreprise TEXT,
            email TEXT,
            telephone TEXT,
            adresse TEXT,
            ville TEXT,
            code_postal TEXT,
            pays TEXT DEFAULT 'France',
            siret TEXT,
            tva_intra TEXT,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_clients_nom ON clients(nom);

        CREATE TABLE IF NOT EXISTS fournisseurs (
            id TEXT PRIMARY KEY,
            nom TEXT NOT NULL,
            entreprise TEXT,
            email TEXT,
            telephone TEXT,
            adresse TEXT,
            ville TEXT,
            code_postal TEXT,
            pays TEXT DEFAULT 'France',
            siret TEXT,
            tva_intra TEXT,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS produits (
            id TEXT PRIMARY KEY,
            reference TEXT UNIQUE NOT NULL,
            designation TEXT NOT NULL,
            description TEXT,
            prix_achat_ht REAL NOT NULL DEFAULT 0,
            prix_vente_ht REAL NOT NULL DEFAULT 0,
            taux_tva REAL NOT NULL DEFAULT 20,
            stock REAL NOT NULL DEFAULT 0,
            stock_alerte REAL NOT NULL DEFAULT 0,
            categorie TEXT,
            unite TEXT DEFAULT 'pcs',
            code_barre TEXT,
            actif INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_produits_ref ON produits(reference);

        CREATE TABLE IF NOT EXISTS mouvements_stock (
            id TEXT PRIMARY KEY,
            produit_id TEXT NOT NULL REFERENCES produits(id),
            type TEXT NOT NULL,
            quantite REAL NOT NULL,
            stock_avant REAL NOT NULL,
            stock_apres REAL NOT NULL,
            motif TEXT,
            document_ref TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS devis (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            client_id TEXT NOT NULL REFERENCES clients(id),
            date_emission TEXT NOT NULL,
            date_validite TEXT NOT NULL,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            total_ht REAL NOT NULL DEFAULT 0,
            total_tva REAL NOT NULL DEFAULT 0,
            total_ttc REAL NOT NULL DEFAULT 0,
            remise REAL NOT NULL DEFAULT 0,
            notes TEXT,
            conditions TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS factures (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            client_id TEXT NOT NULL REFERENCES clients(id),
            devis_id TEXT REFERENCES devis(id),
            date_emission TEXT NOT NULL,
            date_echeance TEXT NOT NULL,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            total_ht REAL NOT NULL DEFAULT 0,
            total_tva REAL NOT NULL DEFAULT 0,
            total_ttc REAL NOT NULL DEFAULT 0,
            montant_paye REAL NOT NULL DEFAULT 0,
            remise REAL NOT NULL DEFAULT 0,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS lignes_document (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            document_type TEXT NOT NULL,
            produit_id TEXT REFERENCES produits(id),
            designation TEXT NOT NULL,
            quantite REAL NOT NULL DEFAULT 1,
            prix_unitaire_ht REAL NOT NULL DEFAULT 0,
            taux_tva REAL NOT NULL DEFAULT 20,
            remise REAL NOT NULL DEFAULT 0,
            total_ht REAL NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_lignes_doc ON lignes_document(document_id);

        CREATE TABLE IF NOT EXISTS reglements (
            id TEXT PRIMARY KEY,
            facture_id TEXT NOT NULL REFERENCES factures(id),
            montant REAL NOT NULL,
            mode TEXT NOT NULL DEFAULT 'virement',
            date_reglement TEXT NOT NULL,
            reference TEXT,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS exercices (
            id TEXT PRIMARY KEY,
            libelle TEXT NOT NULL,
            date_debut TEXT NOT NULL,
            date_fin TEXT NOT NULL,
            cloture INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS comptes (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            intitule TEXT NOT NULL,
            classe TEXT NOT NULL,
            type TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS journaux (
            id TEXT PRIMARY KEY,
            code TEXT UNIQUE NOT NULL,
            libelle TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ecritures (
            id TEXT PRIMARY KEY,
            journal_id TEXT NOT NULL REFERENCES journaux(id),
            compte_id TEXT NOT NULL REFERENCES comptes(id),
            date_ecriture TEXT NOT NULL,
            libelle TEXT NOT NULL,
            debit REAL NOT NULL DEFAULT 0,
            credit REAL NOT NULL DEFAULT 0,
            piece_ref TEXT,
            facture_id TEXT REFERENCES factures(id),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS sequences (
            code TEXT PRIMARY KEY,
            annee TEXT NOT NULL,
            compteur INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            action TEXT NOT NULL,
            entity TEXT NOT NULL,
            entity_id TEXT,
            details TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#;
    for stmt in schema.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }

    // Données de base : journaux + plan comptable simplifié
    sqlx::query(
        r#"INSERT OR IGNORE INTO journaux (id, code, libelle) VALUES
        ('j-vte','VTE','Ventes'),('j-ach','ACH','Achats'),
        ('j-bq','BQ','Banque'),('j-caisse','CAISSE','Caisse'),
        ('j-od','OD','Opérations diverses')"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"INSERT OR IGNORE INTO comptes (id, numero, intitule, classe, type) VALUES
        ('c411','411000','Clients','4','actif'),
        ('c401','401000','Fournisseurs','4','passif'),
        ('c707','707000','Ventes marchandises','7','produit'),
        ('c607','607000','Achats marchandises','6','charge'),
        ('c44571','445710','TVA collectée','4','passif'),
        ('c44566','445660','TVA déductible','4','actif'),
        ('c512','512000','Banque','5','actif'),
        ('c531','531000','Caisse','5','actif'),
        ('c758','758000','Produits divers','7','produit'),
        ('c658','658000','Charges diverses','6','charge')"#,
    )
    .execute(pool)
    .await?;

    // Premier compte : AUCUN compte n'est créé automatiquement.
    //
    // Deux pièges évités :
    // 1. un mot de passe admin connu par défaut (admin/admin123) donne le
    //    contrôle du poste à quiconque a la main dessus ;
    // 2. un mot de passe généré affiché dans l'application est inaccessible
    //    sans déjà être connecté — l'utilisateur se retrouve bloqué.
    //
    // L'application affiche donc un écran de première installation où
    // l'utilisateur choisit lui-même son identifiant et son mot de passe.
    let (user_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count == 0 {
        crate::diagnostics::avertissement(
            "Premier démarrage",
            "Aucun compte existant : l'application demande de créer le compte administrateur.",
        );
    }
    let users: Vec<(String, String)> =
        sqlx::query_as("SELECT username, role FROM users")
            .fetch_all(pool)
            .await?;
    eprintln!("[DB] Utilisateurs ({}): {:?}", users.len(), users);

    run_migrations_v2(pool).await?;

    Ok(())
}

/// Migration v2 : avoirs, achats, dépenses, relances (idempotente)
async fn run_migrations_v2(pool: &DbPool) -> AppResult<()> {
    const V2: &str = r#"
        CREATE TABLE IF NOT EXISTS avoirs (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            facture_id TEXT REFERENCES factures(id),
            client_id TEXT NOT NULL REFERENCES clients(id),
            date_emission TEXT NOT NULL,
            motif TEXT,
            total_ht REAL NOT NULL DEFAULT 0,
            total_tva REAL NOT NULL DEFAULT 0,
            total_ttc REAL NOT NULL DEFAULT 0,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS lignes_avoir (
            id TEXT PRIMARY KEY,
            avoir_id TEXT NOT NULL REFERENCES avoirs(id),
            designation TEXT NOT NULL,
            quantite REAL NOT NULL DEFAULT 1,
            prix_unitaire_ht REAL NOT NULL DEFAULT 0,
            taux_tva REAL NOT NULL DEFAULT 20,
            total_ht REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS commandes_fournisseurs (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            fournisseur_id TEXT NOT NULL REFERENCES fournisseurs(id),
            date_commande TEXT NOT NULL,
            date_livraison_prevue TEXT,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            total_ht REAL NOT NULL DEFAULT 0,
            total_ttc REAL NOT NULL DEFAULT 0,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS lignes_commande (
            id TEXT PRIMARY KEY,
            commande_id TEXT NOT NULL REFERENCES commandes_fournisseurs(id),
            produit_id TEXT REFERENCES produits(id),
            designation TEXT NOT NULL,
            quantite REAL NOT NULL DEFAULT 1,
            quantite_recue REAL NOT NULL DEFAULT 0,
            prix_unitaire_ht REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS depenses (
            id TEXT PRIMARY KEY,
            libelle TEXT NOT NULL,
            categorie TEXT NOT NULL DEFAULT 'divers',
            montant REAL NOT NULL DEFAULT 0,
            date_depense TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'especes',
            fournisseur_id TEXT REFERENCES fournisseurs(id),
            piece_ref TEXT,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#;
    for stmt in V2.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }
    // Colonne relance (ignore l'erreur si elle existe déjà)
    let _ = sqlx::query("ALTER TABLE factures ADD COLUMN derniere_relance TEXT")
        .execute(pool)
        .await;
    run_migrations_v3(pool).await?;
    Ok(())
}

/// Migration v3 : livraison, paie, clôtures caisse (idempotente)
async fn run_migrations_v3(pool: &DbPool) -> AppResult<()> {
    const V3: &str = r#"
        CREATE TABLE IF NOT EXISTS bons_livraison (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            client_id TEXT NOT NULL REFERENCES clients(id),
            devis_id TEXT REFERENCES devis(id),
            date_livraison TEXT NOT NULL,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS lignes_livraison (
            id TEXT PRIMARY KEY,
            bl_id TEXT NOT NULL REFERENCES bons_livraison(id),
            produit_id TEXT REFERENCES produits(id),
            designation TEXT NOT NULL,
            quantite REAL NOT NULL DEFAULT 1,
            prix_unitaire_ht REAL NOT NULL DEFAULT 0,
            taux_tva REAL NOT NULL DEFAULT 18
        );
        CREATE TABLE IF NOT EXISTS employes (
            id TEXT PRIMARY KEY,
            nom TEXT NOT NULL,
            prenom TEXT,
            poste TEXT,
            telephone TEXT,
            salaire_base REAL NOT NULL DEFAULT 0,
            date_embauche TEXT,
            actif INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS bulletins (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            employe_id TEXT NOT NULL REFERENCES employes(id),
            periode TEXT NOT NULL,
            brut REAL NOT NULL,
            cnps REAL NOT NULL,
            its REAL NOT NULL,
            net REAL NOT NULL,
            statut TEXT NOT NULL DEFAULT 'brouillon',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS clotures_z (
            id TEXT PRIMARY KEY,
            date TEXT UNIQUE NOT NULL,
            nb_ventes INTEGER NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            especes REAL NOT NULL DEFAULT 0,
            virement REAL NOT NULL DEFAULT 0,
            cheque REAL NOT NULL DEFAULT 0,
            cb REAL NOT NULL DEFAULT 0,
            mobile REAL NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#;
    for stmt in V3.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }
    run_migrations_v4(pool).await?;
    run_migrations_v5(pool).await?;
    run_migrations_v6(pool).await?;
    run_migrations_v7(pool).await?;
    Ok(())
}

/// Migration v5 : dettes fournisseurs, fidélité, promos, plan OHADA (idempotente)
async fn run_migrations_v5(pool: &DbPool) -> AppResult<()> {
    const V5: &str = r#"
        CREATE TABLE IF NOT EXISTS factures_fournisseurs (
            id TEXT PRIMARY KEY,
            numero TEXT UNIQUE NOT NULL,
            fournisseur_id TEXT NOT NULL REFERENCES fournisseurs(id),
            date_emission TEXT NOT NULL,
            date_echeance TEXT NOT NULL,
            statut TEXT NOT NULL DEFAULT 'emise',
            total_ht REAL NOT NULL DEFAULT 0,
            total_tva REAL NOT NULL DEFAULT 0,
            total_ttc REAL NOT NULL DEFAULT 0,
            montant_paye REAL NOT NULL DEFAULT 0,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS reglements_fournisseurs (
            id TEXT PRIMARY KEY,
            facture_id TEXT NOT NULL REFERENCES factures_fournisseurs(id),
            montant REAL NOT NULL,
            mode TEXT NOT NULL DEFAULT 'virement',
            date_reglement TEXT NOT NULL,
            reference TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS promos (
            id TEXT PRIMARY KEY,
            code TEXT UNIQUE NOT NULL,
            type TEXT NOT NULL DEFAULT 'pourcent',
            valeur REAL NOT NULL DEFAULT 0,
            date_debut TEXT NOT NULL,
            date_fin TEXT NOT NULL,
            actif INTEGER NOT NULL DEFAULT 1
        );
        "#;
    for stmt in V5.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }
    let _ = sqlx::query("ALTER TABLE clients ADD COLUMN points REAL DEFAULT 0")
        .execute(pool)
        .await;
    // Plan comptable OHADA/SYSCOHADA complémentaire (INSERT OR IGNORE)
    sqlx::query(
        r#"INSERT OR IGNORE INTO comptes (id, numero, intitule, classe, type) VALUES
        ('o101','101000','Capital social','1','passif'),
        ('o106','106000','Réserves','1','passif'),
        ('o131','131000','Résultat net','1','passif'),
        ('o244','244100','Matériel et mobilier','2','actif'),
        ('o311','311000','Stocks de marchandises','3','actif'),
        ('o401','401000','Fournisseurs','4','passif'),
        ('o411','411000','Clients','4','actif'),
        ('o443','443000','TVA facturée','4','passif'),
        ('o4456','445660','TVA déductible','4','actif'),
        ('o521','521000','Banques locales','5','actif'),
        ('o571','571000','Caisse','5','actif'),
        ('o601','601000','Achats de marchandises','6','charge'),
        ('o641','641000','Charges de personnel','6','charge'),
        ('o658','658000','Charges diverses','6','charge'),
        ('o701','701000','Ventes de marchandises','7','produit'),
        ('o758','758000','Produits divers','7','produit')"#,
    )
    .execute(pool)
    .await?;
    run_migrations_v6(pool).await?;
    Ok(())
}

/// Migration v4 : index de montée en charge (idempotente)
async fn run_migrations_v4(pool: &DbPool) -> AppResult<()> {
    const V4: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_factures_client ON factures(client_id);
        CREATE INDEX IF NOT EXISTS idx_factures_date ON factures(date_emission);
        CREATE INDEX IF NOT EXISTS idx_factures_statut ON factures(statut);
        CREATE INDEX IF NOT EXISTS idx_devis_client ON devis(client_id);
        CREATE INDEX IF NOT EXISTS idx_devis_statut ON devis(statut);
        CREATE INDEX IF NOT EXISTS idx_reglements_facture ON reglements(facture_id);
        CREATE INDEX IF NOT EXISTS idx_mvt_produit ON mouvements_stock(produit_id);
        CREATE INDEX IF NOT EXISTS idx_ecr_facture ON ecritures(facture_id);
        CREATE INDEX IF NOT EXISTS idx_ecr_compte ON ecritures(compte_id);
        CREATE INDEX IF NOT EXISTS idx_ecr_journal ON ecritures(journal_id);
        CREATE INDEX IF NOT EXISTS idx_ecr_date ON ecritures(date_ecriture);
        CREATE INDEX IF NOT EXISTS idx_lignes_cmd ON lignes_commande(commande_id);
        CREATE INDEX IF NOT EXISTS idx_lignes_bl ON lignes_livraison(bl_id);
        CREATE INDEX IF NOT EXISTS idx_lignes_avoir ON lignes_avoir(avoir_id);
        CREATE INDEX IF NOT EXISTS idx_depenses_date ON depenses(date_depense);
        CREATE INDEX IF NOT EXISTS idx_bulletins_emp ON bulletins(employe_id);
        CREATE INDEX IF NOT EXISTS idx_bulletins_periode ON bulletins(periode);
        CREATE INDEX IF NOT EXISTS idx_bl_client ON bons_livraison(client_id);
        CREATE INDEX IF NOT EXISTS idx_avoirs_client ON avoirs(client_id);
        CREATE INDEX IF NOT EXISTS idx_cmd_fournisseur ON commandes_fournisseurs(fournisseur_id);
        "#;
    for stmt in V4.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

/// Migration v6 : journal d'audit + multi-dépôts (idempotente ettolérante)
async fn run_migrations_v6(pool: &DbPool) -> AppResult<()> {
    // --- audit_log ---
    // ATTENTION : une installation existante peut déjà posséder une table
    // `audit_log` avec un schéma différent (ancienne version). `CREATE TABLE
    // IF NOT EXISTS` ne modifierait rien et les INSERT échoueraient ensuite.
    // On aligne donc les colonnes une par une, en tolérant leur absence.
    let existe: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='audit_log'")
            .fetch_one(pool)
            .await?;
    if existe.0 == 0 {
        sqlx::query(
            "CREATE TABLE audit_log (
                id TEXT PRIMARY KEY,
                utilisateur TEXT,
                role TEXT,
                action TEXT NOT NULL,
                entite TEXT,
                entite_id TEXT,
                detail TEXT,
                montant REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(pool)
        .await?;
    } else {
        let colonnes: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('audit_log')")
                .fetch_all(pool)
                .await?;
        let a: Vec<String> = colonnes.into_iter().map(|c| c.0).collect();
        // Colonnesrequired côté code d'audit ; ajoutées si absentes.
        for (nom, ddl) in [
            ("utilisateur", "ALTER TABLE audit_log ADD COLUMN utilisateur TEXT"),
            ("role", "ALTER TABLE audit_log ADD COLUMN role TEXT"),
            ("entite", "ALTER TABLE audit_log ADD COLUMN entite TEXT"),
            ("entite_id", "ALTER TABLE audit_log ADD COLUMN entite_id TEXT"),
            ("detail", "ALTER TABLE audit_log ADD COLUMN detail TEXT"),
            ("montant", "ALTER TABLE audit_log ADD COLUMN montant REAL"),
            (
                "created_at",
                "ALTER TABLE audit_log ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP",
            ),
        ] {
            if !a.iter().any(|c| c == nom) {
                // Un échec ici ne doit pas bloquer le démarrage de l'app.
                let _ = sqlx::query(ddl).execute(pool).await;
            }
        }
    }
    for idx in [
        "CREATE INDEX IF NOT EXISTS idx_audit_date ON audit_log(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(utilisateur)",
        "CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action)",
    ] {
        let _ = sqlx::query(idx).execute(pool).await;
    }

    const V6: &str = r#"
        CREATE TABLE IF NOT EXISTS depots (
            id TEXT PRIMARY KEY,
            nom TEXT UNIQUE NOT NULL,
            adresse TEXT,
            actif INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS stock_depot (
            depot_id TEXT NOT NULL,
            produit_id TEXT NOT NULL,
            quantite REAL NOT NULL DEFAULT 0,
            PRIMARY KEY (depot_id, produit_id)
        );
        CREATE TABLE IF NOT EXISTS mouvements_depot (
            id TEXT PRIMARY KEY,
            depot_id TEXT NOT NULL,
            produit_id TEXT NOT NULL,
            type TEXT NOT NULL,
            quantite REAL NOT NULL,
            stock_avant REAL NOT NULL,
            stock_apres REAL NOT NULL,
            motif TEXT,
            document_ref TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_mvtdepot ON mouvements_depot(depot_id);
        CREATE INDEX IF NOT EXISTS idx_stockdepot ON stock_depot(produit_id);
        "#;
    for stmt in V6.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await?;
    }
    // Dépôt principal par défaut (le stock existant reste sur le dépôt principal)
    let depot_id = "depot-principal";
    sqlx::query("INSERT OR IGNORE INTO depots (id, nom, adresse, actif) VALUES (?,?,?,1)")
        .bind(depot_id)
        .bind("Dépôt principal")
        .bind("")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO stock_depot (depot_id, produit_id, quantite)
         SELECT ?, id, stock FROM produits",
    )
    .bind(depot_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Journalise une action dans `audit_log` (silencieux : ne bloque jamais l'opération métier).
pub async fn audit(
    pool: &DbPool,
    token: &str,
    app: &tauri::AppHandle,
    action: &str,
    entite: &str,
    entite_id: &str,
    detail: &str,
    montant: Option<f64>,
) {
    let (user, role) = match crate::auth::verify_token(token, &crate::authz::jwt_secret(app)) {
        Ok(c) => (c.username, c.role),
        Err(_) => ("inconnu".to_string(), "?".to_string()),
    };
    let _ = sqlx::query(
        "INSERT INTO audit_log (id,utilisateur,role,action,entite,entite_id,detail,montant) VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&user)
    .bind(&role)
    .bind(action)
    .bind(entite)
    .bind(entite_id)
    .bind(detail)
    .bind(montant)
    .execute(pool)
    .await;
}

async fn next_numero_conn(conn: &mut SqliteConnection, prefix: &str) -> Result<String, sqlx::Error> {
    let annee = chrono::Utc::now().format("%Y").to_string();
    let code = format!("{prefix}-{annee}");
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT compteur FROM sequences WHERE code = ?")
            .bind(&code)
            .fetch_optional(&mut *conn)
            .await?;
    let next = row.map(|r| r.0 + 1).unwrap_or(1);
    sqlx::query(
        "INSERT INTO sequences (code, annee, compteur) VALUES (?,?,?) ON CONFLICT(code) DO UPDATE SET compteur=excluded.compteur",
    )
    .bind(&code)
    .bind(&annee)
    .bind(next)
    .execute(&mut *conn)
    .await?;
    Ok(format!("{}-{}-{:04}", prefix, annee, next))
}

/// Numéro séquentiel (pool). Utilisé hors transaction.
pub async fn next_numero(pool: &DbPool, prefix: &str) -> Result<String, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    next_numero_conn(&mut conn, prefix).await
}

/// Numéro séquentiel à l'intérieur d'une transaction : si la transaction est
/// annulée, le compteur n'est pas consommé (pas de trou dans la numérotation).
pub async fn next_numero_tx(
    tx: &mut Transaction<'_, Sqlite>,
    prefix: &str,
) -> Result<String, sqlx::Error> {
    next_numero_conn(&mut **tx, prefix).await
}

#[cfg(test)]
mod tests_migration {
    /// Régression : une installation existante peut déjà avoir une table
    /// `audit_log` à l'ancien schéma. `CREATE TABLE IF NOT EXISTS` ne corrige
    /// rien et l'INSERT d'audit échoue ensuite sur « no such column ».
    /// Ce test fige le comportement attendu de la migration v6.
    #[tokio::test]
    async fn v6_aligne_une_ancienne_table_audit_sans_perdre_les_donnees() {
        use sqlx::Row;

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");

        // Ancien schéma, comme sur la base d'un utilisateur déjà installé.
        sqlx::query(
            "CREATE TABLE audit_log (
                id TEXT PRIMARY KEY, user_id TEXT, action TEXT NOT NULL,
                entity TEXT, entity_id TEXT, details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO audit_log (id,user_id,action,entity,entity_id,details)
             VALUES ('a1','u1','vente','facture','F-1','avant')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // La migration copie aussi le stock vers le dépôt principal :
        // la table produits doit donc exister.
        sqlx::query("CREATE TABLE produits (id TEXT PRIMARY KEY, stock REAL)")
            .execute(&pool)
            .await
            .unwrap();

        // Rejoue la portion audit de la migration.
        super::run_migrations_v6(&pool).await.expect("migration v6");

        let colonnes: Vec<String> = sqlx::query("SELECT name FROM pragma_table_info('audit_log')")
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.get::<String, _>(0))
            .collect();
        for c in ["utilisateur", "role", "entite", "entite_id", "detail", "montant", "created_at"] {
            assert!(colonnes.iter().any(|x| x == c), "colonne manquante après migration : {c}");
        }

        // L'écriture d'audit doit de nouveau fonctionner (c'était le bug).
        sqlx::query(
            "INSERT INTO audit_log (id,utilisateur,role,action,entite,entite_id,detail,montant)
             VALUES ('a2','caissier','commercial','vente','facture','F-2','apres',1500.0)",
        )
        .execute(&pool)
        .await
        .expect("insertion d'audit après migration");

        // Aucune perte de données historiques.
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n.0, 2, "les lignes existantes doivent être préservées");
    }

    /// Une base vierge doit obtenir le schéma complet, dépôt principal compris.
    #[tokio::test]
    async fn v6_cree_le_schema_complet_et_le_depot_principal() {
        use sqlx::Row;

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        // Table produits minimale : la migration y copie le stock initial.
        sqlx::query("CREATE TABLE produits (id TEXT PRIMARY KEY, stock REAL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO produits (id,stock) VALUES ('p1',42.0)")
            .execute(&pool)
            .await
            .unwrap();

        super::run_migrations_v6(&pool).await.expect("migration v6");

        // Idempotence : un second passage ne doit rien casser.
        super::run_migrations_v6(&pool).await.expect("migration v6 rejouée");

        let depots: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM depots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(depots.0, 1, "le dépôt principal doit être créé");

        let stock: Vec<(String, String, f64)> =
            sqlx::query_as("SELECT depot_id, produit_id, quantite FROM stock_depot")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(stock.len(), 1, "le stock existant doit être repris dans le dépôt");
        assert_eq!(stock[0].2, 42.0, "le stock initial doit être conservé");
        assert_eq!(stock[0].0, "depot-principal");

        // Pas de doublon après le second passage.
        let lignes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stock_depot")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(lignes.0, 1);

        let _unused: Vec<String> = sqlx::query("SELECT name FROM sqlite_master")
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.get::<String, _>(0))
            .collect();
    }
}

#[cfg(test)]
mod tests_numero {
    use super::*;

    async fn base_test() -> DbPool {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        sqlx::query("CREATE TABLE sequences (code TEXT PRIMARY KEY, annee TEXT, compteur INTEGER)")
            .execute(&db)
            .await
            .expect("table");
        db
    }

    #[tokio::test]
    async fn numero_sincremente_chaque_appel() {
        let db = base_test().await;
        let annee = chrono::Utc::now().format("%Y").to_string();

        let n1 = next_numero(&db, "FAC").await.unwrap();
        let n2 = next_numero(&db, "FAC").await.unwrap();
        let n3 = next_numero(&db, "DEV").await.unwrap();

        assert_eq!(n1, format!("FAC-{annee}-0001"));
        assert_eq!(n2, format!("FAC-{annee}-0002"));
        assert_eq!(n3, format!("DEV-{annee}-0001"));
    }

    /// Un rollback ne doit pas brûler de numéro : garantie d'une numérotation
    /// continue, exigée par les fiscalités de facturation.
    #[tokio::test]
    async fn rollback_ne_consomme_pas_de_numero() {
        let db = base_test().await;
        let annee = chrono::Utc::now().format("%Y").to_string();

        let mut tx = db.begin().await.unwrap();
        let _brouille = next_numero_tx(&mut tx, "FAC").await.unwrap();
        tx.rollback().await.unwrap();

        let apres = next_numero(&db, "FAC").await.unwrap();
        assert_eq!(apres, format!("FAC-{annee}-0001"), "le numéro 1 doit être réutilisé");
    }
}
