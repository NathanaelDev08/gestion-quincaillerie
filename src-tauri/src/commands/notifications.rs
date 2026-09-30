use crate::db::DbPool;
use tauri::State;

#[derive(serde::Serialize)]
pub struct Notification {
    pub categorie: String,
    pub titre: String,
    pub detail: String,
    pub lien: String,
    pub niveau: String,
}

/// Centre de notifications calculé en temps réel :
/// impayés, stock bas, devis qui expirent, commandes en retard.
#[tauri::command]
pub async fn notifications_list(pool: State<'_, DbPool>) -> Result<Vec<Notification>, String> {
    let mut out = vec![];

    let imp: (i64, Option<f64>) = sqlx::query_as(
        "SELECT COUNT(*), SUM(total_ttc - montant_paye) FROM factures WHERE date_echeance < date('now') AND statut IN ('emise','partiellement_payee')"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if imp.0 > 0 {
        out.push(Notification {
            categorie: "Impayés".into(),
            titre: format!("{} facture(s) en retard", imp.0),
            detail: format!("Reste dû total : {:.0} F CFA", imp.1.unwrap_or(0.0)),
            lien: "/relances".into(),
            niveau: "urgent".into(),
        });
    }

    let alertes: Vec<(String, f64)> = sqlx::query_as(
        "SELECT designation, stock FROM produits WHERE stock <= stock_alerte AND actif=1 ORDER BY stock ASC LIMIT 5"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    if !alertes.is_empty() {
        let (nb,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM produits WHERE stock <= stock_alerte AND actif=1")
            .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
        out.push(Notification {
            categorie: "Stock".into(),
            titre: format!("{} article(s) sous le seuil", nb),
            detail: alertes.iter().map(|(n, s)| format!("{n} ({s})")).collect::<Vec<_>>().join(", "),
            lien: "/stock".into(),
            niveau: "alerte".into(),
        });
    }

    let devis: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM devis WHERE statut='envoye' AND date_validite BETWEEN date('now') AND date('now','+7 days')"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if devis.0 > 0 {
        out.push(Notification {
            categorie: "Devis".into(),
            titre: format!("{} devis expirent sous 7 jours", devis.0),
            detail: "Relancez vos prospects avant expiration.".into(),
            lien: "/devis".into(),
            niveau: "info".into(),
        });
    }

    let cmd: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM commandes_fournisseurs WHERE date_livraison_prevue IS NOT NULL AND date_livraison_prevue < date('now') AND statut IN ('validee','partielle')"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if cmd.0 > 0 {
        out.push(Notification {
            categorie: "Achats".into(),
            titre: format!("{} commande(s) en retard de livraison", cmd.0),
            detail: "Contactez vos fournisseurs.".into(),
            lien: "/achats".into(),
            niveau: "alerte".into(),
        });
    }

    Ok(out)
}

#[tauri::command]
pub async fn tout_relancer(pool: State<'_, DbPool>) -> Result<i64, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let r = sqlx::query(
        "UPDATE factures SET derniere_relance=? WHERE date_echeance < date('now') AND statut IN ('emise','partiellement_payee')"
    ).bind(&today).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(r.rows_affected() as i64)
}
