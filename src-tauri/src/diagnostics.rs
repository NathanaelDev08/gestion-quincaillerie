//! Journal d'erreurs applicatif : ce qui échoue est visible dans l'application,
//! pas perdu dans la console. Un bug en production sans trace est un bug qu'on
//! ne corrige jamais.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

#[derive(Clone, serde::Serialize)]
pub struct Incident {
    pub id: u64,
    pub horodatage: String,
    pub niveau: String, // "erreur" | "avertissement" | "securite"
    pub contexte: String,
    pub message: String,
}

const CAPACITE: usize = 500;

/// Journal d'incidents en cours : (compteur d'identifiants, entrées).
pub type Journal = (u64, VecDeque<Incident>);

fn nouveau_journal() -> Mutex<Journal> {
    Mutex::new((0, VecDeque::with_capacity(CAPACITE)))
}

static JOURNAL: OnceLock<Mutex<Journal>> = OnceLock::new();

fn journal() -> &'static Mutex<Journal> {
    JOURNAL.get_or_init(nouveau_journal)
}

/// Ajoute une entrée et purge les plus anciennes au-delà de la capacité.
fn ajouter(g: &mut Journal, niveau: &str, contexte: &str, message: &str) {
    g.0 += 1;
    g.1.push_back(Incident {
        id: g.0,
        horodatage: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        niveau: niveau.to_string(),
        contexte: contexte.to_string(),
        message: message.to_string(),
    });
    while g.1.len() > CAPACITE {
        g.1.pop_front();
    }
}

/// Entrées les plus récentes d'abord, filtrées par niveau.
fn lire(g: &Journal, niveau: &str) -> Vec<Incident> {
    g.1
        .iter()
        .rev()
        .filter(|i| niveau.is_empty() || niveau == "tout" || i.niveau == niveau)
        .cloned()
        .collect()
}

fn ecrire(niveau: &str, contexte: &str, message: &str) {
    if let Ok(mut g) = journal().lock() {
        ajouter(&mut g, niveau, contexte, message);
    }
}

/// Enregistre une erreur métier ou technique.
pub fn erreur(contexte: &str, message: &str) {
    eprintln!("[ERREUR] {contexte} : {message}");
    ecrire("erreur", contexte, message);
}

pub fn avertissement(contexte: &str, message: &str) {
    eprintln!("[WARN] {contexte} : {message}");
    ecrire("avertissement", contexte, message);
}

pub fn securite(contexte: &str, message: &str) {
    eprintln!("[SECURITY] {contexte} : {message}");
    ecrire("securite", contexte, message);
}

#[tauri::command]
pub fn incidents_lister(niveau: Option<String>) -> Vec<Incident> {
    let Ok(g) = journal().lock() else { return Vec::new() };
    lire(&g, &niveau.unwrap_or_default())
}

#[tauri::command]
pub fn incidents_vider() -> bool {
    if let Ok(mut g) = journal().lock() {
        g.1.clear();
        return true;
    }
    false
}

