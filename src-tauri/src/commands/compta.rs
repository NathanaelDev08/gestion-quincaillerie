use crate::db::DbPool;
use crate::models::{Compte, Ecriture, Exercice, Journal};
use tauri::{AppHandle, State};
use uuid::Uuid;

#[tauri::command]
pub async fn compta_journal_list(pool: State<'_, DbPool>, journal_code: Option<String>, limit: Option<i64>) -> Result<Vec<Ecriture>, String> {
    let l = limit.unwrap_or(200).clamp(1, 1000);
    if let Some(code) = journal_code {
        sqlx::query_as("SELECT e.*, c.numero as compte_numero, j.code as journal_code FROM ecritures e JOIN comptes c ON c.id=e.compte_id JOIN journaux j ON j.id=e.journal_id WHERE j.code=? ORDER BY e.date_ecriture DESC LIMIT ?")
            .bind(code).bind(l).fetch_all(&*pool).await.map_err(|e| e.to_string())
    } else {
        sqlx::query_as("SELECT e.*, c.numero as compte_numero, j.code as journal_code FROM ecritures e JOIN comptes c ON c.id=e.compte_id JOIN journaux j ON j.id=e.journal_id ORDER BY e.date_ecriture DESC LIMIT ?")
            .bind(l).fetch_all(&*pool).await.map_err(|e| e.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct EcritureInput {
    pub journal_code: String,
    pub compte_numero: String,
    pub date_ecriture: String,
    pub libelle: String,
    pub debit: f64,
    pub credit: f64,
    pub piece_ref: Option<String>,
}

#[tauri::command]
pub async fn compta_journal_create(pool: State<'_, DbPool>, input: EcritureInput) -> Result<Ecriture, String> {
    if input.debit < 0.0 || input.credit < 0.0 { return Err("Montants invalides".to_string()); }
    let j: Option<Journal> = sqlx::query_as("SELECT * FROM journaux WHERE code=?").bind(&input.journal_code)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let c: Option<Compte> = sqlx::query_as("SELECT * FROM comptes WHERE numero=?").bind(&input.compte_numero)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let (j, c) = (j.ok_or("Journal introuvable".to_string())?, c.ok_or("Compte introuvable".to_string())?);
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref) VALUES (?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&j.id).bind(&c.id).bind(&input.date_ecriture).bind(&input.libelle)
        .bind(input.debit).bind(input.credit).bind(&input.piece_ref)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT e.*, c.numero as compte_numero, j.code as journal_code FROM ecritures e JOIN comptes c ON c.id=e.compte_id JOIN journaux j ON j.id=e.journal_id WHERE e.id=?")
        .bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compta_grand_livre(pool: State<'_, DbPool>, compte_numero: String) -> Result<Vec<Ecriture>, String> {
    sqlx::query_as("SELECT e.*, c.numero as compte_numero, j.code as journal_code FROM ecritures e JOIN comptes c ON c.id=e.compte_id JOIN journaux j ON j.id=e.journal_id WHERE c.numero=? ORDER BY e.date_ecriture")
        .bind(compte_numero).fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compta_balance(pool: State<'_, DbPool>) -> Result<Vec<serde_json::Value>, String> {
    let rows: Vec<(String, String, f64, f64)> = sqlx::query_as(
        "SELECT c.numero, c.intitule, COALESCE(SUM(e.debit),0), COALESCE(SUM(e.credit),0) FROM comptes c LEFT JOIN ecritures e ON e.compte_id=c.id GROUP BY c.id ORDER BY c.numero"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(numero, intitule, debit, credit)| {
        serde_json::json!({"numero": numero, "intitule": intitule, "debit": debit, "credit": credit, "solde": debit - credit})
    }).collect())
}

#[tauri::command]
pub async fn compta_exercices_list(pool: State<'_, DbPool>) -> Result<Vec<Exercice>, String> {
    sqlx::query_as("SELECT * FROM exercices ORDER BY date_debut DESC").fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compta_exercices_create(pool: State<'_, DbPool>, libelle: String, date_debut: String, date_fin: String) -> Result<Exercice, String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO exercices (id,libelle,date_debut,date_fin,cloture) VALUES (?,?,?,?,0)")
        .bind(&id).bind(&libelle).bind(&date_debut).bind(&date_fin)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query_as("SELECT * FROM exercices WHERE id=?").bind(&id).fetch_one(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compta_exercices_cloturer(pool: State<'_, DbPool>, app: AppHandle, token: String, id: String) -> Result<bool, String> {
    crate::authz::require_admin(&app, &token)?;
    sqlx::query("UPDATE exercices SET cloture=1 WHERE id=?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(true)
}
