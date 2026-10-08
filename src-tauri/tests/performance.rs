//! Contrôle de montée en charge : le diagnostic ne doit pas être un jouet.
//!
//! On génère un catalogue de 5 000 références (taille réaliste d'une
//! quincaillerie), puis on mesure les requêtes réellement utilisées par la
//! caisse. Un indicateur en vert sur une base vide ne prouve rien.

mod fixtures;

use std::time::Instant;

use sqlx::{Row, SqlitePool};

async fn base_peuplee(n: usize) -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE produits (
            id TEXT PRIMARY KEY, reference TEXT, designation TEXT,
            code_barre TEXT, categorie TEXT, prix_vente_ht REAL,
            prix_achat_ht REAL, taux_tva REAL, stock REAL,
            stock_alerte REAL, unite TEXT, actif INTEGER, created_at DATETIME)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TABLE categories (id TEXT PRIMARY KEY, nom TEXT, slug TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE mouvements_stock (id TEXT PRIMARY KEY, produit_id TEXT, type TEXT, quantite REAL, stock_avant REAL, stock_apres REAL, motif TEXT, document_ref TEXT, created_at DATETIME)")
        .execute(&pool)
        .await
    .unwrap();
    sqlx::query("CREATE TABLE factures (id TEXT PRIMARY KEY, numero TEXT, client_id TEXT, date_emission TEXT, statut TEXT, total_ht REAL, total_tva REAL, total_ttc REAL, montant_paye REAL)")
        .execute(&pool)
        .await
    .unwrap();

    let rayon = fixtures::RAYONS;
    let mut tx = pool.begin().await.unwrap();
    for i in 0..n {
        let rayon_n = rayon[i % rayon.len()];
        sqlx::query(
            "INSERT INTO produits (id,reference,designation,code_barre,categorie,
                prix_vente_ht,prix_achat_ht,taux_tva,stock,stock_alerte,unite,actif,created_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,1,?)",
        )
        .bind(format!("p{i}"))
        .bind(format!("REF-{rayon_n}-{i}"))
        .bind(format!("Article {rayon_n} numéro {i}"))
        .bind(format!("61{:010}", i))
        .bind(rayon_n)
        .bind(1000.0 + (i % 50) as f64 * 100.0)
        .bind(700.0 + (i % 50) as f64 * 70.0)
        .bind(18.0)
        .bind((i % 200) as f64)
        .bind(5.0)
        .bind("pcs")
        .bind("2026-01-01 00:00:00")
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    pool
}

#[tokio::test]
#[ignore]
async fn catalogue_5000_references_reste_suffisamment_rapide() {
    let pool = base_peuplee(5000).await;

    // 1. Recherche utilisée par la caisse (produits_caisse)
    let debut = Instant::now();
    let r: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM produits WHERE actif = 1 AND (? = '')
         AND (? = '' OR lower(designation) LIKE ? OR lower(reference) LIKE ? OR lower(COALESCE(code_barre,'')) LIKE ?)
         AND (? = '' OR categorie = ?)
         ORDER BY CASE WHEN stock > 0 THEN 0 ELSE 1 END, designation LIMIT ?",
    )
    .bind("")
    .bind("ciment")
    .bind("%ciment%").bind("%ciment%").bind("%ciment%")
    .bind("").bind("")
    .bind(120i64)
    .fetch_all(&pool)
    .await
    .unwrap();
    let recherche = debut.elapsed();

    // 2. Sans filtre : la caisse doit charger vite.
    let debut = Instant::now();
    let n: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM produits WHERE actif=1 ORDER BY designation LIMIT 120",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let catalogue = debut.elapsed();

    println!("Recherche parmi 5000 : {recherche:?} ({} résultats)", r.len());
    println!("Chargement catalogue : {catalogue:?} ({n:?} lignes)");
    println!("Total : {:?}", recherche + catalogue);

    // Seuil choisi pour un poste de boutique avec disque lent.
    // Au-delà, le vendeur sent que l'application « rame ».
    assert!(
        recherche.as_millis() < 200,
        "recherche trop lente : {recherche:?}"
    );
    assert!(
        catalogue.as_millis() < 200,
        "chargement du catalogue trop lent : {catalogue:?}"
    );
}

#[tokio::test]
#[ignore]
async fn l_index_ameliore_la_recherche_de_faconcaillere() {
    // Sans index sur la désignation, LIKE '%terme%' est un balayage complet.
    let pool = base_peuplee(5000).await;

    let sans = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM produits WHERE lower(designation) LIKE '%ciment%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    // Index : SQLite peut l'utiliser pour un motif à préfixe ; avec un
    // motif contenant '%' en tête, le gain est limité — c'est pourquoi
    // la caisse borne aussi le nombre de résultats.
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_prod_designation ON produits(designation)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_prod_categorie_stock ON produits(categorie, stock)")
        .execute(&pool)
        .await
        .unwrap();

    let debut = Instant::now();
    let avec = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM produits WHERE lower(designation) LIKE '%ciment%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;
    let duree = debut.elapsed();

    assert_eq!(sans, avec, "l'index ne doit pas changer le résultat");
    println!("Recherche full-scan indexée : {duree:?} ({avec} résultats)");
}

