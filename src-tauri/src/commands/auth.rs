use crate::auth::{create_token, hash_password, verify_password, verify_token};
use crate::authz::{jwt_secret, LoginGuard};
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{User, UserPublic};
use tauri::{AppHandle, State};
use uuid::Uuid;

#[tauri::command]
pub async fn auth_register(
    pool: State<'_, DbPool>,
    username: String,
    password: String,
    full_name: String,
    email: String,
) -> Result<UserPublic, String> {
    inner_register(&pool, username, password, full_name, email)
        .await
        .map_err(|e| e.to_string())
}

async fn inner_register(
    pool: &DbPool,
    username: String,
    password: String,
    full_name: String,
    email: String,
) -> AppResult<UserPublic> {
    let username = username.trim().to_lowercase();
    if username.is_empty() {
        return Err(AppError::Validation("Nom d'utilisateur vide".into()));
    }
    if password.len() < 8 {
        return Err(AppError::Validation("Mot de passe trop court (8 caractères minimum)".into()));
    }
    // Même politique que le changement de mot de passe : un compte admin
    // créé avec un mot de passe trivial reste une faille.
    crate::commands::admin::valider_mot_de_passe(&password)
        .map_err(AppError::Validation)?;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    let role = if count.0 == 0 { "admin" } else { "user" };
    let hash = hash_password(&password)?;
    let id = Uuid::new_v4().to_string();
    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, email, role) VALUES (?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&hash)
    .bind(&full_name)
    .bind(&email)
    .bind(role)
    .execute(pool)
    .await;
    if res.is_err() {
        return Err(AppError::Conflict("Nom d'utilisateur déjà utilisé".into()));
    }
    Ok(UserPublic { id, username, full_name, email, role: role.into() })
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub user: UserPublic,
    pub token: String,
    /// Jeton de rafraîchissement, à durée de vie longue mais révocable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<String>,
}

/// Nombre de comptes existants. Commande publique : permet à l'écran de
/// connexion de proposer la création du compte administrateur au lieu
/// d'afficher un formulaire de connexion qui ne pourra jamais aboutir.
/// Ne révèle rien d'autre que « l'installation est-elle déjà faite ? ».
#[tauri::command]
pub async fn auth_nb_utilisateurs(pool: State<'_, DbPool>) -> Result<i64, String> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(n)
}

#[tauri::command]
pub async fn auth_login(
    pool: State<'_, DbPool>,
    app: AppHandle,
    guard: State<'_, LoginGuard>,
    username: String,
    password: String,
) -> Result<LoginResponse, String> {
    inner_login(&pool, &app, &guard, username, password)
        .await
        .map_err(|e| e.to_string())
}

async fn inner_login(
    pool: &DbPool,
    app: &AppHandle,
    guard: &LoginGuard,
    username: String,
    password: String,
) -> AppResult<LoginResponse> {
    let username = username.trim().to_lowercase();
    guard.check(&username).map_err(AppError::Auth)?;
    let user: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(pool)
            .await?;
    let user = match user {
        Some(u) => u,
        None => {
            guard.failure(&username);
            return Err(AppError::UserNotFound);
        }
    };
    let ok = verify_password(&user.password_hash, &password)?;
    if !ok {
        guard.failure(&username);
        return Err(AppError::InvalidPassword);
    }
    guard.success(&username);
    let secret = jwt_secret(app);
    let token = create_token(&user.id, &user.username, &user.role, &secret)?;

    // Jeton de rafraîchissement : opaque, stocké en base, révocable.
    // Un JWT de 7 jours ne peut pas être révoqué ; celui-ci peut l'être.
    let refresh = crate::auth::jeton_aleatoire();
    let expire = chrono::Utc::now() + chrono::Duration::seconds(crate::auth::DUREE_RAFRAICHISSEMENT_SECONDES);
    sqlx::query(
        "INSERT INTO sessions_refresh (token, user_id, expire_at, cree_at) VALUES (?,?,?,?)",
    )
    .bind(&refresh)
    .bind(&user.id)
    .bind(expire.format("%Y-%m-%d %H:%M:%S").to_string())
    .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .execute(pool)
    .await
    .map_err(|e| crate::error::AppError::Database(sqlx::Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Connexion réussie mais session non persistée : {e}"),
    ))))?;
    // Ménage : on ne garde que les sessions encore valides.
    let _ = sqlx::query("DELETE FROM sessions_refresh WHERE expire_at < ?")
        .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
        .execute(pool)
        .await;

    let token_acc = token;
    Ok(LoginResponse {
        user: user_public(&user),
        token: token_acc,
        refresh: Some(refresh),
    })
}

