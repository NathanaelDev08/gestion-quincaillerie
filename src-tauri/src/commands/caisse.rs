use crate::db::{next_numero_tx, DbPool};
use crate::models::{ClotureZ, LigneInput};
use tauri::{AppHandle, State};
use uuid::Uuid;

/// Modes de paiement acceptés par la caisse (liste blanche serveur).
pub const MODES_PAIEMENT: &[&str] = &["especes", "mobile", "cb", "virement", "cheque", "credit"];

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
pub async fn vente_comptoir(
    pool: State<'_, DbPool>,
    app: AppHandle,
    token: String,
    input: VenteComptoirInput,
) -> Result<VenteComptoirResult, String> {
    let prefixe = crate::services::numbering::doc_prefix(&app, "facture", "FAC");
    let audit = ContexteAudit {
        app: app.clone(),
        token: token.clone(),
        actif: true,
    };
    vente_comptoir_interne(&pool, &prefixe, &audit, input).await
        .inspect_err(|e| {
            // Une base en panne ne doit pas se confondre avec une erreur de
            // saisie : l'origine est journalisée, la caisse bascule en veille.
            crate::diagnostics::erreur("Caisse", e);
            crate::horsligne::marquer_degrade();
        })
}

/// Contexte nécessaire au journal d'audit après une vente.
///
/// Le rejeu hors-ligne n'a pas de session : il fournit un contexte inactif,
/// la vente étant déjà tracée à son enregistrement.
pub struct ContexteAudit {
    pub app: AppHandle,
    pub token: String,
    pub actif: bool,
}

/// Cœur métier de la vente, réutilisé par le rejeu hors-ligne.
pub async fn vente_comptoir_interne(
    pool: &DbPool,
    prefixe_facture: &str,
    audit: &ContexteAudit,
    input: VenteComptoirInput,
) -> Result<VenteComptoirResult, String> {
    // 1. VALIDATION serveur : aucune confiance au frontend.
    let (ht, tva, ttc) = crate::validation::valider_document(&input.lignes)?;
    if !crate::validation::valider_montant(input.montant_recu, "Montant reçu").is_ok() {
        return Err("Montant reçu invalide".to_string());
    }
    if !MODES_PAIEMENT.iter().any(|m| m == &input.mode.as_str()) {
        return Err(format!("Mode de paiement inconnu : {}", input.mode));
    }
    // 2. Contrôle du stock AVANT toute écriture (évite le stock négatif).
    for l in &input.lignes {
        if let Some(pid) = &l.produit_id {
            let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?")
                .bind(pid).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
            match cur {
                Some((s,)) => crate::validation::verifier_stock(s, l.quantite, &l.designation)?,
                None => return Err(format!("Article introuvable : {}", l.designation)),
            }
        }
    }
    if input.montant_recu + 0.001 < ttc {
        return Err(format!(
            "Montant insuffisant : reçu {} pour un total de {}",
            input.montant_recu, ttc
        ));
    }

    // 3. TRANSACTION : tout ou rien. Un échec en cours d'écriture ne laisse
    //    ni facture orpheline, ni stock décrémenté sans vente.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let client_id = match input.client_id.clone() {
        Some(id) if !id.is_empty() => id,
        _ => {
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM clients WHERE nom='Client Comptoir' LIMIT 1")
                    .fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            if let Some((id,)) = existing {
                id
            } else {
                let id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO clients (id,nom,prenom,pays) VALUES (?,?,?,?)")
                    .bind(&id).bind("Client Comptoir").bind("").bind("Côte d'Ivoire")
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
                id
            }
        }
    };

    let fid = Uuid::new_v4().to_string();
    let numero = next_numero_tx(&mut tx, prefixe_facture).await.map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise,notes) VALUES (?,?,?,?,?,'payee',?,?,?,?,?,?)")
        .bind(&fid).bind(&numero).bind(&client_id).bind(&today).bind(&today)
        .bind(ht).bind(tva).bind(ttc).bind(ttc).bind(0.0).bind("Vente comptoir")
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    for l in &input.lignes {
        let lt = l.quantite * l.prix_unitaire_ht * (1.0 - l.remise / 100.0);
        sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(&fid).bind("facture").bind(&l.produit_id).bind(&l.designation)
            .bind(l.quantite).bind(l.prix_unitaire_ht).bind(l.taux_tva).bind(l.remise).bind(lt)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        if let Some(pid) = &l.produit_id {
            let cur: Option<(f64,)> = sqlx::query_as("SELECT stock FROM produits WHERE id=?").bind(pid)
                .fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            let avant = cur.map(|c| c.0).unwrap_or(0.0);
            let apres = avant - l.quantite;
            sqlx::query("UPDATE produits SET stock=? WHERE id=?").bind(apres).bind(pid)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            let mid = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif,document_ref) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&mid).bind(pid).bind("sortie").bind(l.quantite).bind(avant).bind(apres).bind("Vente comptoir").bind(&numero)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
    }
    sqlx::query("INSERT INTO reglements (id,facture_id,montant,mode,date_reglement,reference) VALUES (?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(&fid).bind(ttc).bind(&input.mode).bind(&today).bind(format!("Caisse {numero}"))
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    // Écritures VTE
    let jvte: Option<(String,)> = sqlx::query_as("SELECT id FROM journaux WHERE code='VTE'").fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
    let c411: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='411000'").fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
    let c707: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='707000'").fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
    let ctva: Option<(String,)> = sqlx::query_as("SELECT id FROM comptes WHERE numero='445710'").fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
    if let (Some(j), Some(c1), Some(c2), Some(c3)) = (jvte, c411, c707, ctva) {
        for (cid, debit, credit) in [(&c1.0, ttc, 0.0), (&c2.0, 0.0, ht), (&c3.0, 0.0, tva)] {
            sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&j.0).bind(cid).bind(&today).bind(format!("Caisse {numero}")).bind(debit).bind(credit).bind(&numero).bind(&fid)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
    }
    // Fidélité dans la même transaction
    let pts = (ttc / 1000.0).floor();
    if pts > 0.0 {
        let _ = sqlx::query("UPDATE clients SET points = COALESCE(points,0) + ? WHERE id=?")
            .bind(pts).bind(&client_id).execute(&mut *tx).await;
    }
    tx.commit().await.map_err(|e| format!("Enregistrement annulé : {e}"))?;

    Ok(VenteComptoirResult {
        facture_id: fid,
        numero,
        total_ttc: ttc,
        montant_recu: input.montant_recu,
        rendu: input.montant_recu - ttc,
    })
    .map(|r| {
        if audit.actif {
            // Journal d'audit hors transaction : il ne doit jamais faire échouer
            // une vente déjà validée.
            let (pool2, app2, tok2, n2, t2, nb) = (
                pool.clone(),
                audit.app.clone(),
                audit.token.clone(),
                r.numero.clone(),
                r.total_ttc,
                input.lignes.len(),
            );
            tauri::async_runtime::spawn(async move {
                crate::db::audit(
                    &pool2, &tok2, &app2, "vente_comptoir", "facture", &n2,
                    &format!("{nb} article(s), reçu {}", input.montant_recu),
                    Some(t2),
                )
                .await;
            });
        }
        crate::horsligne::marquer_normal();
        r
    })
}

