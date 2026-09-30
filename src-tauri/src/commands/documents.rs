use crate::db::{next_numero, DbPool};
use crate::models::{Devis, Facture, LigneDocument, LigneInput, Paged, Reglement};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

fn calc_totaux(lignes: &[LigneInput], remise_pct: f64) -> (f64, f64, f64) {
    let mut ht = 0.0;
    let mut tva = 0.0;
    for l in lignes {
        let lt = l.quantite * l.prix_unitaire_ht * (1.0 - l.remise / 100.0);
        ht += lt;
        tva += lt * l.taux_tva / 100.0;
    }
    ht *= 1.0 - remise_pct / 100.0;
    tva *= 1.0 - remise_pct / 100.0;
    (ht, tva, ht + tva)
}

async fn save_lignes(pool: &DbPool, doc_id: &str, dtype: &str, lignes: &[LigneInput]) -> Result<(), String> {
    sqlx::query("DELETE FROM lignes_document WHERE document_id=?").bind(doc_id)
        .execute(pool).await.map_err(|e| e.to_string())?;
    for l in lignes {
        let id = Uuid::new_v4().to_string();
        let total_ht = l.quantite * l.prix_unitaire_ht * (1.0 - l.remise / 100.0);
        sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(doc_id).bind(dtype).bind(&l.produit_id).bind(&l.designation)
            .bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva).bind(l.remise).bind(total_ht)
            .execute(pool).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ================= DEVIS =================
#[derive(serde::Deserialize)]
pub struct DevisInput {
    pub client_id: String,
    pub date_emission: String,
    pub date_validite: String,
    pub remise: f64,
    pub notes: Option<String>,
    pub conditions: Option<String>,
    pub lignes: Vec<LigneInput>,
}

#[tauri::command]
pub async fn devis_list(pool: State<'_, DbPool>, page: Option<i64>, per_page: Option<i64>) -> Result<Paged<Devis>, String> {
    let (page, per_page) = (page.unwrap_or(1).max(1), per_page.unwrap_or(50).clamp(1,200));
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devis").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let data: Vec<Devis> = sqlx::query_as("SELECT d.*, c.nom as client_nom FROM devis d LEFT JOIN clients c ON c.id=d.client_id ORDER BY d.created_at DESC LIMIT ? OFFSET ?")
        .bind(per_page).bind((page-1)*per_page).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(Paged { data, total: total.0, page, per_page })
}

#[tauri::command]
pub async fn devis_get(pool: State<'_, DbPool>, id: String) -> Result<(Devis, Vec<LigneDocument>), String> {
    let d: Option<Devis> = sqlx::query_as("SELECT d.*, c.nom as client_nom FROM devis d LEFT JOIN clients c ON c.id=d.client_id WHERE d.id=?")
        .bind(&id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let d = d.ok_or_else(|| "Devis introuvable".to_string())?;
    let lignes: Vec<LigneDocument> = sqlx::query_as("SELECT * FROM lignes_document WHERE document_id=?")
        .bind(&id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok((d, lignes))
}

#[tauri::command]
pub async fn devis_create(pool: State<'_, DbPool>, app: AppHandle, input: DevisInput) -> Result<Devis, String> {
    let (ht, tva, ttc) = calc_totaux(&input.lignes, input.remise);
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "devis", "DEV")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO devis (id,numero,client_id,date_emission,date_validite,statut,total_ht,total_tva,total_ttc,remise,notes,conditions) VALUES (?,?,?,?,?,'brouillon',?,?,?,?,?,?)")
        .bind(&id).bind(&numero).bind(&input.client_id).bind(&input.date_emission).bind(&input.date_validite)
        .bind(ht).bind(tva).bind(ttc).bind(input.remise).bind(&input.notes).bind(&input.conditions)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    save_lignes(&pool, &id, "devis", &input.lignes).await?;
    let (d, _) = devis_get(pool, id).await?;
    Ok(d)
}

#[tauri::command]
pub async fn devis_update(pool: State<'_, DbPool>, id: String, input: DevisInput) -> Result<Devis, String> {
    let (ht, tva, ttc) = calc_totaux(&input.lignes, input.remise);
    sqlx::query("UPDATE devis SET client_id=?,date_emission=?,date_validite=?,total_ht=?,total_tva=?,total_ttc=?,remise=?,notes=?,conditions=? WHERE id=?")
        .bind(&input.client_id).bind(&input.date_emission).bind(&input.date_validite)
        .bind(ht).bind(tva).bind(ttc).bind(input.remise).bind(&input.notes).bind(&input.conditions).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    save_lignes(&pool, &id, "devis", &input.lignes).await?;
    let (d, _) = devis_get(pool, id).await?;
    Ok(d)
}

#[tauri::command]
pub async fn devis_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    let used: (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM factures WHERE devis_id=?) + (SELECT COUNT(*) FROM bons_livraison WHERE devis_id=?)"
    ).bind(&id).bind(&id)
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if used.0 > 0 {
        return Err("Suppression impossible : une facture ou un bon de livraison est issu de ce devis.".to_string());
    }
    sqlx::query("DELETE FROM lignes_document WHERE document_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM devis WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn devis_dupliquer(pool: State<'_, DbPool>, app: AppHandle, id: String) -> Result<Devis, String> {
    let (d, lignes) = devis_get(pool.clone(), id).await?;
    let input = DevisInput {
        client_id: d.client_id, date_emission: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        date_validite: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        remise: d.remise, notes: d.notes, conditions: d.conditions,
        lignes: lignes.into_iter().map(|l| LigneInput {
            produit_id: l.produit_id, designation: l.designation, quantite: l.quantite,
            prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: l.remise,
        }).collect(),
    };
    devis_create(pool, app, input).await
}

#[tauri::command]
pub async fn devis_changer_statut(pool: State<'_, DbPool>, id: String, statut: String) -> Result<Devis, String> {
    sqlx::query("UPDATE devis SET statut=? WHERE id=?").bind(&statut).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let (d, _) = devis_get(pool, id).await?;
    Ok(d)
}

#[tauri::command]
pub async fn devis_generer_pdf(app: AppHandle, pool: State<'_, DbPool>, id: String) -> Result<String, String> {
    let (d, lignes) = devis_get(pool, id).await?;
    let dir = app.path().document_dir().map_err(|e| e.to_string())?.join("GestionCommerciale");
    let items: Vec<(String, f64, f64, f64)> = lignes.into_iter().map(|l| (l.designation, l.quantite, l.prix_unitaire_ht, l.total_ht)).collect();
    let html = crate::services::pdf::build_facture_html(
        &serde_json::json!({"company_name":"Ma Société"}),
        &d.numero, &d.client_nom.unwrap_or_default(), &items, d.total_ht, d.total_tva, d.total_ttc,
    );
    crate::services::pdf::write_html_file(&dir, &format!("{}-{}.html", d.numero, "devis"), &html).map_err(|e| e.to_string())
}

// ================= FACTURES =================
#[derive(serde::Deserialize)]
pub struct FactureInput {
    pub client_id: String,
    pub devis_id: Option<String>,
    pub date_emission: String,
    pub date_echeance: String,
    pub remise: f64,
    pub notes: Option<String>,
    pub lignes: Vec<LigneInput>,
}

#[tauri::command]
pub async fn factures_list(pool: State<'_, DbPool>, page: Option<i64>, per_page: Option<i64>) -> Result<Paged<Facture>, String> {
    let (page, per_page) = (page.unwrap_or(1).max(1), per_page.unwrap_or(50).clamp(1,200));
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM factures").fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    let data: Vec<Facture> = sqlx::query_as("SELECT f.*, c.nom as client_nom FROM factures f LEFT JOIN clients c ON c.id=f.client_id ORDER BY f.created_at DESC LIMIT ? OFFSET ?")
        .bind(per_page).bind((page-1)*per_page).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(Paged { data, total: total.0, page, per_page })
}

#[tauri::command]
pub async fn factures_get(pool: State<'_, DbPool>, id: String) -> Result<(Facture, Vec<LigneDocument>, Vec<Reglement>), String> {
    let f: Option<Facture> = sqlx::query_as("SELECT f.*, c.nom as client_nom FROM factures f LEFT JOIN clients c ON c.id=f.client_id WHERE f.id=?")
        .bind(&id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let f = f.ok_or_else(|| "Facture introuvable".to_string())?;
    let lignes: Vec<LigneDocument> = sqlx::query_as("SELECT * FROM lignes_document WHERE document_id=?").bind(&id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    let regs: Vec<Reglement> = sqlx::query_as("SELECT * FROM reglements WHERE facture_id=? ORDER BY date_reglement").bind(&id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok((f, lignes, regs))
}

#[tauri::command]
pub async fn factures_create(pool: State<'_, DbPool>, app: AppHandle, input: FactureInput) -> Result<Facture, String> {
    let (ht, tva, ttc) = calc_totaux(&input.lignes, input.remise);
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "facture", "FAC")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO factures (id,numero,client_id,devis_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES (?,?,?,?,?,'emise',?,?,?,?,?,?)")
        .bind(&id).bind(&numero).bind(&input.client_id).bind(&input.devis_id).bind(&input.date_emission).bind(&input.date_echeance)
        .bind(ht).bind(tva).bind(ttc).bind(0.0).bind(input.remise).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    save_lignes(&pool, &id, "facture", &input.lignes).await?;
    // Décrémenter stock
    for l in &input.lignes {
        if let Some(pid) = &l.produit_id {
            sqlx::query("UPDATE produits SET stock = stock - ? WHERE id=?").bind(l.quantite).bind(pid)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
        }
    }
    // Écritures comptables auto (vente)
    let jvte: Option<(String,)> = sqlx::query_as("SELECT id FROM journaux WHERE code='VTE'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c411: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='411000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c707: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='707000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let ctva: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='445710'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let (Some(j), Some(c1), Some(c2), Some(c3)) = (jvte, c411, c707, ctva) {
        let eid = |_: &str| Uuid::new_v4().to_string();
        let e1 = eid(""); let e2 = eid(""); let e3 = eid("");
        sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?, ?,?)")
            .bind(&e1).bind(&j.0).bind(&c1.0).bind(&input.date_emission).bind(format!("Facture {}", numero)).bind(ttc).bind(0.0).bind(&numero).bind(&id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?, ?,?)")
            .bind(&e2).bind(&j.0).bind(&c2.0).bind(&input.date_emission).bind(format!("Facture {}", numero)).bind(0.0).bind(ht).bind(&numero).bind(&id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?, ?,?)")
            .bind(&e3).bind(&j.0).bind(&c3.0).bind(&input.date_emission).bind(format!("Facture {}", numero)).bind(0.0).bind(tva).bind(&numero).bind(&id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
    }
    let (f, _, _) = factures_get(pool, id).await?;
    Ok(f)
}

#[tauri::command]
pub async fn factures_update(pool: State<'_, DbPool>, id: String, input: FactureInput) -> Result<Facture, String> {
    let (ht, tva, ttc) = calc_totaux(&input.lignes, input.remise);
    sqlx::query("UPDATE factures SET client_id=?,date_emission=?,date_echeance=?,total_ht=?,total_tva=?,total_ttc=?,remise=?,notes=? WHERE id=?")
        .bind(&input.client_id).bind(&input.date_emission).bind(&input.date_echeance)
        .bind(ht).bind(tva).bind(ttc).bind(input.remise).bind(&input.notes).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    save_lignes(&pool, &id, "facture", &input.lignes).await?;
    let (f, _, _) = factures_get(pool, id).await?;
    Ok(f)
}

#[tauri::command]
pub async fn factures_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    let used: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM avoirs WHERE facture_id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    if used.0 > 0 {
        return Err("Suppression impossible : un avoir est lié à cette facture (supprimez d'abord l'avoir).".to_string());
    }
    sqlx::query("DELETE FROM lignes_document WHERE document_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM reglements WHERE facture_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM ecritures WHERE facture_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM factures WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn factures_dupliquer(pool: State<'_, DbPool>, app: AppHandle, id: String) -> Result<Facture, String> {
    let (f, lignes, _) = factures_get(pool.clone(), id).await?;
    let input = FactureInput {
        client_id: f.client_id, devis_id: None,
        date_emission: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        date_echeance: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        remise: f.remise, notes: f.notes,
        lignes: lignes.into_iter().map(|l| LigneInput {
            produit_id: l.produit_id, designation: l.designation, quantite: l.quantite,
            prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: l.remise,
        }).collect(),
    };
    factures_create(pool, app, input).await
}

#[tauri::command]
pub async fn factures_changer_statut(pool: State<'_, DbPool>, id: String, statut: String) -> Result<Facture, String> {
    sqlx::query("UPDATE factures SET statut=? WHERE id=?").bind(&statut).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let (f, _, _) = factures_get(pool, id).await?;
    Ok(f)
}

#[tauri::command]
pub async fn factures_from_devis(pool: State<'_, DbPool>, app: AppHandle, devis_id: String) -> Result<Facture, String> {
    let (d, lignes) = devis_get(pool.clone(), devis_id.clone()).await?;
    let input = FactureInput {
        client_id: d.client_id, devis_id: Some(devis_id.clone()),
        date_emission: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        date_echeance: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        remise: d.remise, notes: d.notes,
        lignes: lignes.into_iter().map(|l| LigneInput {
            produit_id: l.produit_id, designation: l.designation, quantite: l.quantite,
            prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: l.remise,
        }).collect(),
    };
    let f = factures_create(pool.clone(), app, input).await?;
    sqlx::query("UPDATE devis SET statut='accepte' WHERE id=?").bind(&devis_id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(f)
}

#[tauri::command]
pub async fn factures_generer_pdf(app: AppHandle, pool: State<'_, DbPool>, id: String) -> Result<String, String> {
    let (f, lignes, _) = factures_get(pool, id).await?;
    let dir = app.path().document_dir().map_err(|e| e.to_string())?.join("GestionCommerciale");
    let items: Vec<(String, f64, f64, f64)> = lignes.into_iter().map(|l| (l.designation, l.quantite, l.prix_unitaire_ht, l.total_ht)).collect();
    let html = crate::services::pdf::build_facture_html(
        &serde_json::json!({"company_name":"Ma Société"}),
        &f.numero, &f.client_nom.unwrap_or_default(), &items, f.total_ht, f.total_tva, f.total_ttc,
    );
    crate::services::pdf::write_html_file(&dir, &format!("{}.html", f.numero), &html).map_err(|e| e.to_string())
}

// ================= REGLEMENTS =================
#[tauri::command]
pub async fn reglements_list(pool: State<'_, DbPool>, facture_id: Option<String>) -> Result<Vec<Reglement>, String> {
    if let Some(fid) = facture_id {
        sqlx::query_as("SELECT * FROM reglements WHERE facture_id=? ORDER BY date_reglement")
            .bind(fid).fetch_all(&*pool).await.map_err(|e| e.to_string())
    } else {
        sqlx::query_as("SELECT * FROM reglements ORDER BY date_reglement DESC LIMIT 200")
            .fetch_all(&*pool).await.map_err(|e| e.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct ReglementInput {
    pub facture_id: String,
    pub montant: f64,
    pub mode: String,
    pub date_reglement: String,
    pub reference: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn reglements_create(pool: State<'_, DbPool>, input: ReglementInput) -> Result<Reglement, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO reglements (id,facture_id,montant,mode,date_reglement,reference,notes) VALUES (?,?,?,?,?,?,?)")
        .bind(&id).bind(&input.facture_id).bind(input.montant).bind(&input.mode)
        .bind(&input.date_reglement).bind(&input.reference).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // MAJ montant payé + statut
    let f: Option<Facture> = sqlx::query_as("SELECT * FROM factures WHERE id=?").bind(&input.facture_id)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let Some(f) = f {
        let paye = f.montant_paye + input.montant;
        let statut = if paye >= f.total_ttc - 0.01 { "payee" } else { "partiellement_payee" };
        sqlx::query("UPDATE factures SET montant_paye=?, statut=? WHERE id=?")
            .bind(paye).bind(statut).bind(&input.facture_id)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        // Fidélité : 1 pt / 1000 F dès le paiement complet
        if paye >= f.total_ttc - 0.01 {
            let _ = crate::commands::fidelite_crediter(&pool, &f.client_id, f.total_ttc).await;
        }
    }
    sqlx::query_as("SELECT * FROM reglements WHERE id=?").bind(&id)
        .fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reglements_lettrage(pool: State<'_, DbPool>, facture_id: String) -> Result<serde_json::Value, String> {
    let (f, _, regs) = factures_get(pool, facture_id).await?;
    let total_regle: f64 = regs.iter().map(|r| r.montant).sum();
    Ok(serde_json::json!({
        "total_ttc": f.total_ttc,
        "total_regle": total_regle,
        "reste": f.total_ttc - total_regle,
        "lettre": total_regle >= f.total_ttc - 0.01
    }))
}
