use crate::db::{next_numero, DbPool};
use crate::models::{Bulletin, Employe};
use tauri::{AppHandle, State};
use uuid::Uuid;

const TAUX_CNPS: f64 = 0.063;
const TAUX_ITS: f64 = 0.10;

// ================= EMPLOYES =================
#[tauri::command]
pub async fn employes_list(pool: State<'_, DbPool>, actifs_only: Option<bool>) -> Result<Vec<Employe>, String> {
    if actifs_only.unwrap_or(false) {
        sqlx::query_as("SELECT * FROM employes WHERE actif=1 ORDER BY nom").fetch_all(&*pool).await.map_err(|e| e.to_string())
    } else {
        sqlx::query_as("SELECT * FROM employes ORDER BY nom").fetch_all(&*pool).await.map_err(|e| e.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct EmployeInput {
    pub nom: String,
    pub prenom: Option<String>,
    pub poste: Option<String>,
    pub telephone: Option<String>,
    pub salaire_base: f64,
    pub date_embauche: Option<String>,
}

#[tauri::command]
pub async fn employes_create(pool: State<'_, DbPool>, input: EmployeInput) -> Result<Employe, String> {
    if input.nom.trim().is_empty() {
        return Err("Nom obligatoire".to_string());
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO employes (id,nom,prenom,poste,telephone,salaire_base,date_embauche,actif) VALUES (?,?,?,?,?,?,?,1)")
        .bind(&id).bind(&input.nom).bind(&input.prenom).bind(&input.poste).bind(&input.telephone)
        .bind(input.salaire_base).bind(&input.date_embauche)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM employes WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn employes_update(pool: State<'_, DbPool>, id: String, input: EmployeInput) -> Result<Employe, String> {
    sqlx::query("UPDATE employes SET nom=?,prenom=?,poste=?,telephone=?,salaire_base=?,date_embauche=? WHERE id=?")
        .bind(&input.nom).bind(&input.prenom).bind(&input.poste).bind(&input.telephone)
        .bind(input.salaire_base).bind(&input.date_embauche).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM employes WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn employes_toggle_actif(pool: State<'_, DbPool>, id: String) -> Result<Employe, String> {
    sqlx::query("UPDATE employes SET actif = 1 - actif WHERE id=?").bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM employes WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

// ================= BULLETINS =================
#[tauri::command]
pub async fn bulletins_list(pool: State<'_, DbPool>, periode: Option<String>) -> Result<Vec<Bulletin>, String> {
    if let Some(p) = periode {
        sqlx::query_as("SELECT b.*, e.nom || ' ' || COALESCE(e.prenom,'') as employe_nom FROM bulletins b LEFT JOIN employes e ON e.id=b.employe_id WHERE b.periode=? ORDER BY e.nom")
            .bind(p).fetch_all(&*pool).await.map_err(|e| e.to_string())
    } else {
        sqlx::query_as("SELECT b.*, e.nom || ' ' || COALESCE(e.prenom,'') as employe_nom FROM bulletins b LEFT JOIN employes e ON e.id=b.employe_id ORDER BY b.periode DESC, e.nom LIMIT 200")
            .fetch_all(&*pool).await.map_err(|e| e.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct BulletinInput {
    pub employe_id: String,
    pub periode: String,
    pub brut: f64,
}

#[tauri::command]
pub async fn bulletins_create(pool: State<'_, DbPool>, app: AppHandle, input: BulletinInput) -> Result<Bulletin, String> {
    if input.brut <= 0.0 {
        return Err("Salaire brut invalide".to_string());
    }
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM bulletins WHERE employe_id=? AND periode=?")
        .bind(&input.employe_id).bind(&input.periode).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    if existing.is_some() {
        return Err("Bulletin déjà établi pour cet employé et cette période".to_string());
    }
    let cnps = (input.brut * TAUX_CNPS).round();
    let its = ((input.brut - cnps) * TAUX_ITS).round();
    let net = input.brut - cnps - its;
    let id = Uuid::new_v4().to_string();
    let numero = next_numero(&pool, &crate::services::numbering::doc_prefix(&app, "bulletin", "BUL")).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO bulletins (id,numero,employe_id,periode,brut,cnps,its,net,statut) VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&numero).bind(&input.employe_id).bind(&input.periode)
        .bind(input.brut).bind(cnps).bind(its).bind(net).bind("brouillon")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT b.*, e.nom || ' ' || COALESCE(e.prenom,'') as employe_nom FROM bulletins b LEFT JOIN employes e ON e.id=b.employe_id WHERE b.id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bulletins_changer_statut(pool: State<'_, DbPool>, id: String, statut: String) -> Result<Bulletin, String> {
    if !["brouillon", "valide", "paye"].contains(&statut.as_str()) {
        return Err("Statut inconnu".to_string());
    }
    sqlx::query("UPDATE bulletins SET statut=? WHERE id=?").bind(&statut).bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    // Paiement -> dépense automatique (une seule fois)
    if statut == "paye" {
        let b: Option<Bulletin> = sqlx::query_as("SELECT * FROM bulletins WHERE id=?").bind(&id)
            .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
        if let Some(b) = b {
            let emp: Option<(String, String)> = sqlx::query_as("SELECT nom, COALESCE(prenom,'') FROM employes WHERE id=?")
                .bind(&b.employe_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
            let (nom, prenom) = emp.unwrap_or_default();
            let deja: Option<(String,)> = sqlx::query_as("SELECT id FROM depenses WHERE piece_ref=?").bind(&id)
                .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
            if deja.is_none() {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                sqlx::query("INSERT INTO depenses (id,libelle,categorie,montant,date_depense,mode,piece_ref) VALUES (?,?,?,?,?,?,?)")
                    .bind(Uuid::new_v4().to_string())
                    .bind(format!("Salaire {nom} {prenom} ({})", b.periode))
                    .bind("salaires").bind(b.net).bind(&today).bind("virement").bind(&id)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
            }
        }
    }
    sqlx::query_as("SELECT b.*, e.nom || ' ' || COALESCE(e.prenom,'') as employe_nom FROM bulletins b LEFT JOIN employes e ON e.id=b.employe_id WHERE b.id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bulletins_delete(pool: State<'_, DbPool>, id: String) -> Result<bool, String> {
    sqlx::query("DELETE FROM bulletins WHERE id=? AND statut='brouillon'").bind(&id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn masse_salariale(pool: State<'_, DbPool>, periode: String) -> Result<serde_json::Value, String> {
    let row: (Option<f64>, Option<f64>, Option<f64>, Option<i64>) = sqlx::query_as(
        "SELECT SUM(brut), SUM(cnps + its), SUM(net), COUNT(*) FROM bulletins WHERE periode=?"
    ).bind(&periode).fetch_one(&*pool).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "brut": row.0.unwrap_or(0.0), "charges": row.1.unwrap_or(0.0),
        "net": row.2.unwrap_or(0.0), "nb": row.3.unwrap_or(0),
    }))
}
