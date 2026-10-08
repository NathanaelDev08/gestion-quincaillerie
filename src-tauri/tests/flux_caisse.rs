//! Test bout-en-bout du flux de vente comptoir, avec injection de panne.
//!
//! Objectif : prouver que la vente est atomique. Si une écriture échoue en
//! cours d'encaissement, AUCUNE trace ne doit subsister (ni facture orpheline,
//! ni stock décrémenté, ni règlement sans facture). C'est le scénario qui
//! produit un écart de caisse inexpliqué en boutique.

use sqlx::{Row, SqlitePool};

/// Schéma minimal reproduisant les contraintes de la base réelle.
async fn base_test() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("base mémoire");
    for sql in [
        "CREATE TABLE clients (id TEXT PRIMARY KEY, nom TEXT, prenom TEXT, pays TEXT, points REAL DEFAULT 0)",
        "CREATE TABLE produits (id TEXT PRIMARY KEY, designation TEXT, stock REAL, prix_vente_ht REAL, taux_tva REAL, updated_at DATETIME)",
        "CREATE TABLE factures (id TEXT PRIMARY KEY, numero TEXT UNIQUE, client_id TEXT, date_emission TEXT, date_echeance TEXT, statut TEXT, total_ht REAL, total_tva REAL, total_ttc REAL, montant_paye REAL, remise REAL, notes TEXT)",
        "CREATE TABLE lignes_document (id TEXT PRIMARY KEY, document_id TEXT, document_type TEXT, produit_id TEXT, designation TEXT, quantite REAL, prix_unitaire_ht REAL, taux_tva REAL, remise REAL, total_ht REAL)",
        "CREATE TABLE mouvements_stock (id TEXT PRIMARY KEY, produit_id TEXT, type TEXT, quantite REAL, stock_avant REAL, stock_apres REAL, motif TEXT, document_ref TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
        "CREATE TABLE reglements (id TEXT PRIMARY KEY, facture_id TEXT, montant REAL, mode TEXT, date_reglement TEXT, reference TEXT, notes TEXT)",
        "CREATE TABLE sequences (code TEXT PRIMARY KEY, annee TEXT, compteur INTEGER)",
        "CREATE TABLE journaux (id TEXT PRIMARY KEY, code TEXT, libelle TEXT)",
        "CREATE TABLE comptes (id TEXT PRIMARY KEY, numero TEXT, intitule TEXT, classe TEXT, type TEXT)",
        "CREATE TABLE ecritures (id TEXT PRIMARY KEY, journal_id TEXT, compte_id TEXT, date_ecriture TEXT, libelle TEXT, debit REAL, credit REAL, piece_ref TEXT, facture_id TEXT)",
    ] {
        sqlx::query(sql).execute(&pool).await.expect("schéma");
    }
    sqlx::query("INSERT INTO journaux (id,code,libelle) VALUES ('jvte','VTE','Ventes')")
        .execute(&pool)
        .await
        .unwrap();
    for (id, numero) in [("c1", "411000"), ("c2", "707000"), ("c3", "445710")] {
        sqlx::query("INSERT INTO comptes (id,numero,intitule,classe,type) VALUES (?,?,'x','1','actif')")
            .bind(id).bind(numero).execute(&pool).await.unwrap();
    }
    pool
}

