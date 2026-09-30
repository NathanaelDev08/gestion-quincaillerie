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
    let like = format!("%{q}%");
    sqlx::query_as("SELECT * FROM produits WHERE designation LIKE ? OR reference LIKE ? LIMIT 50")
        .bind(&like).bind(&like).fetch_all(&*pool).await.map_err(|e| e.to_string())
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
pub async fn produits_ajuster_stock(pool: State<'_, DbPool>, produit_id: String, quantite: f64, motif: String) -> Result<Produit, String> {
    let p: Produit = produits_get(pool.clone(), produit_id.clone()).await?;
    let apres = p.stock + quantite;
    let mid = Uuid::new_v4().to_string();
    let typ = if quantite >= 0.0 { "entree" } else { "sortie" };
    sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
        .bind(&mid).bind(&produit_id).bind(typ).bind(quantite).bind(p.stock).bind(apres).bind(&motif)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("UPDATE produits SET stock=?,updated_at=CURRENT_TIMESTAMP WHERE id=?")
        .bind(apres).bind(&produit_id).execute(&*pool).await.map_err(|e| e.to_string())?;
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
pub async fn stock_inventaire(pool: State<'_, DbPool>, items: Vec<(String, f64)>) -> Result<bool, String> {
    for (pid, nouveau) in items {
        let p: Option<Produit> = sqlx::query_as("SELECT * FROM produits WHERE id=?").bind(&pid)
            .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
        if let Some(p) = p {
            let diff = nouveau - p.stock;
            if diff != 0.0 {
                let mid = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
                    .bind(&mid).bind(&pid).bind("inventaire").bind(diff).bind(p.stock).bind(nouveau).bind("Inventaire")
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                sqlx::query("UPDATE produits SET stock=? WHERE id=?").bind(nouveau).bind(&pid)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(true)
}

#[tauri::command]
pub async fn stock_alerte_seuil(pool: State<'_, DbPool>) -> Result<Vec<Produit>, String> {
    sqlx::query_as("SELECT * FROM produits WHERE stock <= stock_alerte ORDER BY stock ASC")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}
