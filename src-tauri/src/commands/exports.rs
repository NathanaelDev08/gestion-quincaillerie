//! Exports comptables et légaux.
//!
//! Un comptable ne travaille pas avec des données affichées à l'écran : il
//! veut des fichiers exploitables dans son logiciel, et l'administration
//! fiscale peut demander des formatages précis. Ces exports sont donc
//! séparés, et couverts par des tests.
//!
//! Format principal : CSV point-virgule, séparateur décimal virgule — le
//! format attendu par les tableurs francophones et la plupart des logiciels
//! de gestion.

use crate::db::DbPool;
use std::fmt::Write as _;
use tauri::State;

/// Échappe une valeur pour un export CSV : séparateur, guillemets, fin de ligne.
/// Sans cela, un client nommé « Dupont; Marie » décale toutes les colonnes.
pub fn echapper_csv(v: &str) -> String {
    let nettoye = v.replace('\r', " ").replace('\n', " ");
    if nettoye.contains(';') || nettoye.contains('"') {
        format!("\"{}\"", nettoye.replace('"', "\"\""))
    } else {
        nettoye
    }
}

/// Assemble une ligne CSV. Chaque champ est échappé, donc un point-virgule
/// à l'intérieur d'une valeur ne crée pas de colonne supplémentaire.
fn ligne_csv(champs: &[String]) -> String {
    champs
        .iter()
        .map(|c| echapper_csv(c))
        .collect::<Vec<_>>()
        .join(";")
}

/// Nombre de colonnes réelles d'une ligne CSV, en tenant compte des
/// guillemets. Utilisé par les tests pour prouver qu'une valeur exotique
/// ne décale pas les colonnes.
#[cfg(test)]
fn compter_colonnes(ligne: &str) -> usize {
    let mut n = 1;
    let mut dans_guillemets = false;
    let mut c = ligne.chars().peekable();
    while let Some(ch) = c.next() {
        match ch {
            '"' => {
                if dans_guillemets && c.peek() == Some(&'"') {
                    c.next(); // guillemet échappé
                } else {
                    dans_guillemets = !dans_guillemets;
                }
            }
            ';' if !dans_guillemets => n += 1,
            _ => {}
        }
    }
    n
}

fn nombre_fr(n: f64) -> String {
    // Format comptable francophone : virgule décimale, pas de séparateur de milliers.
    format!("{:.2}", n).replace('.', ",")
}

