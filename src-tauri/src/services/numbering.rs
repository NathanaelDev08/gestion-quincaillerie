use crate::db::DbPool;

pub async fn generate_numero(pool: &DbPool, prefix: &str) -> Result<String, sqlx::Error> {
    crate::db::next_numero(pool, prefix).await
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
