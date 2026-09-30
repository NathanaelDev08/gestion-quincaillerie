use crate::db::{next_numero, DbPool};
use crate::models::{FactureFournisseur, Promo, ReglementFournisseur};
use tauri::{AppHandle, State};
use uuid::Uuid;

fn admin(app: &AppHandle, token: &str) -> Result<String, String> {
    crate::authz::require_admin(app, token)
}

// ================= FACTURES FOURNISSEURS (dettes) =================
#[tauri::command]
pub async fn ff_list(pool: State<'_, DbPool>) -> Result<Vec<FactureFournisseur>, String> {
    sqlx::query_as("SELECT f.*, fo.nom as fournisseur_nom FROM factures_fournisseurs f LEFT JOIN fournisseurs fo ON fo.id=f.fournisseur_id ORDER BY f.created_at DESC LIMIT 200")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct FfInput {
    pub fournisseur_id: String,
    pub date_emission: String,
    pub date_echeance: String,
    pub total_ht: f64,
    pub taux_tva: f64,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn ff_create(pool: State<'_, DbPool>, app: AppHandle, token: String, input: FfInput) -> Result<FactureFournisseur, String> {
    admin(&app, &token)?;
    let tva = input.total_ht * input.taux_tva / 100.0;
    let ttc = input.total_ht + tva;
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "facture_achat", "FF")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO factures_fournisseurs (id,numero,fournisseur_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,notes) VALUES (?,?,?,?,?,'emise',?,?,?,?,?)")
        .bind(&id).bind(&numero).bind(&input.fournisseur_id).bind(&input.date_emission).bind(&input.date_echeance)
        .bind(input.total_ht).bind(tva).bind(ttc).bind(0.0).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // Écritures ACH : 401 crédit TTC / 607 débit HT / 445660 débit TVA
    let jach: Option<(String,)> = sqlx::query_as("SELECT id FROM journaux WHERE code='ACH'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c401: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='401000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c607: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='607000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let cded: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='445660'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let (Some(j), Some(c1), Some(c2), Some(c3)) = (jach, c401, c607, cded) {
        for (cid, debit, credit) in [(&c1.0, 0.0, ttc), (&c2.0, input.total_ht, 0.0), (&c3.0, tva, 0.0)] {
            sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref) VALUES (?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&j.0).bind(cid).bind(&input.date_emission).bind(format!("Facture achat {numero}")).bind(debit).bind(credit).bind(&numero)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
        }
    }
    sqlx::query_as("SELECT f.*, fo.nom as fournisseur_nom FROM factures_fournisseurs f LEFT JOIN fournisseurs fo ON fo.id=f.fournisseur_id WHERE f.id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ff_delete(pool: State<'_, DbPool>, app: AppHandle, token: String, id: String) -> Result<bool, String> {
    admin(&app, &token)?;
    sqlx::query("DELETE FROM reglements_fournisseurs WHERE facture_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM factures_fournisseurs WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[derive(serde::Deserialize)]
pub struct ReglementFfInput {
    pub facture_id: String,
    pub montant: f64,
    pub mode: String,
    pub date_reglement: String,
    pub reference: Option<String>,
}

#[tauri::command]
pub async fn ff_payer(pool: State<'_, DbPool>, input: ReglementFfInput) -> Result<ReglementFournisseur, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO reglements_fournisseurs (id,facture_id,montant,mode,date_reglement,reference) VALUES (?,?,?,?,?,?)")
        .bind(&id).bind(&input.facture_id).bind(input.montant).bind(&input.mode).bind(&input.date_reglement).bind(&input.reference)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let f: Option<FactureFournisseur> = sqlx::query_as("SELECT * FROM factures_fournisseurs WHERE id=?").bind(&input.facture_id)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let Some(f) = f {
        let paye = f.montant_paye + input.montant;
        let statut = if paye >= f.total_ttc - 0.01 { "payee" } else { "partiellement_payee" };
        sqlx::query("UPDATE factures_fournisseurs SET montant_paye=?, statut=? WHERE id=?")
            .bind(paye).bind(statut).bind(&input.facture_id).execute(&*pool).await.map_err(|e| e.to_string())?;
    }
    sqlx::query_as("SELECT * FROM reglements_fournisseurs WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ff_reglements(pool: State<'_, DbPool>, facture_id: String) -> Result<Vec<ReglementFournisseur>, String> {
    sqlx::query_as("SELECT * FROM reglements_fournisseurs WHERE facture_id=? ORDER BY date_reglement")
        .bind(facture_id).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

/// Balance âgée des dettes fournisseurs par ancienneté d'échéance.
#[tauri::command]
pub async fn balance_agee(pool: State<'_, DbPool>) -> Result<Vec<serde_json::Value>, String> {
    let rows: Vec<(Option<String>, f64, String, f64)> = sqlx::query_as(
        r#"SELECT fo.nom, f.total_ttc - f.montant_paye, f.date_echeance,
        CASE WHEN f.date_echeance >= date('now') THEN 0
             WHEN f.date_echeance >= date('now','-30 days') THEN 1
             WHEN f.date_echeance >= date('now','-60 days') THEN 2
             WHEN f.date_echeance >= date('now','-90 days') THEN 3 ELSE 4 END as tranche
        FROM factures_fournisseurs f LEFT JOIN fournisseurs fo ON fo.id=f.fournisseur_id
        WHERE f.statut IN ('emise','partiellement_payee') AND (f.total_ttc - f.montant_paye) > 0.005"#
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    let labels = ["Non échues", "< 30 jours", "30-60 jours", "60-90 jours", "> 90 jours"];
    let mut tranches = vec![0.0f64; 5];
    let mut lignes = vec![];
    for (nom, reste, ech, t) in rows {
        tranches[t as usize] += reste;
        lignes.push(serde_json::json!({"fournisseur": nom.unwrap_or_else(|| "-".into()), "reste": reste, "echeance": ech, "tranche": labels[t as usize]}));
    }
    Ok(vec![serde_json::json!({"tranches": labels.iter().enumerate().map(|(i, l)| serde_json::json!({"label": l, "total": tranches[i]})).collect::<Vec<_>>(), "dettes": lignes})])
}

// ================= ETATS OHADA =================
async fn soldes_par_compte(pool: &DbPool) -> Result<Vec<(String, String, String, f64, f64)>, String> {
    sqlx::query_as(
        "SELECT c.numero, c.intitule, c.classe, COALESCE(SUM(e.debit),0), COALESCE(SUM(e.credit),0) FROM comptes c LEFT JOIN ecritures e ON e.compte_id=c.id GROUP BY c.id ORDER BY c.numero"
    ).fetch_all(pool).await.map_err(|e| e.to_string())
}

/// Bilan simplifié SYSCOHADA : actif (emplois) / passif (ressources).
#[tauri::command]
pub async fn ohada_bilan(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    let rows = soldes_par_compte(&pool).await?;
    let mut actif_imm = 0.0; let mut actif_circ = 0.0;
    let mut capitaux = 0.0; let mut dettes = 0.0;
    let mut detail_actif = vec![]; let mut detail_passif = vec![];
    for (numero, intitule, classe, debit, credit) in rows {
        let solde = debit - credit;
        if solde.abs() < 0.005 && !(classe == "4" || classe == "5") {
            // garde les postes significatifs ; ignore les zéros sauf tiers/tréso
        }
        match classe.as_str() {
            "2" => { actif_imm += solde; detail_actif.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": solde})); }
            "3" => { actif_circ += solde; detail_actif.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": solde})); }
            "1" => { let m = credit - debit; capitaux += m; detail_passif.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": m})); }
            "4" | "5" => {
                if solde >= 0.0 { actif_circ += solde; detail_actif.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": solde})); }
                else { dettes += -solde; detail_passif.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": -solde})); }
            }
            _ => {}
        }
    }
    Ok(serde_json::json!({
        "actif_immobilise": actif_imm, "actif_circulant": actif_circ, "total_actif": actif_imm + actif_circ,
        "capitaux_propres": capitaux, "dettes": dettes, "total_passif": capitaux + dettes,
        "detail_actif": detail_actif, "detail_passif": detail_passif,
    }))
}

/// Compte de résultat : produits (classe 7) - charges (classe 6).
#[tauri::command]
pub async fn ohada_resultat(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    let rows = soldes_par_compte(&pool).await?;
    let mut produits = vec![]; let mut charges = vec![];
    let mut tot_p = 0.0; let mut tot_c = 0.0;
    for (numero, intitule, classe, debit, credit) in rows {
        if classe == "7" {
            let m = credit - debit;
            if m.abs() > 0.005 { tot_p += m; produits.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": m})); }
        } else if classe == "6" {
            let m = debit - credit;
            if m.abs() > 0.005 { tot_c += m; charges.push(serde_json::json!({"numero": numero, "intitule": intitule, "montant": m})); }
        }
    }
    Ok(serde_json::json!({
        "produits": produits, "total_produits": tot_p,
        "charges": charges, "total_charges": tot_c,
        "resultat_net": tot_p - tot_c,
    }))
}

/// État TVA : collectée (44571/443) - déductible (44566).
#[tauri::command]
pub async fn ohada_tva(pool: State<'_, DbPool>) -> Result<serde_json::Value, String> {
    let rows = soldes_par_compte(&pool).await?;
    let mut collectee = 0.0; let mut deductible = 0.0;
    for (numero, _, _, debit, credit) in rows {
        if numero.starts_with("44571") || numero.starts_with("443") {
            collectee += credit - debit;
        } else if numero.starts_with("44566") || numero.starts_with("44562") {
            deductible += debit - credit;
        }
    }
    Ok(serde_json::json!({ "collectee": collectee, "deductible": deductible, "due": collectee - deductible }))
}

// ================= FIDELITE & PROMOS =================
/// Points fidélité : 1 pt par tranche de 1000 F d'achat (query client).
#[tauri::command]
pub async fn fidelite_solde(pool: State<'_, DbPool>, client_id: String) -> Result<f64, String> {
    let row: Option<(Option<f64>,)> = sqlx::query_as("SELECT points FROM clients WHERE id=?")
        .bind(&client_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    Ok(row.and_then(|r| r.0).unwrap_or(0.0))
}

/// Conversion : 1 point = 10 F CFA de remise. Décrémente le solde.
#[tauri::command]
pub async fn fidelite_utiliser(pool: State<'_, DbPool>, client_id: String, points: f64) -> Result<f64, String> {
    let solde = fidelite_solde(pool.clone(), client_id.clone()).await?;
    if points > solde {
        return Err(format!("Solde insuffisant : {solde} pts"));
    }
    sqlx::query("UPDATE clients SET points = points - ? WHERE id=?").bind(points).bind(&client_id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(points * 10.0)
}

pub async fn fidelite_crediter(pool: &DbPool, client_id: &str, total_ttc: f64) -> Result<(), String> {
    let pts = (total_ttc / 1000.0).floor();
    if pts > 0.0 {
        sqlx::query("UPDATE clients SET points = COALESCE(points,0) + ? WHERE id=?").bind(pts).bind(client_id)
            .execute(pool).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn promos_list(pool: State<'_, DbPool>) -> Result<Vec<Promo>, String> {
    sqlx::query_as("SELECT * FROM promos ORDER BY date_fin DESC LIMIT 100")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct PromoInput {
    pub code: String,
    pub r#type: String,
    pub valeur: f64,
    pub date_debut: String,
    pub date_fin: String,
}

#[tauri::command]
pub async fn promos_create(pool: State<'_, DbPool>, app: AppHandle, token: String, input: PromoInput) -> Result<Promo, String> {
    admin(&app, &token)?;
    if !["pourcent", "montant"].contains(&input.r#type.as_str()) {
        return Err("Type promo invalide (pourcent|montant)".to_string());
    }
    let id = Uuid::new_v4().to_string();
    let code = input.code.trim().to_uppercase();
    let r = sqlx::query("INSERT INTO promos (id,code,type,valeur,date_debut,date_fin,actif) VALUES (?,?,?,?,?,?,1)")
        .bind(&id).bind(&code).bind(&input.r#type).bind(input.valeur).bind(&input.date_debut).bind(&input.date_fin)
        .execute(&*pool).await;
    if r.is_err() {
        return Err("Code promo déjà utilisé".to_string());
    }
    sqlx::query_as("SELECT * FROM promos WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn promos_toggle(pool: State<'_, DbPool>, app: AppHandle, token: String, id: String) -> Result<bool, String> {
    admin(&app, &token)?;
    sqlx::query("UPDATE promos SET actif = 1 - actif WHERE id=?").bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn promos_valider(pool: State<'_, DbPool>, code: String, total_ttc: f64) -> Result<serde_json::Value, String> {
    let p: Option<Promo> = sqlx::query_as("SELECT * FROM promos WHERE code=? AND actif=1")
        .bind(code.trim().to_uppercase()).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let p = p.ok_or_else(|| "Code promo invalide ou expiré".to_string())?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if p.date_debut > today || p.date_fin < today {
        return Err("Code promo hors période de validité".to_string());
    }
    let remise = if p.r#type == "pourcent" { total_ttc * p.valeur / 100.0 } else { p.valeur.min(total_ttc) };
    Ok(serde_json::json!({ "code": p.code, "type": p.r#type, "valeur": p.valeur, "remise": remise }))
}
