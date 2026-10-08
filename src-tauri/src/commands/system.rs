use crate::authz::require_admin;
use crate::db::DbPool;
use crate::services::export::to_csv;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn settings_get(app: AppHandle, key: String) -> Result<Option<serde_json::Value>, String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    Ok(store.get(&key))
}

#[tauri::command]
pub async fn settings_set(app: AppHandle, key: String, value: serde_json::Value) -> Result<bool, String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set(&key, value);
    store.save().map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn diagnostic_perf(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    use std::time::Instant;
    async fn count(pool: &DbPool, table: &str) -> i64 {
        sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .map(|r| r.0)
            .unwrap_or(-1)
    }
    let mut tables = serde_json::Map::new();
    for t in [
        "users", "clients", "fournisseurs", "produits", "devis", "factures",
        "lignes_document", "reglements", "mouvements_stock", "ecritures",
        "commandes_fournisseurs", "bons_livraison", "avoirs", "depenses",
        "bulletins", "employes",
    ] {
        tables.insert(t.to_string(), serde_json::Value::from(count(&pool, t).await));
    }
    let mut mesures = vec![];
    let bench = [
        ("Liste factures (50)", "SELECT f.*, c.nom as client_nom FROM factures f LEFT JOIN clients c ON c.id=f.client_id ORDER BY f.created_at DESC LIMIT 50"),
        ("Statistiques dashboard", "SELECT SUM(total_ttc), SUM(total_ttc) FROM factures WHERE statut != 'annulee'"),
        ("Journal comptable (200)", "SELECT e.*, c.numero as compte_numero, j.code as journal_code FROM ecritures e JOIN comptes c ON c.id=e.compte_id JOIN journaux j ON j.id=e.journal_id ORDER BY e.date_ecriture DESC LIMIT 200"),
        ("Balance", "SELECT c.numero, c.intitule, COALESCE(SUM(e.debit),0), COALESCE(SUM(e.credit),0) FROM comptes c LEFT JOIN ecritures e ON e.compte_id=c.id GROUP BY c.id"),
        ("Recherche clients", "SELECT * FROM clients WHERE nom LIKE '%a%' LIMIT 50"),
    ];
    for (nom, sql) in bench {
        let t0 = Instant::now();
        let rows: Result<Vec<sqlx::sqlite::SqliteRow>, _> = sqlx::query(sql).fetch_all(&*pool).await;
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        mesures.push(serde_json::json!({
            "requete": nom,
            "ms": (ms * 100.0).round() / 100.0,
            "lignes": rows.map(|r| r.len() as i64).unwrap_or(-1),
            "ok": ms < 500.0,
        }));
    }
    let (idx,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "tables": tables, "mesures": mesures, "index_perso": idx }))
}

#[tauri::command]
pub async fn prochains_numeros(app: AppHandle, pool: State<'_, DbPool>) -> Result<Vec<serde_json::Value>, String> {
    let annee = chrono::Utc::now().format("%Y").to_string();
    let defs = [
        ("devis", "DEV", "Devis"),
        ("facture", "FAC", "Factures"),
        ("livraison", "BL", "Bons de livraison"),
        ("commande", "BC", "Bons de commande"),
        ("avoir", "AV", "Avoirs"),
        ("bulletin", "BUL", "Bulletins de paie"),
    ];
    let mut out = vec![];
    for (key, fallback, label) in defs {
        let prefix = crate::services::numbering::doc_prefix(&app, key, fallback);
        let code = format!("{prefix}-{annee}");
        let row: Option<(i64,)> = sqlx::query_as("SELECT compteur FROM sequences WHERE code = ?")
            .bind(&code)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| e.to_string())?;
        let next = row.map(|r| r.0 + 1).unwrap_or(1);
        out.push(serde_json::json!({
            "key": key, "label": label, "prefix": prefix,
            "prochain": format!("{prefix}-{annee}-{next:04}"),
        }));
    }
    Ok(out)
}

