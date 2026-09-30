use crate::db::DbPool;
use crate::models::{Client, Fournisseur, Paged};
use tauri::State;
use uuid::Uuid;

fn paged<T: serde::Serialize>(data: Vec<T>, total: i64, page: i64, per_page: i64) -> Paged<T> {
    Paged { data, total, page, per_page }
}

// ---------- CLIENTS ----------
#[tauri::command]
pub async fn clients_list(
    pool: State<'_, DbPool>,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<Paged<Client>, String> {
    let (page, per_page) = (page.unwrap_or(1).max(1), per_page.unwrap_or(50).clamp(1, 200));
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let data: Vec<Client> = sqlx::query_as(
        "SELECT * FROM clients ORDER BY nom LIMIT ? OFFSET ?",
    )
    .bind(per_page).bind((page - 1) * per_page)
    .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(paged(data, total.0, page, per_page))
}

#[tauri::command]
pub async fn clients_get(pool: State<'_, DbPool>, id: String) -> Result<Client, String> {
    sqlx::query_as("SELECT * FROM clients WHERE id = ?")
        .bind(id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Client introuvable".to_string())
}

#[tauri::command]
pub async fn clients_search(pool: State<'_, DbPool>, q: String) -> Result<Vec<Client>, String> {
    let like = format!("%{q}%");
    sqlx::query_as("SELECT * FROM clients WHERE nom LIKE ? OR entreprise LIKE ? OR email LIKE ? LIMIT 50")
        .bind(&like).bind(&like).bind(&like)
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clients_create(pool: State<'_, DbPool>, input: Client) -> Result<Client, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO clients (id,nom,prenom,entreprise,email,telephone,adresse,ville,code_postal,pays,siret,tva_intra,notes) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&input.nom).bind(&input.prenom).bind(&input.entreprise)
        .bind(&input.email).bind(&input.telephone).bind(&input.adresse).bind(&input.ville)
        .bind(&input.code_postal).bind(&input.pays).bind(&input.siret).bind(&input.tva_intra).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    clients_get(pool, id).await
}

#[tauri::command]
pub async fn clients_update(pool: State<'_, DbPool>, id: String, input: Client) -> Result<Client, String> {
    sqlx::query("UPDATE clients SET nom=?,prenom=?,entreprise=?,email=?,telephone=?,adresse=?,ville=?,code_postal=?,pays=?,siret=?,tva_intra=?,notes=?,updated_at=CURRENT_TIMESTAMP WHERE id=?")
        .bind(&input.nom).bind(&input.prenom).bind(&input.entreprise)
        .bind(&input.email).bind(&input.telephone).bind(&input.adresse).bind(&input.ville)
        .bind(&input.code_postal).bind(&input.pays).bind(&input.siret).bind(&input.tva_intra).bind(&input.notes)
        .bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    clients_get(pool, id).await
}

#[tauri::command]
pub async fn clients_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    let used: (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM devis WHERE client_id=?) + (SELECT COUNT(*) FROM factures WHERE client_id=?) + (SELECT COUNT(*) FROM avoirs WHERE client_id=?) + (SELECT COUNT(*) FROM bons_livraison WHERE client_id=?)"
    ).bind(&id).bind(&id).bind(&id).bind(&id)
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if used.0 > 0 {
        return Err(format!("Suppression impossible : ce client est lié à {} document(s) (devis, factures, avoirs ou livraisons).", used.0));
    }
    sqlx::query("DELETE FROM clients WHERE id=?").bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

// ---------- FOURNISSEURS ----------
#[tauri::command]
pub async fn fournisseurs_list(pool: State<'_, DbPool>, page: Option<i64>, per_page: Option<i64>) -> Result<Paged<Fournisseur>, String> {
    let (page, per_page) = (page.unwrap_or(1).max(1), per_page.unwrap_or(50).clamp(1, 200));
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM fournisseurs")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let data: Vec<Fournisseur> = sqlx::query_as("SELECT * FROM fournisseurs ORDER BY nom LIMIT ? OFFSET ?")
        .bind(per_page).bind((page-1)*per_page).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(paged(data, total.0, page, per_page))
}

#[tauri::command]
pub async fn fournisseurs_get(pool: State<'_, DbPool>, id: String) -> Result<Fournisseur, String> {
    sqlx::query_as("SELECT * FROM fournisseurs WHERE id=?").bind(id)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Fournisseur introuvable".to_string())
}

#[tauri::command]
pub async fn fournisseurs_search(pool: State<'_, DbPool>, q: String) -> Result<Vec<Fournisseur>, String> {
    let like = format!("%{q}%");
    sqlx::query_as("SELECT * FROM fournisseurs WHERE nom LIKE ? OR entreprise LIKE ? LIMIT 50")
        .bind(&like).bind(&like).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fournisseurs_create(pool: State<'_, DbPool>, input: Fournisseur) -> Result<Fournisseur, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO fournisseurs (id,nom,entreprise,email,telephone,adresse,ville,code_postal,pays,siret,tva_intra,notes) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&input.nom).bind(&input.entreprise).bind(&input.email).bind(&input.telephone)
        .bind(&input.adresse).bind(&input.ville).bind(&input.code_postal).bind(&input.pays)
        .bind(&input.siret).bind(&input.tva_intra).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    fournisseurs_get(pool, id).await
}

#[tauri::command]
pub async fn fournisseurs_update(pool: State<'_, DbPool>, id: String, input: Fournisseur) -> Result<Fournisseur, String> {
    sqlx::query("UPDATE fournisseurs SET nom=?,entreprise=?,email=?,telephone=?,adresse=?,ville=?,code_postal=?,pays=?,siret=?,tva_intra=?,notes=? WHERE id=?")
        .bind(&input.nom).bind(&input.entreprise).bind(&input.email).bind(&input.telephone)
        .bind(&input.adresse).bind(&input.ville).bind(&input.code_postal).bind(&input.pays)
        .bind(&input.siret).bind(&input.tva_intra).bind(&input.notes).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    fournisseurs_get(pool, id).await
}

#[tauri::command]
pub async fn fournisseurs_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    let used: (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM commandes_fournisseurs WHERE fournisseur_id=?) + (SELECT COUNT(*) FROM depenses WHERE fournisseur_id=?)"
    ).bind(&id).bind(&id)
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if used.0 > 0 {
        return Err(format!("Suppression impossible : ce fournisseur est lié à {} document(s) (commandes ou dépenses).", used.0));
    }
    sqlx::query("DELETE FROM fournisseurs WHERE id=?").bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}
