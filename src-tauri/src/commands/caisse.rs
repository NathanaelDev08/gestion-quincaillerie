use crate::db::{next_numero, DbPool};
use crate::models::{ClotureZ, LigneInput};
use tauri::{AppHandle, State};
use uuid::Uuid;

// ================= VENTE COMPTOIR (caisse) =================
#[derive(serde::Deserialize)]
pub struct VenteComptoirInput {
    pub client_id: Option<String>,
    pub lignes: Vec<LigneInput>,
    pub mode: String,
    pub montant_recu: f64,
}

#[derive(serde::Serialize)]
pub struct VenteComptoirResult {
    pub facture_id: String,
    pub numero: String,
    pub total_ttc: f64,
    pub montant_recu: f64,
    pub rendu: f64,
}

#[tauri::command]
pub async fn vente_comptoir(pool: State<'_, DbPool>, app: AppHandle, input: VenteComptoirInput) -> Result<VenteComptoirResult, String> {
    if input.lignes.is_empty() {
        return Err("Panier vide".to_string());
    }
    // Client comptoir par défaut
    let client_id = match input.client_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM clients WHERE nom='Client Comptoir' LIMIT 1")
                .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
            if let Some((id,)) = existing {
                id
            } else {
                let id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO clients (id,nom,prenom,pays) VALUES (?,?,?,?)")
                    .bind(&id).bind("Client Comptoir").bind("").bind("Côte d'Ivoire")
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                id
            }
        }
    };
    let mut ht = 0.0; let mut tva = 0.0;
    for l in &input.lignes {
        let lt = l.quantite * l.prix_unitaire_ht * (1.0 - l.remise / 100.0);
        ht += lt; tva += lt * l.taux_tva / 100.0;
    }
    let ttc = ht + tva;
    if input.montant_recu + 0.001 < ttc {
        return Err(format!("Montant insuffisant : reçu {} pour un total de {}", input.montant_recu, ttc));
    }
    let fid = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "facture", "FAC")).await.map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES (?,?,?,?,?,'payee',?,?,?,?,?,?)")
        .bind(&fid).bind(&numero).bind(&client_id).bind(&today).bind(&today)
        .bind(ht).bind(tva).bind(ttc).bind(ttc).bind(0.0).bind("Vente comptoir")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    for l in &input.lignes {
        let lt = l.quantite * l.prix_unitaire_ht * (1.0 - l.remise / 100.0);
        sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(&fid).bind("facture").bind(&l.produit_id).bind(&l.designation)
            .bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva).bind(l.remise).bind(lt)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        if let Some(pid) = &l.produit_id {
            sqlx::query("UPDATE produits SET stock = stock - ? WHERE id=?").bind(l.quantite).bind(pid)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
            let mid = Uuid::new_v4().to_string();
            let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
                .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
            let apres = cur.map(|c| c.0).unwrap_or(0.0);
            sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&mid).bind(pid).bind("sortie").bind(l.quantite).bind(apres + l.quantite).bind(apres).bind("Vente comptoir").bind(&numero)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
        }
    }
    sqlx::query("INSERT INTO reglements (id,facture_id,montant,mode,date_reglement,reference) VALUES (?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(&fid).bind(ttc).bind(&input.mode).bind(&today).bind(format!("Caisse {numero}"))
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // Fidélité immédiate (vente payée comptant)
    let _ = crate::commands::fidelite_crediter(&pool, &client_id, ttc).await;
    // Écritures VTE
    let jvte: Option<(String,)> = sqlx::query_as("SELECT id FROM journaux WHERE code='VTE'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c411: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='411000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c707: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='707000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let ctva: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='445710'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let (Some(j), Some(c1), Some(c2), Some(c3)) = (jvte, c411, c707, ctva) {
        for (cid, debit, credit) in [(&c1.0, ttc, 0.0), (&c2.0, 0.0, ht), (&c3.0, 0.0, tva)] {
            sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&j.0).bind(cid).bind(&today).bind(format!("Caisse {numero}")).bind(debit).bind(credit).bind(&numero).bind(&fid)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
        }
    }
    Ok(VenteComptoirResult {
        facture_id: fid,
        numero,
        total_ttc: ttc,
        montant_recu: input.montant_recu,
        rendu: input.montant_recu - ttc,
    })
}

// ================= CLOTURE Z =================
#[tauri::command]
pub async fn cloture_z(pool: State<'_, DbPool>, date: String) -> Result<ClotureZ, String> {
    let rows: Vec<(String, Option<f64>, i64)> = sqlx::query_as(
        "SELECT mode, SUM(montant), COUNT(DISTINCT facture_id) FROM reglements WHERE date_reglement = ? GROUP BY mode"
    ).bind(&date).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    let mut total = 0.0; let mut nb = 0i64;
    let mut especes = 0.0; let mut virement = 0.0; let mut cheque = 0.0; let mut cb = 0.0; let mut mobile = 0.0;
    for (mode, sum, n) in rows {
        let s = sum.unwrap_or(0.0);
        total += s; nb = nb.max(n);
        match mode.as_str() {
            "especes" => especes += s,
            "virement" => virement += s,
            "cheque" => cheque += s,
            "cb" => cb += s,
            _ => mobile += s,
        }
    }
    let nb_total: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT facture_id) FROM reglements WHERE date_reglement = ?")
        .bind(&date).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    nb = nb_total.0;
    let existing: Option<ClotureZ> = sqlx::query_as("SELECT * FROM clotures_z WHERE date = ?")
        .bind(&date).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let Some(c) = existing {
        sqlx::query("UPDATE clotures_z SET nb_ventes=?, total=?, especes=?, virement=?, cheque=?, cb=?, mobile=? WHERE id=?")
            .bind(nb).bind(total).bind(especes).bind(virement).bind(cheque).bind(cb).bind(mobile).bind(&c.id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        return sqlx::query_as("SELECT * FROM clotures_z WHERE id=?").bind(&c.id).fetch_one(&*pool).await.map_err(|e| e.to_string());
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO clotures_z (id,date,nb_ventes,total,especes,virement,cheque,cb,mobile) VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&date).bind(nb).bind(total).bind(especes).bind(virement).bind(cheque).bind(cb).bind(mobile)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM clotures_z WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clotures_list(pool: State<'_, DbPool>) -> Result<Vec<ClotureZ>, String> {
    sqlx::query_as("SELECT * FROM clotures_z ORDER BY date DESC LIMIT 60")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}