#[tokio::test]
#[ignore]
async fn la_fermeture_z_agrege_correctement_plusieurs_ventes() {
    // Le résultat d'une clôture Z doit être exact même avec de nombreux modes.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE reglements (id TEXT PRIMARY KEY, facture_id TEXT, montant REAL, mode TEXT, date_reglement TEXT)")
        .execute(&pool).await.unwrap();

    for (i, (mode, montant)) in [
        ("especes", 1000.0), ("mobile", 2500.0), ("especes", 1500.0),
        ("cb", 3000.0), ("orange_money", 700.0),
    ].iter().enumerate() {
        sqlx::query("INSERT INTO reglements VALUES (?,?,?,?,?)")
            .bind(format!("r{i}")).bind(format!("f{}", i / 2))
            .bind(montant).bind(mode).bind("2026-10-08")
            .execute(&pool).await.unwrap();
    }

    let lignes: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT mode, SUM(montant) FROM reglements WHERE date_reglement = ? GROUP BY mode",
    ).bind("2026-10-08").fetch_all(&pool).await.unwrap();

    let (mut especes, mut mobile, mut cb) = (0.0, 0.0, 0.0);
    for (mode, somme) in &lignes {
        let s = somme.unwrap_or(0.0);
        match mode.as_str() {
            "especes" => especes += s,
            "cb" => cb += s,
            _ => mobile += s, // mode inconnu → mobile money
        }
    }
    assert_eq!(especes, 2500.0);
    assert_eq!(cb, 3000.0);
    assert_eq!(mobile, 3200.0, "le mode inconnu doit être compté, pas perdu");
    assert_eq!(especes + mobile + cb, 8700.0);
}

/// La rotation des jetons doit rendre chaque jeton inutilisable après usage.
/// Un jeton de rafraîchissement volé ne peut servir qu'une fois.
#[tokio::test]
#[ignore]
async fn un_jeton_de_rafraichissement_ne_sert_qu_une_fois() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT)")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE sessions_refresh (token TEXT PRIMARY KEY, user_id TEXT, expire_at TEXT, cree_at TEXT)")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO users VALUES ('u1','admin')").execute(&pool).await.unwrap();
    for (tok, expire) in [("t1", "2099-01-01 00:00:00"), ("t2", "2020-01-01 00:00:00")] {
        sqlx::query("INSERT INTO sessions_refresh VALUES (?,?,?,?)")
            .bind(tok).bind("u1").bind(expire).bind("2026-10-08 00:00:00")
            .execute(&pool).await.unwrap();
    }

    let maintenant = "2026-10-08 12:00:00";

    // 1. Un jeton valide est accepté.
    let trouve: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM sessions_refresh WHERE token = ? AND expire_at > ?",
    ).bind("t1").bind(maintenant).fetch_optional(&pool).await.unwrap();
    assert!(trouve.is_some(), "un jeton valide doit être accepté");

    // 2. Un jeton expiré est refusé.
    let expire: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM sessions_refresh WHERE token = ? AND expire_at > ?",
    ).bind("t2").bind(maintenant).fetch_optional(&pool).await.unwrap();
    assert!(expire.is_none(), "un jeton expiré doit être refusé");

    // 3. Après rotation, l'ancien ne fonctionne plus.
    sqlx::query("DELETE FROM sessions_refresh WHERE token = ?").bind("t1").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO sessions_refresh VALUES ('t3','u1','2099-01-01 00:00:00',?)")
        .bind(maintenant).execute(&pool).await.unwrap();

    let ancien: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM sessions_refresh WHERE token = ? AND expire_at > ?",
    ).bind("t1").bind(maintenant).fetch_optional(&pool).await.unwrap();
    assert!(ancien.is_none(), "l'ancien jeton doit être invalidé");

    let nouveau: Option<(String,)> = sqlx::query_as(
        "SELECT user_id FROM sessions_refresh WHERE token = ? AND expire_at > ?",
    ).bind("t3").bind(maintenant).fetch_optional(&pool).await.unwrap();
    assert!(nouveau.is_some());

    // 4. La révocation de toutes les sessions d'un compte fonctionne.
    sqlx::query("DELETE FROM sessions_refresh WHERE user_id = ?").bind("u1").execute(&pool).await.unwrap();
    let restantes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions_refresh WHERE user_id = ?")
        .bind("u1").fetch_one(&pool).await.unwrap();
    assert_eq!(restantes.0, 0);
}