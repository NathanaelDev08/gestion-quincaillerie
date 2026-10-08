use crate::db::DbPool;

#[allow(dead_code)]
pub async fn generate_numero(pool: &DbPool, prefix: &str) -> Result<String, sqlx::Error> {
    crate::db::next_numero(pool, prefix).await
}

/// Clé de chiffrement des sauvegardes (32 octets), persistée dans settings.json.
///
/// Elle protège contre la fuite d'une sauvegarde (clé USB, cloud, collègue),
/// pas contre un attaquant ayant déjà accès au profil Windows : pour ce dernier
/// cas il faudrait un mot de passe saisi par l'utilisateur.
pub fn cle_sauvegarde(app: &tauri::AppHandle) -> [u8; 32] {
    use tauri_plugin_store::StoreExt;
    let mut cle = [0u8; 32];
    if let Ok(store) = app.store("settings.json") {
        let _ = store.reload();
        if let Some(v) = store
            .get("cle_sauvegarde")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            let bytes = hex_decode(&v);
            if bytes.len() == 32 {
                cle.copy_from_slice(&bytes);
                return cle;
            }
        }
        // Première utilisation : clé aléatoire tirée une fois puis conservée.
        let neuf = hex_decode(&format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).replace('-', ""));
        if neuf.len() == 32 {
            cle.copy_from_slice(&neuf);
            store.set("cle_sauvegarde", serde_json::Value::String(hex_encode(&cle)));
            let _ = store.save();
            return cle;
        }
    }
    // Repli : dérivation de la clé JWT, toujours disponible et aléatoire.
    let seed = crate::authz::jwt_secret(app);
    let mut v = sha2::Sha256::new();
    use sha2::Digest;
    v.update(seed.as_bytes());
    let out = v.finalize();
    cle.copy_from_slice(&out);
    cle
}

pub fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn hex_decode(s: &str) -> Vec<u8> {
    let nettoye: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..nettoye.len() / 2)
        .filter_map(|i| u8::from_str_radix(&nettoye[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

/// Préfixe de numérotation configurable (Paramètres → Préférences), avec repli.
pub fn doc_prefix(app: &tauri::AppHandle, key: &str, fallback: &str) -> String {
    use tauri_plugin_store::StoreExt;
    app.store("settings.json")
        .ok()
        .and_then(|s| {
            let _ = s.reload();
            s.get("preferences")
        })
        .and_then(|v| {
            v.get("prefixes")
                .and_then(|p| p.get(key))
                .and_then(|x| x.as_str())
                .map(|s| s.trim().to_uppercase())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}
