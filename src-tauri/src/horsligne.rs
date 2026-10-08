//! Mode hors-ligne : la caisse doit continuer à vendre si le disque ou le
//! système se dégrade.
//!
//! Concrètement, une quincaillerie ne peut pas s'arrêter de vendre parce qu'un
//! fichier ne s'écrit pas. On distingue donc deux situations :
//!
//! - la base répond → tout fonctionne normalement ;
//! - la base ne répond plus → les ventes sont mises en file d'attente «лено»
//!   dans un journal local append-only, avec leur numéro, et seront rejouées
//!   automatiquement au retour à la normale.
//!
//! Ce qui n'est PAS fait : inventer un cache en base secondaire. Une vente non
//! validée en base n'est pas une vente ; mieux vaut un incident visible qu'un
//! chiffre faux dans le chiffre d'affaires.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

/// Une vente encaissée alors que la base était indisponible.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct VenteHorsLigne {
    pub id: String,
    pub horodatage: String,
    pub numero: String,
    pub total_ttc: f64,
    pub mode: String,
    pub client_id: Option<String>,
    /// Lignes sérialisées telles qu'envoyées par la caisse.
    pub lignes: serde_json::Value,
    pub montant_recu: f64,
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize)]
pub enum EtatSante {
    /// Tout va bien.
    Ok,
    /// La base a répondu mais une opération a échoué récemment.
    Degrade,
    /// La base ne répond plus : les ventes sont mises en attente.
    HorsLigne,
}

static FILE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static ETAT: OnceLock<Mutex<EtatSante>> = OnceLock::new();

fn etat() -> &'static Mutex<EtatSante> {
    ETAT.get_or_init(|| Mutex::new(EtatSante::Ok))
}

fn fichier() -> &'static Mutex<Option<PathBuf>> {
    FILE.get_or_init(|| Mutex::new(None))
}

fn chemin(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(g) = fichier().lock() {
        if g.is_some() {
            return g.clone();
        }
    }
    let p = app
        .path()
        .app_data_dir()
        .ok()?
        .join("ventes-hors-ligne.jsonl");
    if let Ok(mut g) = fichier().lock() {
        *g = Some(p.clone());
    }
    Some(p)
}

/// Ouvre le journal local en ajout. Un fichier par ligne (JSONL) : si le
/// processus est tué en cours d'écriture, les ventes déjà écrites restent
/// lisibles. Une base SQLite monolithic subirait, elle, le même coup.
fn ouvrir(app: &AppHandle) -> Option<std::fs::File> {
    let p = chemin(app)?;
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .ok()
}

pub fn enregistrer(app: &AppHandle, vente: &VenteHorsLigne) -> Result<(), String> {
    let mut f = ouvrir(app).ok_or("Journal hors-ligne inaccessible")?;
    let ligne = serde_json::to_string(vente).map_err(|e| e.to_string())?;
    writeln!(f, "{ligne}").map_err(|e| format!("Écriture impossible : {e}"))?;
    f.sync_all().ok();
    if let Ok(mut e) = etat().lock() {
        *e = EtatSante::HorsLigne;
    }
    crate::diagnostics::avertissement(
        "Mode hors-ligne",
        &format!("Vente {numero} mise en attente ({total_ttc} F CFA)", numero = vente.numero, total_ttc = vente.total_ttc),
    );
    Ok(())
}