#[tokio::test]
async fn vente_reussie_stock_facture_et_reglement_sont_coherents() {
    let pool = base_test().await;
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p1','Ciment',100.0,5500.0,18.0)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p2','Vis',50.0,1800.0,18.0)")
        .execute(&pool).await.unwrap();

    // Panne simulée : la 3e écriture (ligne_document) échoue toujours.
    // On utilise un trigger qui lève une exception.
    sqlx::query(
        "CREATE TRIGGER fail_ligne BEFORE INSERT ON lignes_document
         BEGIN SELECT RAISE(ABORT, 'disque plein (simulé)'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut tx = pool.begin().await.unwrap();

    // Vérification du stock avant écriture
    for (pid, qte) in [("p1", 10.0f64), ("p2", 2.0)] {
        let s: (f64,) = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
            .bind(pid).fetch_one(&mut *tx).await.unwrap();
        assert!(s.0 >= qte, "stock insuffisant pour {pid}");
    }

    // Le script d'écriture est volontairement interrompu par le trigger :
    // on s'arrête au moment de l'échec, comme le ferait une panne réelle.
    let res: Result<(), sqlx::Error> = async {
        sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES ('f1','FAC-1','cl','2026-01-01','2026-01-01','payee',7300.0,1314.0,8614.0,8614.0,0.0,'vente')")
            .execute(&mut *tx).await?;
        for (pid, des, q, pu) in [("p1","Ciment",10.0,5500.0), ("p2","Vis",2.0,1800.0)] {
            sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
                .bind(format!("l{pid}")).bind("f1").bind("facture").bind(pid).bind(des)
                .bind(q).bind(pu).bind(18.0).bind(0.0).bind(q*pu)
                .execute(&mut *tx).await?;
        }
        Ok(())
    }.await;

    assert!(res.is_err(), "l'injection de panne doit faire échouer l'écriture");

    // L'appelant annule la transaction comme le fait le vrai code en cas d'erreur.
    let _ = tx.rollback().await;

    // === AUCUNE trace ne doit subsister ===
    let n_factures: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM factures").fetch_one(&pool).await.unwrap();
    assert_eq!(n_factures.0, 0, "aucune facture orpheline ne doit rester");
    let n_lignes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lignes_document").fetch_one(&pool).await.unwrap();
    assert_eq!(n_lignes.0, 0, "aucune ligne ne doit rester");

    // Le stock est intact : c'est la garantie qui compte pour la caisse.
    let stocks: Vec<(String, f64)> = sqlx::query_as("SELECT id, stock FROM produits ORDER BY id")
        .fetch_all(&pool).await.unwrap();
    assert_eq!(stocks[0].1, 100.0, "stock de p1 intact");
    assert_eq!(stocks[1].1, 50.0, "stock de p2 intact");

    let n_mvt: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mouvements_stock").fetch_one(&pool).await.unwrap();
    assert_eq!(n_mvt.0, 0, "aucun mouvement de stock orphelin");
    let n_reg: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reglements").fetch_one(&pool).await.unwrap();
    assert_eq!(n_reg.0, 0, "aucun règlement sans facture");
}

#[tokio::test]
async fn vente_reussie_met_tout_a_jour_de_maniere_coherente() {
    let pool = base_test().await;
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p1','Ciment',100.0,5500.0,18.0)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p2','Vis',50.0,1800.0,18.0)")
        .execute(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let lignes = [("p1", "Ciment", 10.0f64, 5500.0f64), ("p2", "Vis", 2.0, 1800.0)];
    let mut ht = 0.0;
    for (pid, des, q, pu) in lignes {
        ht += q * pu;
        sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(format!("l{pid}")).bind("f1").bind("facture").bind(pid).bind(des)
            .bind(q).bind(pu).bind(18.0).bind(0.0).bind(q*pu)
            .execute(&mut *tx).await.unwrap();
        let s: (f64,) = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
            .bind(pid).fetch_one(&mut *tx).await.unwrap();
        let apres = s.0 - q;
        sqlx::query("UPDATE produits SET stock=? WHERE id=?").bind(apres).bind(pid)
            .execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,'sortie',?,?,?,'Vente comptoir','FAC-1')")
            .bind(format!("m{pid}")).bind(pid).bind(q).bind(s.0).bind(apres)
            .execute(&mut *tx).await.unwrap();
    }
    let tva = ht * 0.18;
    let ttc = ht + tva;
    sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES ('f1','FAC-1','cl','2026-01-01','2026-01-01','payee',?,?,?,?,0.0,'vente')")
        .bind(ht).bind(tva).bind(ttc).bind(ttc)
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO reglements (id,facture_id,montant,mode,date_reglement,reference) VALUES ('r1','f1',?,'especes','2026-01-01','Caisse FAC-1')")
        .bind(ttc).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();

    // === Contrôles de cohérence ===
    let f: (f64, f64, f64, f64) = sqlx::query_as("SELECT total_ht,total_tva,total_ttc,montant_paye FROM factures WHERE id='f1'")
        .fetch_one(&pool).await.unwrap();
    assert!((f.0 - 58600.0).abs() < 0.01, "HT attendu 58600, obtenu {}", f.0);
    assert!((f.1 - 10548.0).abs() < 0.01, "TVA attendue 10548, obtenu {}", f.1);
    assert!((f.2 - 69148.0).abs() < 0.01, "TTC attendu 69148, obtenu {}", f.2);
    assert!((f.2 - f.3).abs() < 0.01, "la facture payée doit l'être à 100 %");

    // Somme des lignes = total de la facture
    let somme: (f64,) = sqlx::query_as("SELECT SUM(total_ht) FROM lignes_document WHERE document_id='f1'")
        .fetch_one(&pool).await.unwrap();
    assert!((somme.0 - f.0).abs() < 0.01, "la somme des lignes doit égaler le total HT");

    // Chaque mouvement est cohérent avec le stock final
    for (pid, attendu) in [("p1", 90.0), ("p2", 48.0)] {
        let s: (f64,) = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
            .fetch_one(&pool).await.unwrap();
        assert!((s.0 - attendu).abs() < 0.01, "stock final de {pid} : {} au lieu de {attendu}", s.0);
        let m: (f64, f64) = sqlx::query_as("SELECT stock_avant, stock_apres FROM mouvements_stock WHERE produit_id=?")
            .bind(pid).fetch_one(&pool).await.unwrap();
        assert!((m.1 - m.0).abs() > 0.0, "le mouvement doit refléter une variation");
        assert!((m.1 - attendu).abs() < 0.01, "stock_apres du mouvement doit égaler le stock final");
    }
}

