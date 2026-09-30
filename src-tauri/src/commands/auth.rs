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
    Ok(LoginResponse { user: user.into(), token })
}

#[tauri::command]
pub async fn auth_logout() -> Result<bool, String> {
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

#[tauri::command]
pub async fn auth_refresh_token(app: AppHandle, token: String) -> Result<String, String> {
    let secret = jwt_secret(&app);
    let claims = verify_token(&token, &secret).map_err(|e| e.to_string())?;
    create_token(&claims.sub, &claims.username, &claims.role, &secret)
        .map_err(|e| e.to_string())
}
