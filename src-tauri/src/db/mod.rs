use crate::error::AppResult;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
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

/// Copie la base vers Documents/GestionCommerciale/backups une fois par jour.
async fn auto_backup(app: &AppHandle, pool: &DbPool) -> Result<(), String> {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let dst_dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("GestionCommerciale/backups");
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

pub async fn run_migrations(pool: &DbPool) -> AppResult<()> {
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

    // Compte de test par défaut (créé uniquement si aucun utilisateur n'existe)
    let (user_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        let hash = crate::auth::hash_password("admin123")?;
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, full_name, email, role) VALUES (?,?,?,?,?,?)",
        )
        .bind(&id)
        .bind("admin")
        .bind(&hash)
        .bind("Administrateur")
        .bind("admin@demo.local")
        .bind("admin")
        .execute(pool)
        .await?;
        eprintln!("[DB] Compte test 'admin' créé");
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

pub async fn next_numero(pool: &DbPool, prefix: &str) -> Result<String, sqlx::Error> {
    let annee = chrono::Utc::now().format("%Y").to_string();
    let code = format!("{prefix}-{annee}");
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT compteur FROM sequences WHERE code = ?")
            .bind(&code)
            .fetch_optional(pool)
            .await?;
    let next = row.map(|r| r.0 + 1).unwrap_or(1);
    sqlx::query(
        "INSERT INTO sequences (code, annee, compteur) VALUES (?,?,?) ON CONFLICT(code) DO UPDATE SET compteur=excluded.compteur",
    )
    .bind(&code)
    .bind(&annee)
    .bind(next)
    .execute(pool)
    .await?;
    Ok(format!("{}-{}-{:04}", prefix, annee, next))
}