// ================= CLOTURE Z =================
#[tauri::command]
pub async fn cloture_z(pool: State<'_, DbPool>, date: String) -> Result<ClotureZ, String> {
    // Format attendu : AAAA-MM-JJ. Sans cette garde, une date malformée
    // renvoyerait une clôture vide qui écraserait la vraie clôture du jour.
    let d = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| format!("Date invalide : {date} (format attendu AAAA-MM-JJ)"))?;

    // Une seule requête d'agrégation : le total et le détail des modes
    // proviennent du même instantané, donc ils ne peuvent pas diverger.
    let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
        "SELECT mode, SUM(montant) FROM reglements WHERE date_reglement = ? GROUP BY mode",
    )
    .bind(d.format("%Y-%m-%d").to_string())
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut especes = 0.0;
    let mut virement = 0.0;
    let mut cheque = 0.0;
    let mut cb = 0.0;
    let mut mobile = 0.0;
    for (mode, sum) in rows {
        let s = sum.unwrap_or(0.0);
        match mode.as_str() {
            "especes" => especes += s,
            "virement" => virement += s,
            "cheque" => cheque += s,
            "cb" => cb += s,
            // Tout mode inconnu est compté en mobile money plutôt que perdu :
            // une recette encaissée ne doit jamais disparaître d'une clôture Z.
            _ => mobile += s,
        }
    }

    let nb_total: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT facture_id) FROM reglements WHERE date_reglement = ?")
            .bind(d.format("%Y-%m-%d").to_string())
            .fetch_one(&*pool)
            .await
            .map_err(|e| e.to_string())?;
    let nb = nb_total.0;

    // Arrondi comptable. Le total est la somme DES MOTS ARRONDIS (et non
    // l'arrondi de la somme brute) : c'est la seule façon que le total affiché
    // soit exactement égal à la somme des modes affichés, au centime près.
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    let especes = r2(especes);
    let virement = r2(virement);
    let cheque = r2(cheque);
    let cb = r2(cb);
    let mobile = r2(mobile);
    let total = r2(especes + virement + cheque + cb + mobile);
    // Transaction : lecture des agrégats puis écriture cohérente. Si une
    // écriture échoue, aucun état partiel de clôture n'est laissé en base.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Idempotent : recalculer la clôture d'une journée déjà clôturée la met à
    // jour au lieu de créer une seconde ligne (pas de double comptage en Z).
    sqlx::query(
        "INSERT INTO clotures_z (id,date,nb_ventes,total,especes,virement,cheque,cb,mobile)
         VALUES (?,?,?,?,?,?,?,?,?)
         ON CONFLICT(date) DO UPDATE SET
            nb_ventes=excluded.nb_ventes, total=excluded.total,
            especes=excluded.especes, virement=excluded.virement,
            cheque=excluded.cheque, cb=excluded.cb, mobile=excluded.mobile",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&date)
    .bind(nb)
    .bind(total)
    .bind(especes)
    .bind(virement)
    .bind(cheque)
    .bind(cb)
    .bind(mobile)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    sqlx::query_as("SELECT * FROM clotures_z WHERE date = ?")
        .bind(&date)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clotures_list(pool: State<'_, DbPool>) -> Result<Vec<ClotureZ>, String> {
    sqlx::query_as("SELECT * FROM clotures_z ORDER BY date DESC LIMIT 60")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())
}
