use crate::db::{next_numero, DbPool};
use crate::models::{BonLivraison, Facture, LigneDocument, LigneInput, LigneLivraison};
use tauri::{AppHandle, State};
use uuid::Uuid;

// ================= BONS DE LIVRAISON =================
#[tauri::command]
pub async fn bls_list(pool: State<'_, DbPool>) -> Result<Vec<BonLivraison>, String> {
    sqlx::query_as("SELECT b.*, c.nom as client_nom FROM bons_livraison b LEFT JOIN clients c ON c.id=b.client_id ORDER BY b.created_at DESC LIMIT 200")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bls_get(pool: State<'_, DbPool>, id: String) -> Result<(BonLivraison, Vec<LigneLivraison>), String> {
    let b: Option<BonLivraison> = sqlx::query_as("SELECT b.*, c.nom as client_nom FROM bons_livraison b LEFT JOIN clients c ON c.id=b.client_id WHERE b.id=?")
        .bind(&id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let b = b.ok_or_else(|| "Bon de livraison introuvable".to_string())?;
    let lignes: Vec<LigneLivraison> = sqlx::query_as("SELECT * FROM lignes_livraison WHERE bl_id=?")
        .bind(&id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok((b, lignes))
}

#[derive(serde::Deserialize)]
pub struct BlInput {
    pub client_id: String,
    pub devis_id: Option<String>,
    pub date_livraison: String,
    pub notes: Option<String>,
    pub lignes: Vec<LigneInput>,
}

async fn insert_bl(pool: &DbPool, prefix: &str, input: &BlInput) -> Result<BonLivraison, String> {
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(pool, prefix).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO bons_livraison (id,numero,client_id,devis_id,date_livraison,statut,notes) VALUES (?,?,?,?,?,'brouillon',?)")
        .bind(&id).bind(&numero).bind(&input.client_id).bind(&input.devis_id).bind(&input.date_livraison).bind(&input.notes)
        .execute(pool).await.map_err(|e| e.to_string())?;
    for l in &input.lignes {
        sqlx::query("INSERT INTO lignes_livraison (id,bl_id,produit_id,designation,quantite,prix_unitaire_ht,taux_tva) VALUES (?,?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(&id).bind(&l.produit_id).bind(&l.designation)
            .bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva)
            .execute(pool).await.map_err(|e| e.to_string())?;
    }
    let (b, _) = bls_get_inner(pool, &id).await?;
    Ok(b)
}

async fn bls_get_inner(pool: &DbPool, id: &str) -> Result<(BonLivraison, Vec<LigneLivraison>), String> {
    let b: Option<BonLivraison> = sqlx::query_as("SELECT b.*, c.nom as client_nom FROM bons_livraison b LEFT JOIN clients c ON c.id=b.client_id WHERE b.id=?")
        .bind(id).fetch_optional(pool).await.map_err(|e| e.to_string())?;
    let b = b.ok_or_else(|| "Bon de livraison introuvable".to_string())?;
    let lignes: Vec<LigneLivraison> = sqlx::query_as("SELECT * FROM lignes_livraison WHERE bl_id=?")
        .bind(id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    Ok((b, lignes))
}

#[tauri::command]
pub async fn bls_create(pool: State<'_, DbPool>, app: AppHandle, input: BlInput) -> Result<BonLivraison, String> {
    let prefix = crate::services::numbering::doc_prefix(&app, "livraison", "BL");
    insert_bl(&pool, &prefix, &input).await
}

#[tauri::command]
pub async fn bls_from_devis(pool: State<'_, DbPool>, app: AppHandle, devis_id: String) -> Result<BonLivraison, String> {
    let lignes: Vec<LigneDocument> = sqlx::query_as("SELECT * FROM lignes_document WHERE document_id=?")
        .bind(&devis_id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    if lignes.is_empty() {
        return Err("Devis sans lignes".to_string());
    }
    let d: Option<(String, String)> = sqlx::query_as("SELECT client_id, date_emission FROM devis WHERE id=?")
        .bind(&devis_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let (client_id, _) = d.ok_or_else(|| "Devis introuvable".to_string())?;
    let input = BlInput {
        client_id,
        devis_id: Some(devis_id.clone()),
        date_livraison: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        notes: None,
        lignes: lignes.into_iter().map(|l| LigneInput {
            produit_id: l.produit_id, designation: l.designation, quantite: l.quantite,
            prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: 0.0,
        }).collect(),
    };
    let b = insert_bl(&pool, &crate::services::numbering::doc_prefix(&app, "livraison", "BL"), &input).await?;
    sqlx::query("UPDATE devis SET statut='accepte' WHERE id=?").bind(&devis_id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(b)
}

#[tauri::command]
pub async fn bls_changer_statut(pool: State<'_, DbPool>, id: String, statut: String) -> Result<BonLivraison, String> {
    let (avant, lignes) = bls_get_inner(&pool, &id).await?;
    let deja_livre = avant.statut == "livre" || avant.statut == "facture";
    sqlx::query("UPDATE bons_livraison SET statut=? WHERE id=?").bind(&statut).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // Sortie de stock unique au passage à "livre"
    if statut == "livre" && !deja_livre {
        let (b, _) = bls_get_inner(&pool, &id).await?;
        for l in &lignes {
            if let Some(pid) = &l.produit_id {
                sqlx::query("UPDATE produits SET stock = stock - ? WHERE id=?").bind(l.quantite).bind(pid)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                let mid = Uuid::new_v4().to_string();
                let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
                    .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
                let apres = cur.map(|c| c.0).unwrap_or(0.0);
                sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,?,?,?,?,?,?)")
                    .bind(&mid).bind(pid).bind("sortie").bind(l.quantite).bind(apres + l.quantite).bind(apres).bind("Livraison client").bind(&b.numero)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
            }
        }
    }
    let (b, _) = bls_get_inner(&pool, &id).await?;
    Ok(b)
}

#[tauri::command]
pub async fn bls_to_facture(pool: State<'_, DbPool>, app: AppHandle, id: String) -> Result<Facture, String> {
    let (b, lignes) = bls_get_inner(&pool, &id).await?;
    let (ht, tva): (f64, f64) = lignes.iter().map(|l| {
        let lt = l.quantite * l.prix_unitaire_ht;
        (lt, lt * l.taux_tva / 100.0)
    }).fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let fid = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "facture", "FAC")).await.map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT INTO factures (id,numero,client_id,devis_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise) VALUES (?,?,?,?,?,'emise',?,?,?,?,?,0)")
        .bind(&fid).bind(&numero).bind(&b.client_id).bind(&b.devis_id).bind(&today).bind(&today)
        .bind(ht).bind(tva).bind(ht + tva).bind(0.0)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    for l in &lignes {
        let lt = l.quantite * l.prix_unitaire_ht;
        sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(&fid).bind("facture").bind(&l.produit_id).bind(&l.designation)
            .bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva).bind(0.0).bind(lt)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
    }
    // Écritures VTE
    let jvte: Option<(String,)> = sqlx::query_as("SELECT id FROM journaux WHERE code='VTE'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c411: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='411000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c707: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='707000'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let ctva: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='445710'").fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if let (Some(j), Some(c1), Some(c2), Some(c3)) = (jvte, c411, c707, ctva) {
        for (cid, debit, credit) in [(&c1.0, ht + tva, 0.0), (&c2.0, 0.0, ht), (&c3.0, 0.0, tva)] {
            sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&j.0).bind(cid).bind(&today).bind(format!("Facture {numero}")).bind(debit).bind(credit).bind(&numero).bind(&fid)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
        }
    }
    sqlx::query("UPDATE bons_livraison SET statut='facture' WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    let f: Option<Facture> = sqlx::query_as("SELECT f.*, c.nom as client_nom FROM factures f LEFT JOIN clients c ON c.id=f.client_id WHERE f.id=?")
        .bind(&fid).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    f.ok_or_else(|| "Facture introuvable".to_string())
}

#[tauri::command]
pub async fn bls_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    sqlx::query("DELETE FROM lignes_livraison WHERE bl_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM bons_livraison WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}
