use crate::db::DbPool;
use serde::Serialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

// ================= JOURNAL D'AUDIT =================

#[derive(Serialize, sqlx::FromRow)]
pub struct AuditRow {
    pub id: String,
    pub utilisateur: String,
    pub role: String,
    pub action: String,
    pub entite: String,
    pub entite_id: String,
    pub detail: Option<String>,
    pub montant: Option<f64>,
    pub created_at: String,
}

#[tauri::command]
pub async fn audit_list(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    action: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AuditRow>, String> {
    crate::authz::require_admin(&app, &token)?;
    let lim = limit.unwrap_or(200).clamp(1, 1000);
    let rows: Vec<AuditRow> = match action {
        Some(a) if !a.is_empty() && a != "tout" => sqlx::query_as(
            "SELECT * FROM audit_log WHERE action = ? ORDER BY created_at DESC, rowid DESC LIMIT ?",
        )
        .bind(a)
        .bind(lim)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?,
        _ => sqlx::query_as("SELECT * FROM audit_log ORDER BY created_at DESC, rowid DESC LIMIT ?")
            .bind(lim)
            .fetch_all(&*pool)
            .await
            .map_err(|e| e.to_string())?,
    };
    Ok(rows)
}

#[tauri::command]
pub async fn audit_actions(pool: State<'_, DbPool>) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT action FROM audit_log ORDER BY action")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[tauri::command]
pub async fn audit_purge(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
) -> Result<bool, String> {
    crate::authz::require_admin(&app, &token)?;
    sqlx::query("DELETE FROM audit_log")
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

// ================= MULTI-DÉPÔTS =================

#[derive(Serialize, sqlx::FromRow)]
pub struct Depot {
    pub id: String,
    pub nom: String,
    pub adresse: Option<String>,
    pub actif: i64,
}

#[derive(Serialize, serde::Deserialize)]
pub struct DepotInput {
    pub nom: String,
    pub adresse: Option<String>,
}

#[tauri::command]
pub async fn depots_list(pool: State<'_, DbPool>) -> Result<Vec<Depot>, String> {
    sqlx::query_as("SELECT * FROM depots ORDER BY actif DESC, nom")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn depots_create(pool: State<'_, DbPool>, input: DepotInput) -> Result<Depot, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO depots (id,nom,adresse,actif) VALUES (?,?,?,1)")
        .bind(&id)
        .bind(input.nom.trim())
        .bind(input.adresse)
        .execute(&*pool)
        .await
        .map_err(|e| format!("Dépôt déjà existant ou nom invalide : {e}"))?;
    sqlx::query_as("SELECT * FROM depots WHERE id=?")
        .bind(&id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DepotStock {
    pub depot_id: String,
    depot_nom: String,
    pub produit_id: String,
    pub designation: String,
    pub reference: String,
    pub categorie: Option<String>,
    pub quantite: f64,
    pub unite: Option<String>,
}

/// Stock d'un dépôt, avec recherche texte.
#[tauri::command]
pub async fn depot_stock(
    pool: State<'_, DbPool>,
    depot_id: String,
    q: Option<String>,
) -> Result<Vec<DepotStock>, String> {
    let motif = format!("%{}%", q.unwrap_or_default().trim().to_lowercase());
    sqlx::query_as(
        "SELECT sd.depot_id, d.nom as depot_nom, sd.produit_id, p.designation, p.reference,
                p.categorie, sd.quantite, p.unite
         FROM stock_depot sd
         JOIN produits p ON p.id = sd.produit_id
         JOIN depots d ON d.id = sd.depot_id
         WHERE sd.depot_id = ?
           AND (? = '%%' OR lower(p.designation) LIKE ? OR lower(p.reference) LIKE ?)
         ORDER BY p.designation LIMIT 500",
    )
    .bind(&depot_id)
    .bind(&motif)
    .bind(&motif)
    .bind(&motif)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

/// Valeur totale du stock par dépôt.
#[tauri::command]
pub async fn depots_valeur(pool: State<'_, DbPool>) -> Result<Vec<(String, String, f64, i64)>, String> {
    sqlx::query_as(
        "SELECT d.id, d.nom, COALESCE(SUM(sd.quantite * p.prix_achat_ht), 0) AS valeur, COUNT(sd.produit_id) AS nb
         FROM depots d
         LEFT JOIN stock_depot sd ON sd.depot_id = d.id
         LEFT JOIN produits p ON p.id = sd.produit_id
         GROUP BY d.id, d.nom
         ORDER BY d.nom",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct TransfertInput {
    pub produit_id: String,
    pub depot_origine: String,
    pub depot_destination: String,
    pub quantite: f64,
}

/// Transfert inter-dépôts : décrémente l'origine, incrémente la destination,
/// journalise le mouvement et le stock global.
#[tauri::command]
pub async fn depot_transferer(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    input: TransfertInput,
) -> Result<bool, String> {
    if input.quantite <= 0.0 {
        return Err("Quantité invalide".to_string());
    }
    if input.depot_origine == input.depot_destination {
        return Err("Dépôt origine et destination identiques".to_string());
    }
    let avant: Option<(f64,)> = sqlx::query_as(
        "SELECT quantite FROM stock_depot WHERE depot_id=? AND produit_id=?",
    )
    .bind(&input.depot_origine)
    .bind(&input.produit_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    let dispo = avant.map(|x| x.0).unwrap_or(0.0);
    if dispo + 1e-9 < input.quantite {
        return Err(format!(
            "Stock insuffisant dans le dépôt d'origine ({dispo} disponible)"
        ));
    }
    let apres = dispo - input.quantite;
    sqlx::query("UPDATE stock_depot SET quantite=? WHERE depot_id=? AND produit_id=?")
        .bind(apres)
        .bind(&input.depot_origine)
        .bind(&input.produit_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO stock_depot (depot_id,produit_id,quantite) VALUES (?,?,?)
         ON CONFLICT(depot_id,produit_id) DO UPDATE SET quantite=quantite+excluded.quantite",
    )
    .bind(&input.depot_destination)
    .bind(&input.produit_id)
    .bind(input.quantite)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO mouvements_depot (id,depot_id,produit_id,type,quantite,stock_avant,stock_apres,motif)
         VALUES (?,?,?,'transfert_sortant',?,?,?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&input.depot_origine)
    .bind(&input.produit_id)
    .bind(input.quantite)
    .bind(dispo)
    .bind(apres)
    .bind(format!("Vers {}", input.depot_destination))
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let dest_avant: Option<(f64,)> = sqlx::query_as(
        "SELECT quantite FROM stock_depot WHERE depot_id=? AND produit_id=?",
    )
    .bind(&input.depot_destination)
    .bind(&input.produit_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    let dv = dest_avant.map(|x| x.0).unwrap_or(0.0);
    sqlx::query(
        "INSERT INTO mouvements_depot (id,depot_id,produit_id,type,quantite,stock_avant,stock_apres,motif)
         VALUES (?,?,?,'transfert_entrant',?,?,?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&input.depot_destination)
    .bind(&input.produit_id)
    .bind(input.quantite)
    .bind(dv)
    .bind(dv + input.quantite)
    .bind(format!("Depuis {}", input.depot_origine))
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let pool2 = pool.inner().clone();
    let tok = token.clone();
    let pid = input.produit_id.clone();
    let q = input.quantite;
    tauri::async_runtime::spawn(async move {
        crate::db::audit(
            &pool2, &tok, &app, "transfert_depot", "produit", &pid,
            &format!("{} unité(s) vers {}", q, input.depot_destination), None,
        )
        .await;
    });
    Ok(true)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DepotMouvement {
    pub designation: Option<String>,
    pub produit_id: String,
    #[sqlx(rename = "type")]
    pub type_mvt: String,
    pub quantite: f64,
    pub stock_avant: f64,
    pub stock_apres: f64,
    pub motif: Option<String>,
    pub created_at: String,
}

/// Mouvements récents d'un dépôt.
#[tauri::command]
pub async fn depot_mouvements(
    pool: State<'_, DbPool>,
    depot_id: String,
) -> Result<Vec<DepotMouvement>, String> {
    sqlx::query_as(
        "SELECT p.designation, m.produit_id, m.type, m.quantite, m.stock_avant, m.stock_apres, m.motif, m.created_at
         FROM mouvements_depot m
         LEFT JOIN produits p ON p.id = m.produit_id
         WHERE m.depot_id = ? ORDER BY m.created_at DESC, m.rowid DESC LIMIT 100",
    )
    .bind(&depot_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

