use crate::db::{next_numero, DbPool};
use crate::models::{Avoir, CommandeFournisseur, Depense, LigneCommande, LigneCommandeInput};
use tauri::{AppHandle, State};
use uuid::Uuid;

// ================= AVOIRS =================
#[tauri::command]
pub async fn avoirs_list(pool: State<'_, DbPool>) -> Result<Vec<Avoir>, String> {
    sqlx::query_as("SELECT a.*, c.nom as client_nom, f.numero as facture_numero FROM avoirs a LEFT JOIN clients c ON c.id=a.client_id LEFT JOIN factures f ON f.id=a.facture_id ORDER BY a.created_at DESC LIMIT 200")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct AvoirInput {
    pub facture_id: Option<String>,
    pub client_id: String,
    pub date_emission: String,
    pub motif: Option<String>,
    pub lignes: Vec<crate::models::LigneInput>,
    pub retour_stock: bool,
}

#[tauri::command]
pub async fn avoirs_create(pool: State<'_, DbPool>, app: AppHandle, input: AvoirInput) -> Result<Avoir, String> {
    let mut ht = 0.0; let mut tva = 0.0;
    for l in &input.lignes {
        let lt = l.quantite * l.prix_unitaire_ht;
        ht += lt; tva += lt * l.taux_tva / 100.0;
    }
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "avoir", "AV")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO avoirs (id,numero,facture_id,client_id,date_emission,motif,total_ht,total_tva,total_ttc,statut) VALUES (?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&numero).bind(&input.facture_id).bind(&input.client_id).bind(&input.date_emission)
        .bind(&input.motif).bind(ht).bind(tva).bind(ht + tva).bind("emis")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    for l in &input.lignes {
        let lid = Uuid::new_v4().to_string();
        let total_ht = l.quantite * l.prix_unitaire_ht;
        sqlx::query("INSERT INTO lignes_avoir (id,avoir_id,designation,quantite,prix_unitaire_ht,taux_tva,total_ht) VALUES (?,?,?,?,?,?,?)")
            .bind(&lid).bind(&id).bind(&l.designation).bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva).bind(total_ht)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        // Retour marchandise en stock
        if input.retour_stock {
            if let Some(pid) = &l.produit_id {
                sqlx::query("UPDATE produits SET stock = stock + ? WHERE id=?").bind(l.quantite).bind(pid)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                let mid = Uuid::new_v4().to_string();
                let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
                    .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
                let apres = cur.map(|c| c.0).unwrap_or(0.0);
                sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,?,?,?,?,?,?)")
                    .bind(&mid).bind(pid).bind("entree").bind(l.quantite).bind(apres - l.quantite).bind(apres).bind("Retour avoir").bind(&numero)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
            }
        }
    }
    sqlx::query_as("SELECT a.*, c.nom as client_nom, f.numero as facture_numero FROM avoirs a LEFT JOIN clients c ON c.id=a.client_id LEFT JOIN factures f ON f.id=a.facture_id WHERE a.id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn avoirs_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    sqlx::query("DELETE FROM lignes_avoir WHERE avoir_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM avoirs WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

// ================= COMMANDES FOURNISSEURS =================
#[tauri::command]
pub async fn commandes_list(pool: State<'_, DbPool>) -> Result<Vec<CommandeFournisseur>, String> {
    sqlx::query_as("SELECT cf.*, f.nom as fournisseur_nom FROM commandes_fournisseurs cf LEFT JOIN fournisseurs f ON f.id=cf.fournisseur_id ORDER BY cf.created_at DESC LIMIT 200")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn commandes_get(pool: State<'_, DbPool>, id: String) -> Result<(CommandeFournisseur, Vec<LigneCommande>), String> {
    let c: Option<CommandeFournisseur> = sqlx::query_as("SELECT cf.*, f.nom as fournisseur_nom FROM commandes_fournisseurs cf LEFT JOIN fournisseurs f ON f.id=cf.fournisseur_id WHERE cf.id=?")
        .bind(&id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c = c.ok_or_else(|| "Commande introuvable".to_string())?;
    let lignes: Vec<LigneCommande> = sqlx::query_as("SELECT * FROM lignes_commande WHERE commande_id=?")
        .bind(&id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok((c, lignes))
}

#[derive(serde::Deserialize)]
pub struct CommandeInput {
    pub fournisseur_id: String,
    pub date_commande: String,
    pub date_livraison_prevue: Option<String>,
    pub notes: Option<String>,
    pub lignes: Vec<LigneCommandeInput>,
}

#[tauri::command]
pub async fn commandes_create(pool: State<'_, DbPool>, app: AppHandle, input: CommandeInput) -> Result<CommandeFournisseur, String> {
    let ht: f64 = input.lignes.iter().map(|l| l.quantite * l.prix_unitaire_ht).sum();
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "commande", "BC")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO commandes_fournisseurs (id,numero,fournisseur_id,date_commande,date_livraison_prevue,statut,total_ht,total_ttc,notes) VALUES (?,?,?,?,?,'brouillon',?,?,?)")
        .bind(&id).bind(&numero).bind(&input.fournisseur_id).bind(&input.date_commande).bind(&input.date_livraison_prevue)
        .bind(ht).bind(ht * 1.2).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    for l in &input.lignes {
        let lid = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lignes_commande (id,commande_id,produit_id,designation,quantite,quantite_recue,prix_unitaire_ht) VALUES (?,?,?,?,?,?,?)")
            .bind(&lid).bind(&id).bind(&l.produit_id).bind(&l.designation).bind(l.quantite).bind(0.0).bind(l.prix_unitaire_ht)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
    }
    let (c, _) = commandes_get(pool, id).await?;
    Ok(c)
}

#[tauri::command]
pub async fn commandes_changer_statut(pool: State<'_, DbPool>, id: String, statut: String) -> Result<CommandeFournisseur, String> {
    sqlx::query("UPDATE commandes_fournisseurs SET statut=? WHERE id=?").bind(&statut).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let (c, _) = commandes_get(pool, id).await?;
    Ok(c)
}

#[tauri::command]
pub async fn commandes_receptionner(pool: State<'_, DbPool>, id: String, recues: Vec<(String, f64)>) -> Result<CommandeFournisseur, String> {
    for (ligne_id, qte) in recues {
        let l: Option<LigneCommande> = sqlx::query_as("SELECT * FROM lignes_commande WHERE id=?").bind(&ligne_id)
            .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
        if let Some(l) = l {
            let nouvelle = (l.quantite_recue + qte).min(l.quantite);
            let delta = nouvelle - l.quantite_recue;
            sqlx::query("UPDATE lignes_commande SET quantite_recue=? WHERE id=?").bind(nouvelle).bind(&ligne_id)
                .execute(&*pool).await.map_err(|e| e.to_string())?;
            if delta > 0.0 {
                if let Some(pid) = &l.produit_id {
                    sqlx::query("UPDATE produits SET stock = stock + ? WHERE id=?").bind(delta).bind(pid)
                        .execute(&*pool).await.map_err(|e| e.to_string())?;
                    let mid = Uuid::new_v4().to_string();
                    let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
                        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
                    let apres = cur.map(|c| c.0).unwrap_or(0.0);
                    let (cmd, _) = commandes_get(pool.clone(), id.clone()).await?;
                    sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,?,?,?,?,?,?)")
                        .bind(&mid).bind(pid).bind("entree").bind(delta).bind(apres - delta).bind(apres).bind("Réception achat").bind(&cmd.numero)
                        .execute(&*pool).await.map_err(|e| e.to_string())?;
                }
            }
        }
    }
    // Statut auto
    let (_, lignes) = commandes_get(pool.clone(), id.clone()).await?;
    let tout = lignes.iter().all(|l| l.quantite_recue >= l.quantite - 0.0001);
    let partiel = lignes.iter().any(|l| l.quantite_recue > 0.0);
    let statut = if tout { "livree" } else if partiel { "partielle" } else { "validee" };
    commandes_changer_statut(pool, id, statut.into()).await
}

#[tauri::command]
pub async fn commandes_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    sqlx::query("DELETE FROM lignes_commande WHERE commande_id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM commandes_fournisseurs WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

// ================= DEPENSES =================
#[tauri::command]
pub async fn depenses_list(pool: State<'_, DbPool>, limit: Option<i64>) -> Result<Vec<Depense>, String> {
    let l = limit.unwrap_or(200).clamp(1, 1000);
    sqlx::query_as("SELECT * FROM depenses ORDER BY date_depense DESC LIMIT ?").bind(l)
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct DepenseInput {
    pub libelle: String,
    pub categorie: String,
    pub montant: f64,
    pub date_depense: String,
    pub mode: String,
    pub fournisseur_id: Option<String>,
    pub piece_ref: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn depenses_create(pool: State<'_, DbPool>, input: DepenseInput) -> Result<Depense, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO depenses (id,libelle,categorie,montant,date_depense,mode,fournisseur_id,piece_ref,notes) VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&input.libelle).bind(&input.categorie).bind(input.montant).bind(&input.date_depense)
        .bind(&input.mode).bind(&input.fournisseur_id).bind(&input.piece_ref).bind(&input.notes)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM depenses WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn depenses_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    sqlx::query("DELETE FROM depenses WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn depenses_par_categorie(pool: State<'_, DbPool>) -> Result<Vec<serde_json::Value>, String> {
    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT categorie, SUM(montant) FROM depenses WHERE date_depense >= date('now','start of month') GROUP BY categorie ORDER BY SUM(montant) DESC"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(c, t)| serde_json::json!({"categorie": c, "total": t.unwrap_or(0.0)})).collect())
}
