use crate::db::DbPool;
use crate::models::{CaMensuel, DashboardStats, Facture, TopItem};
use tauri::State;

#[tauri::command]
pub async fn dashboard_stats(pool: State<'_, DbPool>) -> Result<DashboardStats, String> {
    let ca: (Option<f64>,) = sqlx::query_as("SELECT SUM(total_ttc) FROM factures WHERE statut != 'annulee'")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let mois: (Option<f64>,) = sqlx::query_as("SELECT SUM(total_ttc) FROM factures WHERE date_emission >= date('now','start of month') AND statut != 'annulee'")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let imp: (Option<f64>,) = sqlx::query_as("SELECT SUM(total_ttc - montant_paye) FROM factures WHERE statut IN ('emise','partiellement_payee')")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let nb_c: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let nb_p: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM produits WHERE actif=1").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let nb_d: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devis WHERE statut IN ('brouillon','envoye')").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let stock_v: (Option<f64>,) = sqlx::query_as("SELECT SUM(stock * prix_achat_ht) FROM produits").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let alertes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM produits WHERE stock <= stock_alerte").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    Ok(DashboardStats {
        ca_total: ca.0.unwrap_or(0.0), ca_mois: mois.0.unwrap_or(0.0),
        factures_impayees: imp.0.unwrap_or(0.0), nb_clients: nb_c.0, nb_produits: nb_p.0,
        nb_devis_en_cours: nb_d.0, stock_valeur: stock_v.0.unwrap_or(0.0), alertes_stock: alertes.0,
    })
}

#[tauri::command]
pub async fn dashboard_ca_mensuel(pool: State<'_, DbPool>) -> Result<Vec<CaMensuel>, String> {
    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', date_emission) as m, SUM(total_ttc) FROM factures WHERE date_emission >= date('now','-12 months') AND statut != 'annulee' GROUP BY m ORDER BY m"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(mois, ca)| CaMensuel { mois, ca: ca.unwrap_or(0.0) }).collect())
}

#[tauri::command]
pub async fn dashboard_top_produits(pool: State<'_, DbPool>) -> Result<Vec<TopItem>, String> {
    let rows: Vec<(String, Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT designation, SUM(total_ht), SUM(quantite) FROM lignes_document WHERE document_type='facture' GROUP BY designation ORDER BY SUM(total_ht) DESC LIMIT 10"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(nom, total, qte)| TopItem { nom, total: total.unwrap_or(0.0), quantite: qte }).collect())
}

#[tauri::command]
pub async fn dashboard_top_clients(pool: State<'_, DbPool>) -> Result<Vec<TopItem>, String> {
    let rows: Vec<(Option<String>, Option<f64>)> = sqlx::query_as(
        "SELECT c.nom, SUM(f.total_ttc) FROM factures f LEFT JOIN clients c ON c.id=f.client_id WHERE f.statut != 'annulee' GROUP BY c.id ORDER BY SUM(f.total_ttc) DESC LIMIT 10"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(nom, total)| TopItem { nom: nom.unwrap_or_else(|| "-".into()), total: total.unwrap_or(0.0), quantite: None }).collect())
}

#[tauri::command]
pub async fn dashboard_factures_en_retard(pool: State<'_, DbPool>) -> Result<Vec<Facture>, String> {
    sqlx::query_as("SELECT f.*, c.nom as client_nom, c.telephone as telephone FROM factures f LEFT JOIN clients c ON c.id=f.client_id WHERE f.date_echeance < date('now') AND f.statut IN ('emise','partiellement_payee') ORDER BY f.date_echeance LIMIT 20")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_marges(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    let tot: (Option<f64>,) = sqlx::query_as(
        "SELECT SUM(l.total_ht - l.quantite * COALESCE(p.prix_achat_ht, 0.0)) FROM lignes_document l LEFT JOIN produits p ON p.id = l.produit_id WHERE l.document_type = 'facture'"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let mois: (Option<f64>,) = sqlx::query_as(
        "SELECT SUM(l.total_ht - l.quantite * COALESCE(p.prix_achat_ht, 0.0)) FROM lignes_document l LEFT JOIN produits p ON p.id = l.produit_id JOIN factures f ON f.id = l.document_id WHERE l.document_type = 'facture' AND f.date_emission >= date('now','start of month') AND f.statut != 'annulee'"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let ca_mois: (Option<f64>,) = sqlx::query_as(
        "SELECT SUM(total_ht) FROM factures WHERE date_emission >= date('now','start of month') AND statut != 'annulee'"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let (m_tot, m_mois, ca_m) = (tot.0.unwrap_or(0.0), mois.0.unwrap_or(0.0), ca_mois.0.unwrap_or(0.0));
    let taux = if ca_m > 0.0 { m_mois / ca_m * 100.0 } else { 0.0 };
    Ok(serde_json::json!({ "marge_totale": m_tot, "marge_mois": m_mois, "taux_marge_mois": taux }))
}

#[tauri::command]
pub async fn dashboard_depenses_mensuelles(pool: State<'_, DbPool>) -> Result<Vec<CaMensuel>, String> {
    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', date_depense) as m, SUM(montant) FROM depenses WHERE date_depense >= date('now','-12 months') GROUP BY m ORDER BY m"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(mois, total)| CaMensuel { mois, ca: total.unwrap_or(0.0) }).collect())
}