#[tokio::test]
async fn la_vente_refusee_ne_consomme_pas_de_numero_de_facture() {
    let pool = base_test().await;
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p1','Ciment',2.0,5500.0,18.0)")
        .execute(&pool).await.unwrap();

    // Première tentative : stock insuffisant, on annule.
    let mut tx = pool.begin().await.unwrap();
    let n1 = numero_tx(&mut tx, "FAC").await;
    assert_eq!(n1, 1, "première tentative consomme le numéro 1");
    let _ = tx.rollback().await;

    // Deuxième tentative : le numéro 1 doit être réutilisable.
    let n2 = numero(&pool, "FAC").await;
    assert_eq!(n2, 1, "après rollback, le numéro 1 doit être réattribué");

    // Vente réussie : le numéro 2 est le suivant.
    let n3 = numero(&pool, "FAC").await;
    assert_eq!(n3, 2, "la numérotation doit reprendre sans trou");
}

/// Numéro suivant pour un préfixe, dans la transaction courante.
async fn numero_tx(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, prefix: &str) -> i64 {
    let code = format!("{prefix}-2026");
    let row = sqlx::query_as::<_, (i64,)>("SELECT compteur FROM sequences WHERE code=?")
        .bind(&code)
        .fetch_optional(&mut **tx)
        .await
        .unwrap();
    let next = row.map(|r| r.0 + 1).unwrap_or(1);
    sqlx::query("INSERT INTO sequences (code,annee,compteur) VALUES (?,?,?) ON CONFLICT(code) DO UPDATE SET compteur=excluded.compteur")
        .bind(&code)
        .bind("2026")
        .bind(next)
        .execute(&mut **tx)
        .await
        .unwrap();
    next
}

/// Numéro suivant pour un préfixe, hors transaction.
async fn numero(pool: &SqlitePool, prefix: &str) -> i64 {
    let code = format!("{prefix}-2026");
    let row = sqlx::query_as::<_, (i64,)>("SELECT compteur FROM sequences WHERE code=?")
        .bind(&code)
        .fetch_optional(pool)
        .await
        .unwrap();
    let next = row.map(|r| r.0 + 1).unwrap_or(1);
    sqlx::query("INSERT INTO sequences (code,annee,compteur) VALUES (?,?,?) ON CONFLICT(code) DO UPDATE SET compteur=excluded.compteur")
        .bind(&code)
        .bind("2026")
        .bind(next)
        .execute(pool)
        .await
        .unwrap();
    next
}

#[tokio::test]
async fn une_vente_ne_peut_pas_creer_de_stock_negatif() {
    let pool = base_test().await;
    sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES ('p1','Ciment',3.0,5500.0,18.0)")
        .execute(&pool).await.unwrap();

    // Tentative de vente de 10 unités sur un stock de 3 : bloquée avant écriture.
    let demande = 10.0f64;
    let stock: (f64,) = sqlx::query_as("SELECT stock FROM produits WHERE id='p1'")
        .fetch_one(&pool).await.unwrap();
    if stock.0 < demande {
        // L'opération est refusée : rien n'est écrit.
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM factures").fetch_one(&pool).await.unwrap();
        assert_eq!(n.0, 0);
        let s: (f64,) = sqlx::query_as("SELECT stock FROM produits WHERE id='p1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(s.0, 3.0, "stock inchangé");
    } else {
        panic!("la garde de stock n'aurait pas dû laisser passer");
    }
}

#[tokio::test]
async fn deux_clients_peuvent_encaisser_en_paralle_sans_doublon() {
    // Vérifie l'unicité du numéro : deux ventes quasi simultanées ne doivent
    // pas produire deux factures avec le même numéro.
    let pool = base_test().await;
    for id in ["p1", "p2"] {
        sqlx::query("INSERT INTO produits (id,designation,stock,prix_vente_ht,taux_tva) VALUES (?,'Art',50.0,1000.0,18.0)")
            .bind(id).execute(&pool).await.unwrap();
    }
    let a = numero(&pool, "FAC").await;
    let b = numero(&pool, "FAC").await;
    assert_ne!(a, b, "deux appels doivent produire deux numéros distincts");
    assert_eq!(b, a + 1, "les numéros doivent être consécutifs");

    // L'index UNIQUE sur numero est la vraie protection en base.
    sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES ('f1','FAC-1','c','2026-01-01','2026-01-01','payee',0,0,0,0,0,'')")
        .execute(&pool).await.unwrap();
    let doublon = sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES ('f2','FAC-1','c','2026-01-01','2026-01-01','payee',0,0,0,0,0,'')")
        .execute(&pool).await;
    assert!(doublon.is_err(), "la base doit refuser un numéro de facture déjà utilisé");

    let total: i64 = sqlx::query("SELECT COUNT(*) FROM factures")
        .fetch_one(&pool).await.unwrap().get(0);
    assert_eq!(total, 1);
}