fn user_public(u: &User) -> UserPublic {
    UserPublic {
        id: u.id.clone(),
        username: u.username.clone(),
        full_name: u.full_name.clone(),
        email: u.email.clone(),
        role: u.role.clone(),
    }
}

/// Déconnexion : révoque réellement le jeton de rafraîchissement.
///
/// Sans cela, « se déconnecter » ne servait à rien — le jeton restait valide
/// 7 jours dans le navigateur.
#[tauri::command]
pub async fn auth_logout(
    pool: State<'_, DbPool>,
    refresh: Option<String>,
) -> Result<bool, String> {
    if let Some(r) = refresh.filter(|x| !x.trim().is_empty()) {
        sqlx::query("DELETE FROM sessions_refresh WHERE token = ?")
            .bind(&r)
            .execute(&*pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(true)
}

/// Échange un jeton de rafraîchissement contre un nouveau jeton d'accès.
///
/// L'ancien est immédiatement invalidé : un jeton de rafraîchissement volé
/// ne peut servir qu'une fois.
#[tauri::command]
pub async fn auth_refresh_token(
    pool: State<'_, DbPool>,
    app: AppHandle,
    refresh: String,
) -> Result<LoginResponse, String> {
    if refresh.trim().is_empty() {
        return Err("Jeton de rafraîchissement manquant".to_string());
    }
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM sessions_refresh WHERE token = ? AND expire_at > ?",
    )
    .bind(&refresh)
    .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    let (user_id,) = row.ok_or("Session expirée : reconnecte-toi.")?;

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    let user = user.ok_or("Compte introuvable")?;

    // Rotation : on retire l'ancien avant d'en créer un nouveau.
    sqlx::query("DELETE FROM sessions_refresh WHERE token = ?")
        .bind(&refresh)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let nouveau = crate::auth::jeton_aleatoire();
    let expire = chrono::Utc::now()
        + chrono::Duration::seconds(crate::auth::DUREE_RAFRAICHISSEMENT_SECONDES);
    sqlx::query(
        "INSERT INTO sessions_refresh (token, user_id, expire_at, cree_at) VALUES (?,?,?,?)",
    )
    .bind(&nouveau)
    .bind(&user.id)
    .bind(expire.format("%Y-%m-%d %H:%M:%S").to_string())
    .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let secret = jwt_secret(&app);
    let token = create_token(&user.id, &user.username, &user.role, &secret)?;
    Ok(LoginResponse {
        user: user_public(&user),
        token,
        refresh: Some(nouveau),
    })
}

/// Révoque toutes les sessions d'un compte (changement de mot de passe,
/// compte compromis). Les jetons d'accès déjà émis restent valides jusqu'à
/// leur expiration (1 heure au plus).
#[tauri::command]
pub async fn auth_revoquer_sessions(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    user_id: String,
) -> Result<bool, String> {
    let claims = verify_token(&token, &jwt_secret(&app)).map_err(|e| e.to_string())?;
    if claims.role != "admin" && claims.sub != user_id {
        return Err("Action réservée aux administrateurs".to_string());
    }
    let n = sqlx::query("DELETE FROM sessions_refresh WHERE user_id = ?")
        .bind(&user_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    crate::diagnostics::securite(
        "Sessions",
        &format!("{} session(s) révoquée(s)", n.rows_affected()),
    );
    Ok(true)
}

#[tauri::command]
pub async fn auth_get_current_user(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
) -> Result<UserPublic, String> {
    let secret = jwt_secret(&app);
    let claims = verify_token(&token, &secret).map_err(|e| e.to_string())?;
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    user.map(|u| u.into()).ok_or_else(|| "Utilisateur introuvable".to_string())
}


