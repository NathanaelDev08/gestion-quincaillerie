use crate::authz::require_admin;
use crate::db::DbPool;
use crate::models::{Facture, UserPublic};
use tauri::{AppHandle, State};

// ---------- Utilisateurs ----------
#[tauri::command]
pub async fn users_list(pool: State<'_, DbPool>, app: AppHandle, token: String) -> Result<Vec<UserPublic>, String> {
    require_admin(&app, &token)?;
    let rows: Vec<(String, String, String, String, String)> =
        sqlx::query_as("SELECT id, username, full_name, email, role FROM users ORDER BY username")
            .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id, username, full_name, email, role)| {
        UserPublic { id, username, full_name, email, role }
    }).collect())
}

#[tauri::command]
pub async fn users_changer_role(pool: State<'_, DbPool>, app: AppHandle, token: String, id: String, role: String) -> Result<bool, String> {
    require_admin(&app, &token)?;
    if !["admin", "user", "comptable", "commercial"].contains(&role.as_str()) {
        return Err("Rôle inconnu".to_string());
    }
    sqlx::query("UPDATE users SET role=? WHERE id=?").bind(&role).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn users_delete(pool: State<'_, DbPool>, app: AppHandle, token: String, id: String) -> Result<bool, String> {
    let actor = require_admin(&app, &token)?;
    if actor == id {
        return Err("Vous ne pouvez pas supprimer votre propre compte".to_string());
    }
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if n <= 1 {
        return Err("Impossible de supprimer le dernier utilisateur".to_string());
    }
    sqlx::query("DELETE FROM users WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

// ---------- Relances ----------
#[tauri::command]
pub async fn factures_relancer(pool: State<'_, DbPool>, id: String) -> Result<Facture, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("UPDATE factures SET derniere_relance=? WHERE id=?").bind(&today).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // SQLite < 3.35 n'a pas DROP COLUMN ; la colonne peut manquer sur les vieilles bases -> déjà gérée par migration v2
    let f: Option<Facture> = sqlx::query_as("SELECT f.*, c.nom as client_nom FROM factures f LEFT JOIN clients c ON c.id=f.client_id WHERE f.id=?")
        .bind(&id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    f.ok_or_else(|| "Facture introuvable".to_string())
}

// ---------- Divers ----------
#[tauri::command]
pub async fn produits_categories(pool: State<'_, DbPool>) -> Result<Vec<String>, String> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as("SELECT DISTINCT categorie FROM produits WHERE categorie IS NOT NULL AND categorie != '' ORDER BY categorie")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().filter_map(|r| r.0).collect())
}
