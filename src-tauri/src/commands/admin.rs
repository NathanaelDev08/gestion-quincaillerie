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

// ---------- Sécurité du mot de passe ----------

/// Politesse de mot de passe : longueur minimale et variété de caractères.
/// Volontairement simple à retenir : un mot de passe complexe que personne
/// n'arrive à taper finit noté sur un post-it collé à l'écran.
pub fn valider_mot_de_passe(mdp: &str) -> Result<(), String> {
    if mdp.chars().count() < 8 {
        return Err("Le mot de passe doit contenir au moins 8 caractères".to_string());
    }
    if mdp.chars().count() > 128 {
        return Err("Mot de passe trop long (128 caractères maximum)".to_string());
    }
    if mdp == mdp.to_lowercase() {
        return Err("Ajoute au moins une majuscule".to_string());
    }
    if mdp.chars().all(|c| !c.is_ascii_digit()) {
        return Err("Ajoute au moins un chiffre".to_string());
    }
    let courants = ["password", "12345678", "admin123", "qwertyui", "azerty12"];
    let minuscules = mdp.to_lowercase();
    if courants.iter().any(|c| minuscules.contains(c)) {
        return Err("Ce mot de passe est trop courant, choisis-en un autre".to_string());
    }
    Ok(())
}

/// Change le mot de passe de l'utilisateur connecté.
/// L'ancien est vérifié : sans lui, quelqu'un qui ouvre l'application sur un
/// poste laissé ouvert pourrait prendre le compte.
#[tauri::command]
pub async fn users_changer_mot_de_passe(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    ancien: String,
    nouveau: String,
) -> Result<bool, String> {
    let actor = require_admin(&app, &token)?;
    if ancien.is_empty() {
        return Err("Ancien mot de passe obligatoire".to_string());
    }
    if ancien == nouveau {
        return Err("Le nouveau mot de passe doit être différent de l'ancien".to_string());
    }
    valider_mot_de_passe(&nouveau)?;

    let hash: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE id=?")
        .bind(&actor)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    let (hash,) = hash.ok_or_else(|| "Utilisateur introuvable".to_string())?;
    if !crate::auth::verify_password(&hash, &ancien).unwrap_or(false) {
        crate::diagnostics::securite("Mot de passe", "Ancien mot de passe incorrect");
        return Err("Ancien mot de passe incorrect".to_string());
    }

    let nouveau_hash = crate::auth::hash_password(&nouveau)
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
        .bind(&nouveau_hash)
        .bind(&actor)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    crate::diagnostics::securite("Mot de passe", "Mot de passe administrateur modifié");
    Ok(true)
}

/// Réinitialise le mot de passe d'un compte et renvoie un secret à connu.
#[tauri::command]
pub async fn users_reinitialiser_mot_de_passe(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    id: String,
) -> Result<String, String> {
    require_admin(&app, &token)?;
    let existe: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id=?")
        .bind(&id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    if existe.0 == 0 {
        return Err("Utilisateur introuvable".to_string());
    }
    let mdp = crate::auth::mot_de_passe_aleatoire();
    let hash = crate::auth::hash_password(&mdp).map_err(|e| e.to_string())?;
    sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
        .bind(&hash)
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    crate::diagnostics::securite(
        "Mot de passe",
        &format!("Mot de passe réinitialisé par un administrateur : {mdp}"),
    );
    Ok(mdp)
}

/// Indique si le compte utilise encore le mot de passe créé au premier lancement.
/// Indique si le compte utilise encore le mot de passe créé au premier
/// lancement. Conservé comme garde-fou pour les installations antérieures.
#[tauri::command]
#[allow(dead_code)]
pub async fn users_mot_de_passe_par_defaut(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    id: String,
) -> Result<bool, String> {
    require_admin(&app, &token)?;
    let hash: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE id=?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    match hash {
        Some((h,)) => Ok(crate::auth::verify_password(&h, "admin123").unwrap_or(false)),
        None => Ok(false),
    }
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
