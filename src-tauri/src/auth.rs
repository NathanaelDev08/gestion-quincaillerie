use crate::error::{AppError, AppResult};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub exp: usize,
}

/// Génère un mot de passe initial fort et lisible par un humain.
/// 16 caractères issus d'un alphabet sans ambiguïté (pas de 0/O, 1/l/I),
/// garanti au moins une majuscule, une minuscule et un chiffre.
pub fn mot_de_passe_aleatoire() -> String {
    use rand_core::RngCore;
    const MAJ: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    const MIN: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
    const NUM: &[u8] = b"23456789";
    const TOUS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";

    let mut rng = rand_core::OsRng;
    let mut mdp: Vec<u8> = Vec::with_capacity(16);
    let mut idx = |taille: usize| (rng.next_u32() as usize) % taille;
    mdp.push(MAJ[idx(MAJ.len())]);
    mdp.push(MIN[idx(MIN.len())]);
    mdp.push(NUM[idx(NUM.len())]);
    for _ in 0..13 {
        mdp.push(TOUS[idx(TOUS.len())]);
    }
    // Mélange : évite que les caractèresNoise soient toujours en tête.
    for i in (1..mdp.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        mdp.swap(i, j);
    }
    String::from_utf8(mdp).unwrap_or_else(|_| "Quincaillerie2026".to_string())
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Auth(e.to_string()))
}

pub fn verify_password(hash: &str, password: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| AppError::Auth(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Durée de vie d'un jeton d'accès : court. Un jeton de 7 jours volé donne
/// 7 jours d'accès ; 1 heure limite la fenêtre d'exposition.
pub const DUREE_JETON_SECONDES: i64 = 3600;

/// Durée de vie du jeton de rafraîchissement : 7 jours.
pub const DUREE_RAFRAICHISSEMENT_SECONDES: i64 = 7 * 24 * 3600;

/// Jeton opaque à usage unique, stocké en base pour pouvoir être révoqué.
pub fn jeton_aleatoire() -> String {
    use rand_core::RngCore;
    let mut b = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn create_token(user_id: &str, username: &str, role: &str, secret: &str) -> AppResult<String> {
    create_token_duree(user_id, username, role, secret, DUREE_JETON_SECONDES)
}

pub fn create_token_duree(
    user_id: &str,
    username: &str,
    role: &str,
    secret: &str,
    duree: i64,
) -> AppResult<String> {
    let exp = chrono::Utc::now().timestamp() as usize + duree as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Auth(e.to_string()))
}

pub fn verify_token(token: &str, secret: &str) -> AppResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::InvalidToken)
}
