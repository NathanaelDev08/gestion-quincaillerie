//! Inspection ponctuelle du schéma de la base installée (diagnostic).
//! Lancer : cargo test inspect_schema -- --ignored --nocapture

#[tokio::test]
#[ignore]
async fn inspect_schema() {
    let Some(path) = std::env::var("APPDATA").ok().map(|a| {
        std::path::PathBuf::from(a)
            .join("com.quincaillerie.app")
            .join("gestion.db")
    }) else {
        println!("APPDATA introuvable");
        return;
    };
    println!("Base : {:?}", path);
    if !path.exists() {
        println!("Base absente");
        return;
    }
    let url = format!("sqlite:{}?mode=rw", path.to_string_lossy());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connexion");

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("liste des tables");

    println!("\n=== TABLES ({} ) ===", tables.len());
    for (t,) in &tables {
        let cols: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT name, type, \"notnull\" FROM pragma_table_info(?) ORDER BY cid",
        )
        .bind(t)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
        let liste: Vec<String> =
            cols.iter().map(|(n, ty, _)| format!("{n}:{ty}")).collect();
        println!("  {t} → {}", liste.join(", "));
    }

    if tables.iter().any(|(t,)| t == "audit_log") {
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .unwrap_or((-1,));
        println!("\naudit_log : {} ligne(s)", n.0);
    }
}
