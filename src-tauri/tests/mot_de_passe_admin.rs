//! Réinitialisation locale du mot de passe administrateur.
//! Usage : cargo test --test mot_de_passe_admin -- --ignored --nocapture
//! Variables d'environnement : GESTION_MDP (obligatoire)

#[tokio::test]
#[ignore]
async fn reinitialise_mot_de_passe_admin() {
    let Ok(mdp) = std::env::var("GESTION_MDP") else {
        println!("GESTION_MDP non défini : rien fait");
        return;
    };
    if mdp.chars().count() < 8 {
        eprintln!("Mot de passe trop court : abandon");
        return;
    }

    let Some(appdata) = std::env::var("APPDATA").ok() else {
        eprintln!("APPDATA introuvable");
        return;
    };
    let path = std::path::PathBuf::from(appdata)
        .join("com.quincaillerie.app")
        .join("gestion.db");
    if !path.exists() {
        eprintln!("Base absente : {}", path.display());
        return;
    }
    let url = format!("sqlite:{}?mode=rw", path.to_string_lossy());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connexion");

    // L'algorithme argon2 du binaire n'est pas linkable depuis un test
    // d'intégration : on appelle la commande du binaire via un petit
    // « utilitaire » écrit en Rust dans le crate principal. Ce test
    // d'intégration ne peut donc pas hasher seul ; il vérifie et affiche
    // l'état, et l'écriture est faite par le binaire via
    // `users_changer_mot_de_passe` depuis l'application.
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .expect("lecture");
    println!("Base : {} — {} compte(s)", path.display(), n.0);

    let comptes: Vec<(String, String)> = sqlx::query_as("SELECT username, role FROM users")
        .fetch_all(&pool)
        .await
        .expect("comptes");
    for (u, r) in comptes {
        println!("  {u} ({r})");
    }

    let hash = hash_argon2(&mdp);
    let r = sqlx::query("UPDATE users SET password_hash=? WHERE username='admin'")
        .bind(&hash)
        .execute(&pool)
        .await
        .expect("mise à jour");
    println!(
        "Mot de passe de « admin » mis à jour ({} ligne(s)).",
        r.rows_affected()
    );
    let _ = pool.close().await;
}

/// Réimplémentation d'argon2id conforme à `argon2::Argon2::default()`
/// (variante Argon2id, version 19, mémoire 19 MiB, 2 itérations, parallélisme 1).
fn hash_argon2(password: &str) -> String {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}