fn date_fr(d: &str) -> String {
    if d.len() >= 10 {
        format!("{}/{}/{}", &d[8..10], &d[5..7], &d[0..4])
    } else {
        d.to_string()
    }
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct LigneVente {
    pub numero: String,
    pub date: String,
    pub client: String,
    pub article: String,
    pub quantite: f64,
    pub pu_ht: f64,
    pub taux_tva: f64,
    pub total_ht: f64,
    pub total_tva: f64,
    pub total_ttc: f64,
    pub mode: String,
}

/// Journal des ventes détaillé (une ligne par article vendu).
#[tauri::command]
pub async fn export_ventes_csv(
    pool: State<'_, DbPool>,
    date_debut: Option<String>,
    date_fin: Option<String>,
) -> Result<String, String> {
    let debut = date_debut.unwrap_or_else(|| "1970-01-01".into());
    let fin = date_fin.unwrap_or_else(|| "2999-12-31".into());
    if debut > fin {
        return Err("La date de début doit précéder la date de fin".to_string());
    }

    let lignes: Vec<LigneVente> = sqlx::query_as(
        "SELECT f.numero, f.date_emission as date, COALESCE(c.nom,'') as client,
                l.designation as article, l.quantite, l.prix_unitaire_ht as pu_ht,
                l.taux_tva, l.total_ht, f.total_tva, f.total_ttc,
                COALESCE((SELECT r.mode FROM reglements r WHERE r.facture_id=f.id LIMIT 1),'') as mode
         FROM lignes_document l
         JOIN factures f ON f.id = l.document_id
         LEFT JOIN clients c ON c.id = f.client_id
         WHERE l.document_type='facture' AND f.statut != 'annulee'
           AND f.date_emission BETWEEN ? AND ?
         ORDER BY f.date_emission, f.numero",
    )
    .bind(&debut)
    .bind(&fin)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = String::from(
        "Piece;Date;Client;Article;Quantite;PU_HT;Taux_TVA;Total_HT;Total_TVA;Total_TTC;Reglement\n",
    );
    for l in &lignes {
        let _ = writeln!(
            out,
            "{}",
            ligne_csv(&[
                l.numero.clone(),
                date_fr(&l.date),
                l.client.clone(),
                l.article.clone(),
                nombre_fr(l.quantite),
                nombre_fr(l.pu_ht),
                nombre_fr(l.taux_tva),
                nombre_fr(l.total_ht),
                nombre_fr(l.total_tva),
                nombre_fr(l.total_ttc),
                l.mode.clone(),
            ])
        );
    }
    Ok(out)
}

/// Synthèse par période : le format que veut un comptable pour la TVA.
#[tauri::command]
pub async fn export_tva_csv(
    pool: State<'_, DbPool>,
    mois: String,
) -> Result<String, String> {
    let m = mois.trim();
    let format_valide = m.len() == 7
        && m.as_bytes().get(4) == Some(&b'-')
        && m[..4].bytes().all(|c| c.is_ascii_digit())
        && m[5..].bytes().all(|c| c.is_ascii_digit());
    if !format_valide {
        return Err("Format attendu : AAAA-MM (ex. 2026-10)".to_string());
    }
    let debut = format!("{mois}-01");
    let fin = format!("{mois}-31");

    let tva: (Option<f64>, Option<f64>, Option<f64>) = sqlx::query_as(
        "SELECT SUM(total_ht), SUM(total_tva), SUM(total_ttc) FROM factures
         WHERE statut != 'annulee' AND date_emission BETWEEN ? AND ?",
    )
    .bind(&debut)
    .bind(&fin)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let achats: (Option<f64>, Option<f64>, Option<f64>) = sqlx::query_as(
        "SELECT SUM(total_ht), SUM(total_tva), SUM(total_ttc) FROM factures_fournisseurs
         WHERE date_emission BETWEEN ? AND ?",
    )
    .bind(&debut)
    .bind(&fin)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let ht = tva.0.unwrap_or(0.0);
    let collectee = tva.1.unwrap_or(0.0);
    let deductible = achats.1.unwrap_or(0.0);

    let mut out = String::from("Rubrique;Montant\n");
    let _ = writeln!(out, "{}", ligne_csv(&["Période".into(), mois.clone()]));
    let _ = writeln!(out, "{}", ligne_csv(&["Ventes HT".into(), nombre_fr(ht)]));
    let _ = writeln!(out, "{}", ligne_csv(&["TVA collectée".into(), nombre_fr(collectee)]));
    let _ = writeln!(out, "{}", ligne_csv(&["Achats HT".into(), nombre_fr(achats.0.unwrap_or(0.0))]));
    let _ = writeln!(out, "{}", ligne_csv(&["TVA déductible".into(), nombre_fr(deductible)]));
    let _ = writeln!(out, "{}", ligne_csv(&["TVA due".into(), nombre_fr(collectee - deductible)]));
    let _ = writeln!(out, "{}", ligne_csv(&["Ventes TTC".into(), nombre_fr(tva.2.unwrap_or(0.0))]));
    Ok(out)
}

/// Balance générale des comptes, au format attendu par un comptable.
#[tauri::command]
pub async fn export_balance_csv(pool: State<'_, DbPool>) -> Result<String, String> {
    let lignes: Vec<(String, String, String, Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT c.numero, c.intitule, c.type,
                SUM(e.debit) AS debit, SUM(e.credit) AS credit
         FROM comptes c LEFT JOIN ecritures e ON e.compte_id = c.id
         GROUP BY c.numero, c.intitule, c.type
         HAVING SUM(e.debit) IS NOT NULL OR SUM(e.credit) IS NOT NULL
         ORDER BY c.numero",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = String::from("Compte;Intitule;Type;Debit;Credit;Solde\n");
    let mut td = 0.0;
    let mut tc = 0.0;
    for (numero, intitule, type_, debit, credit) in &lignes {
        let d = debit.unwrap_or(0.0);
        let c = credit.unwrap_or(0.0);
        td += d;
        tc += c;
        let _ = writeln!(
            out,
            "{}",
            ligne_csv(&[
                numero.clone(),
                intitule.clone(),
                type_.clone(),
                nombre_fr(d),
                nombre_fr(c),
                nombre_fr(d - c),
            ])
        );
    }
    let _ = writeln!(out, "{}", ligne_csv(&["TOTAL".into(), "".into(), "".into(), nombre_fr(td), nombre_fr(tc), nombre_fr(td - tc)]));
    Ok(out)
}

/// Écritures du journal, période donnée.
#[tauri::command]
pub async fn export_journal_csv(
    pool: State<'_, DbPool>,
    date_debut: String,
    date_fin: String,
) -> Result<String, String> {
    if date_debut > date_fin {
        return Err("La date de début doit précéder la date de fin".to_string());
    }
    let lignes: Vec<(String, String, String, String, f64, f64, Option<String>)> = sqlx::query_as(
        "SELECT e.date_ecriture, COALESCE(j.code,''), COALESCE(c.numero,''),
                e.libelle, e.debit, e.credit, e.piece_ref
         FROM ecritures e
         LEFT JOIN journaux j ON j.id = e.journal_id
         LEFT JOIN comptes c ON c.id = e.compte_id
         WHERE e.date_ecriture BETWEEN ? AND ?
         ORDER BY e.date_ecriture, e.created_at",
    )
    .bind(&date_debut)
    .bind(&date_fin)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = String::from("Date;Journal;Compte;Libelle;Debit;Credit;Piece\n");
    for (date, journal, compte, libelle, debit, credit, piece) in &lignes {
        let _ = writeln!(
            out,
            "{}",
            ligne_csv(&[
                date_fr(date), journal.clone(), compte.clone(), libelle.clone(),
                nombre_fr(*debit), nombre_fr(*credit), piece.clone().unwrap_or_default(),
            ])
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echappe_les_separateurs_et_guillemets() {
        assert_eq!(echapper_csv("simple"), "simple");
        assert_eq!(echapper_csv("Dupont; Marie"), "\"Dupont; Marie\"");
        assert_eq!(echapper_csv("dit \"oui\""), "\"dit \"\"oui\"\"\"");
    }

    #[test]
    fn neutralise_les_sauts_de_ligne() {
        // Un saut de ligne casserait le CSV en deux lignes.
        assert_eq!(echapper_csv("a\nb"), "a b");
        assert_eq!(echapper_csv("a\r\nb"), "a  b");
    }

    #[test]
    fn formate_les_nombres_a_la_francaise() {
        assert_eq!(nombre_fr(1234.5), "1234,50");
        assert_eq!(nombre_fr(0.0), "0,00");
        assert_eq!(nombre_fr(-12.3), "-12,30");
    }

    #[test]
    fn formate_les_dates_francaises() {
        assert_eq!(date_fr("2026-10-08"), "08/10/2026");
        assert_eq!(date_fr("2026-10-08 14:30:00"), "08/10/2026");
        // Une date inattendue est renvoyée telle quelle plutôt que d'échouer.
        assert_eq!(date_fr("inconnu"), "inconnu");
    }

    #[test]
    fn genere_une_entete_par_colonne() {
        let l = ligne_csv(&["A".into(), "B".into(), "C".into()]);
        assert_eq!(l, "A;B;C");
    }

    #[test]
    fn un_client_avec_point_virgule_ne_decale_pas_les_colonnes() {
        let l = ligne_csv(&["F1".into(), "Dupont; Marie".into(), "1000".into()]);
        // `split(';')` compterait 4 à tort : le point-virgule protégé par
        // des guillemets ne sépare pas deux colonnes.
        assert_eq!(compter_colonnes(&l), 3, "colonnes : {l}");
        assert!(l.contains("\"Dupont; Marie\""));
    }

    #[test]
    fn plusieurs_valeurs_exotiques_garder_le_nombre_de_colonnes() {
        let l = ligne_csv(&[
            "F1".into(),
            "Société \"A;B\" SARL".into(),
            "Café; Thé".into(),
            "ligne\ncoupée".into(),
            "1000".into(),
        ]);
        assert_eq!(compter_colonnes(&l), 5, "colonnes : {l}");
    }
}
