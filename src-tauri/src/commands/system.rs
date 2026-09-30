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
    let dst_dir = app.path().document_dir().map_err(|e| e.to_string())?.join("GestionCommerciale/backups");
    std::fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    for suffix in ["gestion.db", "gestion.db-wal", "gestion.db-shm"] {
        let src = dir.join(suffix);
        if src.exists() {
            let dst = dst_dir.join(format!("backup-{stamp}-{suffix}"));
            std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        }
    }
    // Empreinte d'intégrité SHA256 du fichier principal
    let main = dst_dir.join(format!("backup-{stamp}-gestion.db"));
    let bytes = std::fs::read(&main).map_err(|e| e.to_string())?;
    let digest = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
    std::fs::write(dst_dir.join(format!("backup-{stamp}-gestion.db.sha256")), &digest)
        .map_err(|e| e.to_string())?;
    Ok(main.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn restore_database(app: AppHandle, token: String, backup_path: String) -> Result<bool, String> {
    require_admin(&app, &token)?;
    // Vérifie l'empreinte d'intégrité si le fichier .sha256 accompagne la sauvegarde
    let sidecar = format!("{backup_path}.sha256");
    if std::path::Path::new(&sidecar).exists() {
        let expected = std::fs::read_to_string(&sidecar).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(&backup_path).map_err(|e| e.to_string())?;
        let actual = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
        if actual.trim() != expected.trim() {
            return Err("Sauvegarde corrompue : empreinte SHA256 invalide. Restauration annulée.".to_string());
        }
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // Retirer les journaux pour repartir sur le fichier restauré
    for suffix in ["gestion.db-wal", "gestion.db-shm"] {
        let _ = std::fs::remove_file(dir.join(suffix));
    }
    let dst = dir.join("gestion.db");
    std::fs::copy(&backup_path, &dst).map_err(|e| e.to_string())?;
    Ok(true)
}