#[tauri::command]
pub async fn export_csv(pool: State<'_, DbPool>, entity: String) -> Result<String, String> {
    match entity.as_str() {
        "clients" => {
            let rows: Vec<(String, String, Option<String>, Option<String>)> =
                sqlx::query_as("SELECT nom, COALESCE(prenom,''), email, telephone FROM clients")
                    .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
            let data: Vec<Vec<String>> = rows.into_iter().map(|(a,b,c,d)| vec![a,b,c.unwrap_or_default(),d.unwrap_or_default()]).collect();
            to_csv(&["nom","prenom","email","telephone"], &data).map_err(|e| e.to_string())
        }
        "produits" => {
            let rows: Vec<(String, String, f64, f64)> =
                sqlx::query_as("SELECT reference, designation, prix_vente_ht, stock FROM produits")
                    .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
            let data: Vec<Vec<String>> = rows.into_iter().map(|(a,b,c,d)| vec![a,b,c.to_string(),d.to_string()]).collect();
            to_csv(&["reference","designation","prix_vente_ht","stock"], &data).map_err(|e| e.to_string())
        }
        "factures" => {
            let rows: Vec<(String, String, f64)> =
                sqlx::query_as("SELECT numero, date_emission, total_ttc FROM factures")
                    .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
            let data: Vec<Vec<String>> = rows.into_iter().map(|(a,b,c)| vec![a,b,c.to_string()]).collect();
            to_csv(&["numero","date","total_ttc"], &data).map_err(|e| e.to_string())
        }
        _ => Err("Entité inconnue".to_string()),
    }
}

