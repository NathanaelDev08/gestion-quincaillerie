use crate::auth::verify_token;
use crate::config::AppConfig;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Secret JWT persistant : généré aléatoirement à la première utilisation,
/// stocké dans settings.json (plus de secret codé en dur).
pub fn jwt_secret(app: &AppHandle) -> String {
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store("settings.json") {
        let _ = store.reload();
        if let Some(v) = store.get("jwt_secret").and_then(|v| v.as_str().map(|s| s.to_string())) {
            if v.len() >= 32 {
                return v;
            }
        }
        let fresh = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).replace('-', "");
        store.set("jwt_secret", serde_json::Value::String(fresh.clone()));
        let _ = store.save();
        return fresh;
    }
    AppConfig::default().jwt_secret
}

/// Vérifie le token et exige le rôle admin. Retourne l'id utilisateur.
pub fn require_admin(app: &AppHandle, token: &str) -> Result<String, String> {
    let secret = jwt_secret(app);
    let claims = verify_token(token, &secret).map_err(|e| e.to_string())?;
    if claims.role != "admin" {
        return Err("Action réservée aux administrateurs".to_string());
    }
    Ok(claims.sub)
}

/// Garde anti-bruteforce : 5 échecs => verrouillage 15 minutes par identifiant.
pub struct LoginGuard {
    fails: Mutex<HashMap<String, (u32, Instant)>>,
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self { fails: Mutex::new(HashMap::new()) }
    }
}

const MAX_FAILS: u32 = 5;
const LOCK_TIME: Duration = Duration::from_secs(15 * 60);

impl LoginGuard {
    pub fn check(&self, login: &str) -> Result<(), String> {
        let key = login.trim().to_lowercase();
        let map = self.fails.lock().map_err(|_| "Erreur interne".to_string())?;
        if let Some((n, t)) = map.get(&key) {
            if *n >= MAX_FAILS && t.elapsed() < LOCK_TIME {
                let reste = (LOCK_TIME - t.elapsed()).as_secs() / 60 + 1;
                return Err(format!("Compte verrouillé après {MAX_FAILS} échecs. Réessayez dans ~{reste} min."));
            }
        }
        Ok(())
    }

    pub fn success(&self, login: &str) {
        if let Ok(mut map) = self.fails.lock() {
            map.remove(&login.trim().to_lowercase());
        }
    }

    pub fn failure(&self, login: &str) {
        if let Ok(mut map) = self.fails.lock() {
            let e = map.entry(login.trim().to_lowercase()).or_insert((0, Instant::now()));
            e.0 += 1;
            e.1 = Instant::now();
        }
    }
}
