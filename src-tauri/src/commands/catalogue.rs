use crate::db::DbPool;
use crate::models::{MouvementStock, Paged, Produit};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn produits_list(pool: State<'_, DbPool>, page: Option<i64>, per_page: Option<i64>) -> Result<Paged<Produit>, String> {
    let (page, per_page) = (page.unwrap_or(1).max(1), per_page.unwrap_or(50).clamp(1,200));
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM produits").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let data: Vec<Produit> = sqlx::query_as("SELECT * FROM produits ORDER BY designation LIMIT ? OFFSET ?")
        .bind(per_page).bind((page-1)*per_page).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(Paged { data, total: total.0, page, per_page })
}

#[tauri::command]
pub async fn produits_get(pool: State<'_, DbPool>, id: String) -> Result<Produit, String> {
    sqlx::query_as("SELECT * FROM produits WHERE id=?").bind(id)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Produit introuvable".to_string())
}

#[tauri::command]
pub async fn produits_search(pool: State<'_, DbPool>, q: String) -> Result<Vec<Produit>, String> {
    let like = format!("%{}%", crate::validation::nettoyer(&q, 80));
    sqlx::query_as("SELECT * FROM produits WHERE designation LIKE ? OR reference LIKE ? LIMIT 50")
        .bind(&like).bind(&like).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

/// Catalogue de la caisse : recherche et filtre par catégorie appliqués en SQL.
///
/// Charger tout le catalogue côté front ne tient pas à l'échelle : une vraie
/// quincaillerie a plusieurs milliers de références. Le tri place les articles
/// disponibles en premier — un vendeur doit voir ce qu'il peut vendre.
#[tauri::command]
pub async fn produits_caisse(
    pool: State<'_, DbPool>,
    q: Option<String>,
    categorie: Option<String>,
    limite: Option<i64>,
) -> Result<Vec<Produit>, String> {
    let lim = limite.unwrap_or(120).clamp(1, 500);
    let recherche = crate::validation::nettoyer(&q.unwrap_or_default(), 80).to_lowercase();
    let cat = crate::validation::nettoyer(&categorie.unwrap_or_default(), 60);
    let motif = format!("%{recherche}%");

    let mut sql = String::from(
        "SELECT * FROM produits WHERE actif = 1 AND (stock > 0 OR ? = '') ",
    );
    sql.push_str(" AND ( ? = '' OR lower(designation) LIKE ? OR lower(reference) LIKE ? OR lower(COALESCE(code_barre,'')) LIKE ? ) ");
    sql.push_str(" AND ( ? = '' OR categorie = ? ) ");
    // Disponible d'abord, puis alphabétique : le vendeur voit ce qu'il vend.
    sql.push_str("ORDER BY CASE WHEN stock > 0 THEN 0 ELSE 1 END, designation LIMIT ?");

    sqlx::query_as(&sql)
        .bind(&cat)
        .bind(&cat)
        .bind(&motif).bind(&motif).bind(&motif)
        .bind(&cat).bind(&cat)
        .bind(lim)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn produits_create(pool: State<'_, DbPool>, input: Produit) -> Result<Produit, String> {
    let id = Uuid::new_v4().to_string();
    let r = sqlx::query("INSERT INTO produits (id,reference,designation,description,prix_achat_ht,prix_vente_ht,taux_tva,stock,stock_alerte,categorie,unite,code_barre,actif) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&input.reference).bind(&input.designation).bind(&input.description)
        .bind(input.prix_achat_ht).bind(input.prix_vente_ht).bind(input.taux_tva)
        .bind(input.stock).bind(input.stock_alerte).bind(&input.categorie).bind(&input.unite)
        .bind(&input.code_barre).bind(input.actif)
        .execute(&*pool).await;
    if r.is_err() { return Err("Référence déjà utilisée".to_string()); }
    // mouvement initial
    if input.stock != 0.0 {
        let mid = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
            .bind(&mid).bind(&id).bind("entree").bind(input.stock).bind(0.0).bind(input.stock).bind("Stock initial")
            .execute(&*pool).await.map_err(|e| e.to_string())?;
    }
    produits_get(pool, id).await
}

#[tauri::command]
pub async fn produits_update(pool: State<'_, DbPool>, id: String, input: Produit) -> Result<Produit, String> {
    sqlx::query("UPDATE produits SET reference=?,designation=?,description=?,prix_achat_ht=?,prix_vente_ht=?,taux_tva=?,stock_alerte=?,categorie=?,unite=?,code_barre=?,actif=?,updated_at=CURRENT_TIMESTAMP WHERE id=?")
        .bind(&input.reference).bind(&input.designation).bind(&input.description)
        .bind(input.prix_achat_ht).bind(input.prix_vente_ht).bind(input.taux_tva)
        .bind(input.stock_alerte).bind(&input.categorie).bind(&input.unite).bind(&input.code_barre)
        .bind(input.actif).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    produits_get(pool, id).await
}

#[tauri::command]
pub async fn produits_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    let used: (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM lignes_document WHERE produit_id=?) + (SELECT COUNT(*) FROM lignes_livraison WHERE produit_id=?) + (SELECT COUNT(*) FROM lignes_commande WHERE produit_id=?) + (SELECT COUNT(*) FROM mouvements_stock WHERE produit_id=?)"
    ).bind(&id).bind(&id).bind(&id).bind(&id)
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if used.0 > 0 {
        // Désactivation plutôt que suppression pour préserver l'historique
        sqlx::query("UPDATE produits SET actif=0, updated_at=CURRENT_TIMESTAMP WHERE id=?").bind(&id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        return Ok(true);
    }
    sqlx::query("DELETE FROM produits WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn produits_stock_mouvements(pool: State<'_, DbPool>, produit_id: String) -> Result<Vec<MouvementStock>, String> {
    sqlx::query_as("SELECT m.*, p.designation FROM mouvements_stock m LEFT JOIN produits p ON p.id=m.produit_id WHERE m.produit_id=? ORDER BY m.created_at DESC LIMIT 100")
        .bind(produit_id).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn produits_ajuster_stock(
    pool: State<'_, DbPool>,
    produit_id: String,
    quantite: f64,
    motif: String,
) -> Result<Produit, String> {
    if quantite == 0.0 {
        return Err("Quantité d'ajustement nulle : rien à corriger".to_string());
    }
    if !quantite.is_finite() || quantite.abs() > crate::validation::QUANTITE_MAX {
        return Err("Quantité d'ajustement invalide".to_string());
    }
    let motif = crate::validation::nettoyer(&motif, 120);
    if motif.is_empty() {
        return Err("Motif obligatoire : un ajustement de stock doit être justifié".to_string());
    }

    // Transaction : le mouvement et la mise à jour du stock sont indissociables.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let p: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
        .bind(&produit_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let (avant,) = p.ok_or_else(|| format!("Article introuvable : {produit_id}"))?;
    let apres = avant + quantite;

    // Garde-fou : un ajustement ne doit jamais rendre le stock négatif, sinon
    // l'inventaire physique et le stock théorique deviennent incompatibles.
    if apres < -1e-9 {
        return Err(format!(
            "Ajustement refusé : le stock passerait en négatif ({avant} → {})",
            apres
        ));
    }

    let mid = Uuid::new_v4().to_string();
    let typ = if quantite > 0.0 { "entree" } else { "sortie" };
    sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
        .bind(&mid).bind(&produit_id).bind(typ).bind(quantite).bind(avant).bind(apres).bind(&motif)
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    sqlx::query("UPDATE produits SET stock=?,updated_at=CURRENT_TIMESTAMP WHERE id=?")
        .bind(apres).bind(&produit_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    produits_get(pool, produit_id).await
}

// ---------- Stock global ----------
#[tauri::command]
pub async fn stock_list(pool: State<'_, DbPool>) -> Result<Vec<Produit>, String> {
    sqlx::query_as("SELECT * FROM produits ORDER BY stock ASC").fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stock_mouvements(pool: State<'_, DbPool>, limit: Option<i64>) -> Result<Vec<MouvementStock>, String> {
    let l = limit.unwrap_or(100).clamp(1, 500);
    sqlx::query_as("SELECT m.*, p.designation FROM mouvements_stock m LEFT JOIN produits p ON p.id=m.produit_id ORDER BY m.created_at DESC LIMIT ?")
        .bind(l).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
/// Inventaire physique : aligne le stock théorique sur le compté.
/// Tout ou rien : si une ligne échoue, aucun stock n'est modifié.
pub async fn stock_inventaire(pool: State<'_, DbPool>, items: Vec<(String, f64)>) -> Result<bool, String> {
    if items.is_empty() {
        return Err("Inventaire vide".to_string());
    }
    // Pré-validation de toutes les lignes avant d'écrire quoi que ce soit :
    // évite de laisser un inventaire à moitié appliqué après une erreur.
    for (pid, compte) in &items {
        crate::validation::valider_quantite(*compte, "Quantité comptée")
            .map_err(|e| format!("Article {pid} : {e}"))?;
        let row: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
            .bind(pid)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| e.to_string())?;
        if row.is_none() {
            return Err(format!("Article introuvable : {pid}"));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let mut ecarts = 0usize;
    for (pid, compte) in &items {
        let nouveau = crate::validation::valider_quantite(*compte, "Quantité comptée")?;
        let p: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        let Some((avant,)) = p else { continue };
        let diff = nouveau - avant;
        if diff.abs() > 1e-9 {
            ecarts += 1;
            let mid = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
                .bind(&mid).bind(pid).bind("inventaire").bind(diff).bind(avant).bind(nouveau).bind("Inventaire physique")
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("UPDATE produits SET stock=?,updated_at=CURRENT_TIMESTAMP WHERE id=?")
                .bind(nouveau).bind(pid)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    eprintln!("[STOCK] Inventaire appliqué : {ecarts} écart(s) corrigé(s) sur {} ligne(s)", items.len());
    Ok(true)
}

#[tauri::command]
pub async fn stock_alerte_seuil(pool: State<'_, DbPool>) -> Result<Vec<Produit>, String> {
    sqlx::query_as("SELECT * FROM produits WHERE stock <= stock_alerte ORDER BY stock ASC")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}