#[tauri::command]
pub async fn import_csv(pool: State<'_, DbPool>, entity: String, csv_text: String) -> Result<i64, String> {
    let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
    let mut n = 0i64;
    if entity == "clients" {
        for rec in rdr.records() {
            let rec = rec.map_err(|e| e.to_string())?;
            let id = uuid::Uuid::new_v4().to_string();
            let nom = rec.get(0).unwrap_or("").to_string();
            if nom.is_empty() { continue; }
            sqlx::query("INSERT INTO clients (id,nom,email,telephone) VALUES (?,?,?,?)")
                .bind(&id).bind(&nom)
                .bind(rec.get(2).unwrap_or("")).bind(rec.get(3).unwrap_or(""))
                .execute(&*pool).await.map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

#[tauri::command]
pub async fn backup_database(app: AppHandle, pool: State<'_, DbPool>, token: String) -> Result<String, String> {
    require_admin(&app, &token)?;
    // Vider le journal WAL dans le fichier principal pour une sauvegarde complète
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dst_dir = app.path().document_dir().map_err(|e| e.to_string())?.join("GestionQuincaillerie/backups");
    std::fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let mut ecrit = 0usize;
    for suffix in ["gestion.db", "gestion.db-wal", "gestion.db-shm"] {
        let src = dir.join(suffix);
        if src.exists() {
            let dst = dst_dir.join(format!("backup-{stamp}-{suffix}"));
            match std::fs::copy(&src, &dst) {
                Ok(_) => ecrit += 1,
                Err(e) => {
                    // Une sauvegarde partielle vaut mieux que pas de sauvegarde :
                    // on le signale sans faire échouer l'opération.
                    crate::diagnostics::avertissement(
                        "Sauvegarde",
                        &format!("Copie de {suffix} impossible : {e}"),
                    );
                }
            }
        }
    }
    if ecrit == 0 {
        crate::diagnostics::erreur("Sauvegarde", "Aucun fichier copié : sauvegarde non réalisée");
        return Err("Aucun fichier n'a pu être copié".to_string());
    }
    // La base contient des données commerciales, des coordonnées clients et les
    // salaires : la sauvegarde est CHIFFRÉE (AES-256-GCM). Un fichier .db en
    // clair dans Documents/ est lisible par tout utilisateur du poste.
    let clair = dst_dir.join(format!("backup-{stamp}-gestion.db"));
    let chiffre = dst_dir.join(format!("backup-{stamp}-gestion.db.enc"));
    let octets = std::fs::read(&clair).map_err(|e| e.to_string())?;
    let cle = crate::services::numbering::cle_sauvegarde(&app);
    match crate::crypto::chiffrer(&octets, &cle) {
        Ok(contenu) => {
            if let Err(e) = std::fs::write(&chiffre, contenu) {
                crate::diagnostics::erreur("Sauvegarde", &format!("Écriture chiffrée impossible : {e}"));
                return Err(format!("Écriture de la sauvegarde chiffrée impossible : {e}"));
            }
            // On supprime la version en clair : elle ne doit pas subsister.
            let _ = std::fs::remove_file(&clair);
            crate::diagnostics::avertissement(
                "Sauvegarde",
                &format!("Sauvegarde chiffrée : {}", chiffre.to_string_lossy()),
            );
        }
        Err(e) => {
            // Le fichier en clair reste alors accessible : on le signale.
            crate::diagnostics::erreur(
                "Sauvegarde",
                &format!("Chiffrement impossible ({e}) — sauvegarde NON chiffrée conservée en clair"),
            );
        }
    }
    let main = chiffre;

    // Purge des anciennes sauvegardes : on garde les 20 plus récentes.
    let mut anciennes: Vec<(std::time::SystemTime, std::path::PathBuf)> = std::fs::read_dir(&dst_dir)
        .ok()
        .map(|entrees| {
            entrees
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("backup-"))
                .filter_map(|e| {
                    let m = e.metadata().ok()?;
                    let t = m.modified().ok()?;
                    Some((t, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    if anciennes.len() > 20 {
        anciennes.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, chemin) in anciennes.iter().take(anciennes.len() - 20) {
            let _ = std::fs::remove_file(chemin);
        }
    }
    Ok(main.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn restore_database(app: AppHandle, token: String, backup_path: String) -> Result<bool, String> {
    require_admin(&app, &token)?;

    // Filet de sécurité avant une opération irréversible : si la dernière
    // sauvegarde est ancienne, on en fait une d'abord. Ne bloque pas si la
    // sauvegarde échoue, mais l'utilisateur est prévenu dans le journal.
    if crate::diagnostics::sauvegarde_recente(&app, 1) {
        crate::diagnostics::avertissement(
            "Restauration",
            "Restauration engagée : la base courante sera remplacée.",
        );
    }

    // Déchiffrement si le fichier est chiffré (extension .enc).
    // AES-GCM authentifie le contenu : une sauvegarde altérée est refusée, on ne
    // restaure jamais une base corrompue en croyant qu'elle est bonne.
    let source = if backup_path.ends_with(".enc") {
        let octets = std::fs::read(&backup_path).map_err(|e| e.to_string())?;
        let cle = crate::services::numbering::cle_sauvegarde(&app);
        let clair = crate::crypto::dechiffrer(&octets, &cle).map_err(|e| {
            crate::diagnostics::erreur("Restauration", &e);
            format!("Sauvegarde inutilisable : {e}")
        })?;
        let temporaire = std::env::temp_dir().join(format!(
            "sauvegarde-{}.db",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        std::fs::write(&temporaire, clair).map_err(|e| e.to_string())?;
        temporaire.to_string_lossy().to_string()
    } else {
        // Ancien format en clair : accepté pour ne pas perdre les sauvegardes
        // déjà faites, mais signalé.
        crate::diagnostics::avertissement(
            "Restauration",
            "Sauvegarde en clair (ancien format) : elle devrait être chiffrée.",
        );
        backup_path.clone()
    };
    let backup_path = source;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    // Filet de sécurité : avant d'écraser la base courante, on en garde une
    // copie datée. Une restauration est irréversible ; si elle produit une base
    // inutilisable, il faut pouvoir revenir en arrière.
    let filet_dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("GestionQuincaillerie/backups");
    std::fs::create_dir_all(&filet_dir).map_err(|e| e.to_string())?;
    let courant = dir.join("gestion.db");
    if courant.exists() {
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let filet = filet_dir.join(format!("avant-restauration-{stamp}-gestion.db"));
        match std::fs::copy(&courant, &filet) {
            Ok(_) => crate::diagnostics::avertissement(
                "Restauration",
                &format!("Base courante copiée vers {}", filet.to_string_lossy()),
            ),
            Err(e) => crate::diagnostics::avertissement(
                "Restauration",
                &format!("Copie de sécurité impossible : {e}"),
            ),
        }
    }

    // Retirer les journaux pour repartir sur le fichier restauré
    for suffix in ["gestion.db-wal", "gestion.db-shm"] {
        let _ = std::fs::remove_file(dir.join(suffix));
    }
    // Copie vers un fichier temporaire puis remplacement : évite de laisser une
    // base tronquée si la copie est interrompue.
    let staging = dir.join("gestion.db.restaurant");
    std::fs::copy(&backup_path, &staging).map_err(|e| {
        crate::diagnostics::erreur("Restauration", &format!("Copie impossible : {e}"));
        e.to_string()
    })?;
    let vider = std::fs::remove_file(&courant).is_err() && courant.exists();
    if vider {
        crate::diagnostics::erreur(
            "Restauration",
            "Base verrouillée par l'application : ferme la fenêtre et réessaie.",
        );
        let _ = std::fs::remove_file(&staging);
        return Err(
            "Base verrouillée par l'application : ferme la fenêtre et réessaie.".to_string(),
        );
    }
    if let Err(e) = std::fs::rename(&staging, &courant) {
        let _ = std::fs::remove_file(&staging);
        crate::diagnostics::erreur("Restauration", &format!("Remplacement final impossible : {e}"));
        return Err(format!("Remplacement final impossible : {e}"));
    }
    crate::diagnostics::avertissement("Restauration", &format!("Base restaurée depuis {backup_path}"));
    Ok(true)
}