/// Exporte le journal sur disque : l'utilisateur peut l'envoyer au support.
#[tauri::command]
pub fn incidents_exporter(app: AppHandle) -> Result<String, String> {
    let incidents = incidents_lister(Some("tout".into()));
    let dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("GestionQuincaillerie");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let chemin = dir.join(format!(
        "journal-{}.txt",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let mut texte = String::from("JOURNAL D'INCIDENTS — Gestion Quincaillerie\n");
    texte.push_str(&format!(
        "Généré le {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    for i in &incidents {
        texte.push_str(&format!(
            "[{}] {} | {} : {}\n",
            i.horodatage,
            i.niveau.to_uppercase(),
            i.contexte,
            i.message
        ));
    }
    std::fs::write(&chemin, texte).map_err(|e| e.to_string())?;
    Ok(chemin.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------
// Garde-fous de sauvegarde avant opération destructive
// ---------------------------------------------------------------------

/// Une sauvegarde de moins de `heures` existe-t-elle déjà ?
pub fn sauvegarde_recente(app: &AppHandle, heures: i64) -> bool {
    let Ok(dir) = app.path().document_dir() else { return false };
    let dir = dir.join("GestionQuincaillerie/backups");
    let limite = std::time::SystemTime::now()
        - std::time::Duration::from_secs((heures.max(0) as u64) * 3600);
    std::fs::read_dir(&dir)
        .ok()
        .map(|entrees| {
            entrees.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("backup-")
                    && e.metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|m| m > limite)
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Note dans le journal qu'une opération à risque est sur le point d'être faite.
/// Retourne `true` si une sauvegarde récente rend la sauvegarde préalable inutile.
/// La sauvegarde elle-même reste déclenchée par l'appelant, qui possède le pool.
#[allow(dead_code)]
pub fn avant_destructif(app: &AppHandle, heures: i64, raison: &str) -> bool {
    if sauvegarde_recente(app, heures) {
        return true;
    }
    avertissement(
        "Sauvegarde",
        &format!("« {raison} » : sauvegarde recommandée avant de continuer"),
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Chaque test utilise son propre journal : les tests Rust s'exécutent en
    // parallèle et partageraient sinon le même état global.

    /// Journal isolé : le verrou est unwrappé à chaque accès pour éviter de
    /// propager le `MutexGuard` (non 'static) entre fonctions.
    fn journal_de_test() -> Journal {
        let m = nouveau_journal();
        let g = m.lock().unwrap();
        (g.0, g.1.clone())
    }

    #[test]
    fn journal_borne_et_ordonne() {
        let mut g = journal_de_test();
        for i in 0..(CAPACITE + 50) {
            ajouter(&mut g, "erreur", "test", &format!("incident {i}"));
        }
        let liste = lire(&g, "");
        assert_eq!(liste.len(), CAPACITE, "le journal doit rester borné");
        assert!(
            liste[0].message.contains(&format!("{}", CAPACITE + 49)),
            "le plus récent doit être en tête"
        );
        // 550 entrées écrites, 500 conservées : les 50 plus anciennes purgées.
        assert!(
            liste[liste.len() - 1].message.contains("incident 50"),
            "le plus ancien conservé doit être le 50e, pas un plus ancien"
        );
    }

    #[test]
    fn filtre_par_niveau() {
        let mut g = journal_de_test();
        ajouter(&mut g, "erreur", "ctx", "une erreur");
        ajouter(&mut g, "avertissement", "ctx", "un avertissement");
        ajouter(&mut g, "securite", "ctx", "un refus");
        assert_eq!(lire(&g, "erreur").len(), 1);
        assert_eq!(lire(&g, "avertissement").len(), 1);
        assert_eq!(lire(&g, "securite").len(), 1);
        assert_eq!(lire(&g, "tout").len(), 3);
        assert_eq!(lire(&g, "").len(), 3);
        // Un niveau inconnu ne retourne rien, plutôt que tout.
        assert_eq!(lire(&g, "inexistant").len(), 0);
    }

    #[test]
    fn identifiants_uniques_et_chronologiques() {
        let mut g = journal_de_test();
        ajouter(&mut g, "erreur", "a", "premier");
        ajouter(&mut g, "erreur", "b", "second");
        let liste = lire(&g, "");
        assert_eq!(liste.len(), 2);
        assert!(liste[0].id > liste[1].id, "le plus récent a l'id le plus élevé");
        assert!(!liste[0].horodatage.is_empty());
        assert_eq!(liste[0].id, g.0);
    }

    #[test]
    fn la_vidange_reinitialise_le_contenu() {
        let mut g = journal_de_test();
        ajouter(&mut g, "erreur", "ctx", "x");
        g.1.clear();
        assert!(lire(&g, "").is_empty());
    }

    #[test]
    fn le_texte_exporte_contient_les_incidents() {
        let mut g = journal_de_test();
        ajouter(&mut g, "erreur", "Caisse", "vente refusée");
        let mut texte = String::new();
        for i in lire(&g, "tout") {
            texte.push_str(&format!(
                "{} {} {} {}\n",
                i.horodatage, i.niveau, i.contexte, i.message
            ));
        }
        assert!(texte.contains("vente refusée"), "le message doit figurer : {texte}");
        assert!(texte.contains("Caisse"), "le contexte doit figurer : {texte}");
    }
}