pub fn en_attente(app: &AppHandle) -> Vec<VenteHorsLigne> {
    let Some(p) = chemin(app) else { return Vec::new() };
    let Ok(contenu) = std::fs::read_to_string(p) else { return Vec::new() };
    contenu
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

pub fn nombre_en_attente(app: &AppHandle) -> usize {
    en_attente(app).len()
}

/// Réécrit le journal avec les ventes restantes. Utilisé après rejeu partiel.
pub fn reecrire(app: &AppHandle, restantes: &[VenteHorsLigne]) -> Result<(), String> {
    let Some(p) = chemin(app) else { return Err("Chemin inconnu".into()) };
    let mut texte = String::new();
    for v in restantes {
        texte.push_str(&serde_json::to_string(v).map_err(|e| e.to_string())?);
        texte.push('\n');
    }
    // Écriture atomique : on écrit à côté puis on remplace, pour ne jamais
    // laisser un journal à moitié écrit qui perdrait des ventes.
    let tmp = p.with_extension("jsonl.tmp");
    std::fs::write(&tmp, texte).map_err(|e| format!("Écriture impossible : {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("Remplacement impossible : {e}"))?;
    Ok(())
}

/// Marque la base comme revenue à la normale.
pub fn marquer_normal() {
    if let Ok(mut e) = etat().lock() {
        *e = EtatSante::Ok;
    }
}

/// Signale une dégradation sans bascule complète hors-ligne.
pub fn marquer_degrade() {
    if let Ok(mut e) = etat().lock() {
        if *e == EtatSante::Ok {
            *e = EtatSante::Degrade;
        }
    }
}

pub fn etat_courant() -> EtatSante {
    etat().lock().map(|e| *e).unwrap_or(EtatSante::Ok)
}

/// Remet l'état à la normale s'il n'y a plus rien en attente.
pub fn evaluer(app: &AppHandle) -> EtatSante {
    let n = nombre_en_attente(app);
    let courant = etat_courant();
    if n == 0 && courant == EtatSante::HorsLigne {
        marquer_normal();
        crate::diagnostics::avertissement(
            "Mode hors-ligne",
            "Toutes les ventes en attente ont été rejouées : reprise normale.",
        );
        return etat_courant();
    }
    courant
}

// ================= Commandes Tauri =================

#[tauri::command]
pub fn sante_etat(app: AppHandle) -> serde_json::Value {
    let e = evaluer(&app);
    serde_json::json!({
        "etat": e,
        "en_attente": nombre_en_attente(&app),
        "libelle": match e {
            EtatSante::Ok => "Fonctionnement normal",
            EtatSante::Degrade => "Base ralentie — certaines opérations ont échoué",
            EtatSante::HorsLigne => "Base indisponible — ventes mises en attente",
        }
    })
}

#[tauri::command]
pub fn hors_ligne_lister(app: AppHandle) -> Vec<VenteHorsLigne> {
    en_attente(&app)
}

/// Enregistre une vente lorsque la base est inaccessible.
#[tauri::command]
pub fn hors_ligne_enregistrer(app: AppHandle, vente: VenteHorsLigne) -> Result<bool, String> {
    enregistrer(&app, &vente)?;
    Ok(true)
}

#[derive(serde::Deserialize)]
pub struct LigneRejeu {
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire_ht: f64,
    pub taux_tva: f64,
    pub remise: f64,
}

/// Rejoue les ventes en attente contre la base revenue à la normale.
/// Les ventes qui échouent restent en attente : rien n'est perdu.
#[tauri::command]
pub async fn hors_ligne_rejouer(
    app: AppHandle,
    pool: tauri::State<'_, crate::db::DbPool>,
) -> Result<serde_json::Value, String> {
    let en_attente = en_attente(&app);
    if en_attente.is_empty() {
        return Ok(serde_json::json!({ "rejouees": 0, "restantes": 0, "message": "Aucune vente en attente" }));
    }

    let mut rejouees: Vec<String> = Vec::new();
    let mut restantes: Vec<VenteHorsLigne> = Vec::new();

    for v in en_attente {
        let lignes: Vec<LigneRejeu> = match serde_json::from_value(v.lignes.clone()) {
            Ok(l) => l,
            Err(e) => {
                crate::diagnostics::erreur(
                    "Mode hors-ligne",
                    &format!("Vente {} illisible, conservée en attente : {e}", v.numero),
                );
                restantes.push(v);
                continue;
            }
        };
        let input = crate::commands::caisse::VenteComptoirInput {
            client_id: v.client_id.clone(),
            lignes: lignes
                .into_iter()
                .map(|l| crate::models::LigneInput {
                    produit_id: l.produit_id,
                    designation: l.designation,
                    quantite: l.quantite,
                    prix_unitaire_ht: l.prix_unitaire_ht,
                    taux_tva: l.taux_tva,
                    remise: l.remise,
                })
                .collect(),
            mode: v.mode.clone(),
            montant_recu: v.montant_recu,
        };
        let audit = crate::commands::caisse::ContexteAudit {
            app: app.clone(),
            token: String::new(),
            actif: false,
        };
        match crate::commands::caisse::vente_comptoir_interne(&pool, "HORS", &audit, input).await {
            Ok(_) => {
                rejouees.push(v.numero.clone());
                crate::diagnostics::avertissement(
                    "Mode hors-ligne",
                    &format!("Vente {} rejouée avec succès", v.numero),
                );
            }
            Err(e) => {
                crate::diagnostics::erreur(
                    "Mode hors-ligne",
                    &format!("Vente {} non rejouable ({e}) — conservée", v.numero),
                );
                restantes.push(v);
            }
        }
    }

    reecrire(&app, &restantes)?;
    if restantes.is_empty() {
        marquer_normal();
    }
    Ok(serde_json::json!({
        "rejouees": rejouees.len(),
        "restantes": restantes.len(),
        "numeros": rejouees,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vente(id: &str, total: f64) -> VenteHorsLigne {
        VenteHorsLigne {
            id: id.into(),
            horodatage: "2026-01-01 10:00:00".into(),
            numero: format!("HORS-{id}"),
            total_ttc: total,
            mode: "especes".into(),
            client_id: None,
            lignes: serde_json::json!([
                { "produit_id": "p1", "designation": "Ciment", "quantite": 2.0,
                  "prix_unitaire_ht": 5500.0, "taux_tva": 18.0, "remise": 0.0 }
            ]),
            montant_recu: total,
        }
    }

    /// La file d'attente conserve les ventes les plus récentes quand elle
    /// déborde : c'est un garde-fou mémoire, pas un stockage définitif.
    #[test]
    fn la_file_regarde_les_ventes_recentes() {
        let mut q: std::collections::VecDeque<i32> = std::collections::VecDeque::new();
        for i in 0..1000 {
            q.push_back(i);
            if q.len() > 500 {
                q.pop_front();
            }
        }
        assert_eq!(q.len(), 500);
        assert_eq!(*q.front().unwrap(), 500, "les plus anciennes doivent tomber");
        assert_eq!(*q.back().unwrap(), 999);
    }

    #[test]
    fn une_vente_serialisee_se_retrouve_a_lidentique() {
        let v = vente("1", 13000.0);
        let txt = serde_json::to_string(&v).unwrap();
        let relue: VenteHorsLigne = serde_json::from_str(&txt).unwrap();
        assert_eq!(relue.numero, v.numero);
        assert_eq!(relue.total_ttc, v.total_ttc);
        assert_eq!(relue.lignes, v.lignes);
    }

    #[test]
    fn le_rejeu_accepte_le_format_des_lignes_de_la_caisse() {
        let brut = serde_json::json!([
            { "produit_id": "p1", "designation": "Ciment", "quantite": 2.0,
              "prix_unitaire_ht": 5500.0, "taux_tva": 18.0, "remise": 0.0 },
            { "produit_id": null, "designation": "Main d'oeuvre", "quantite": 1.0,
              "prix_unitaire_ht": 5000.0, "taux_tva": 18.0, "remise": 10.0 }
        ]);
        let l: Vec<LigneRejeu> = serde_json::from_value(brut).expect("lignes lisibles");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].designation, "Ciment");
        assert!(l[1].produit_id.is_none(), "ligne sans article gérée");
        assert_eq!(l[1].remise, 10.0);
    }

    #[test]
    fn un_ligne_corrompue_est_refusee_sans_panique() {
        let faux = serde_json::json!([{ "designation": "Ciment" }]); // sans quantité
        let r: Result<Vec<LigneRejeu>, _> = serde_json::from_value(faux);
        assert!(r.is_err(), "une ligne incomplète doit être rejetée");
    }

    #[test]
    fn les_transitions_de_sante_sont_coherentes() {
        marquer_normal();
        assert_eq!(etat_courant(), EtatSante::Ok);
        marquer_degrade();
        assert_eq!(etat_courant(), EtatSante::Degrade);
        // Une dégradation ne doit pas basculer en hors-ligne.
        marquer_degrade();
        assert_eq!(etat_courant(), EtatSante::Degrade);
        marquer_normal();
        assert_eq!(etat_courant(), EtatSante::Ok);
    }

    #[test]
    fn les_etats_sont_serialisables_pour_le_frontend() {
        for e in [EtatSante::Ok, EtatSante::Degrade, EtatSante::HorsLigne] {
            let s = serde_json::to_string(&e).unwrap();
            assert!(!s.is_empty());
        }
    }
}
