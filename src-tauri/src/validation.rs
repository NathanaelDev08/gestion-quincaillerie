//! Validation des données métier — exécutée côté serveur avant toute écriture.
//!
//! Règle : le frontend ne fait jamais confiance. Un prix négatif, une quantité
//! à zéro ou un stock fantaisiste sont rejetés ici, avant d'atteindre la base.

/// Montant maximum accepté (garde-fou contre les erreurs de saisie).
pub const MONTANT_MAX: f64 = 1_000_000_000.0;
/// Quantité maximale sur une ligne de document.
pub const QUANTITE_MAX: f64 = 1_000_000.0;

pub fn valider_montant(valeur: f64, champ: &str) -> Result<f64, String> {
    if !valeur.is_finite() {
        return Err(format!("{champ} : valeur invalide"));
    }
    if valeur < 0.0 {
        return Err(format!("{champ} : ne peut pas être négatif ({valeur})"));
    }
    if valeur > MONTANT_MAX {
        return Err(format!("{champ} : montant trop élevé (max {MONTANT_MAX})"));
    }
    Ok((valeur * 100.0).round() / 100.0)
}

pub fn valider_quantite(valeur: f64, champ: &str) -> Result<f64, String> {
    if !valeur.is_finite() {
        return Err(format!("{champ} : valeur invalide"));
    }
    if valeur <= 0.0 {
        return Err(format!("{champ} : doit être supérieur à 0 ({valeur})"));
    }
    if valeur > QUANTITE_MAX {
        return Err(format!("{champ} : quantité trop élevée (max {QUANTITE_MAX})"));
    }
    Ok((valeur * 1000.0).round() / 1000.0)
}

pub fn valider_tva(taux: f64) -> Result<f64, String> {
    if !taux.is_finite() || taux < 0.0 || taux > 100.0 {
        return Err(format!("TVA invalide : {taux} % (attendu entre 0 et 100)"));
    }
    Ok(taux)
}

pub fn valider_remise(remise: f64) -> Result<f64, String> {
    if !remise.is_finite() || remise < 0.0 || remise > 100.0 {
        return Err(format!("Remise invalide : {remise} % (attendu entre 0 et 100)"));
    }
    Ok(remise)
}

/// Contrôle une ligne de document (vente, devis, facture, avoir…).
pub fn valider_ligne(l: &crate::models::LigneInput) -> Result<(f64, f64, f64), String> {
    let q = valider_quantite(l.quantite, "Quantité")?;
    let pu = valider_montant(l.prix_unitaire_ht, "Prix unitaire")?;
    valider_tva(l.taux_tva)?;
    let remise = valider_remise(l.remise)?;
    if l.designation.trim().is_empty() {
        return Err("Désignation obligatoire sur chaque ligne".to_string());
    }
    let total = (q * pu * (1.0 - remise / 100.0) * 100.0).round() / 100.0;
    if total > MONTANT_MAX {
        return Err(format!(
            "Ligne « {}» : total trop élevé",
            l.designation.chars().take(30).collect::<String>()
        ));
    }
    Ok((q, pu, total))
}

/// Valide l'ensemble d'un document : au moins une ligne, totaux cohérents.
pub fn valider_document(lignes: &[crate::models::LigneInput]) -> Result<(f64, f64, f64), String> {
    if lignes.is_empty() {
        return Err("Document vide : ajoute au moins une ligne".to_string());
    }
    let mut ht = 0.0;
    let mut tva = 0.0;
    for (i, l) in lignes.iter().enumerate() {
        let (_q, _pu, total) = valider_ligne(l).map_err(|e| format!("Ligne {} : {e}", i + 1))?;
        ht += total;
        tva += total * l.taux_tva / 100.0;
    }
    let ht = (ht * 100.0).round() / 100.0;
    let tva = (tva * 100.0).round() / 100.0;
    if ht > MONTANT_MAX {
        return Err("Total du document trop élevé".to_string());
    }
    Ok((ht, tva, ht + tva))
}

/// Vérifie qu'un stock ne devient pas négatif après une sortie.
pub fn verifier_stock(stock_avant: f64, quantite_sortie: f64, designation: &str) -> Result<(), String> {
    if stock_avant + 1e-9 < quantite_sortie {
        return Err(format!(
            "Stock insuffisant pour {designation} : {stock_avant} disponible(s)"
        ));
    }
    Ok(())
}

/// Nettoie une chaîne libre (trim + suppression des caractères de contrôle).
pub fn nettoyer(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reproduit l'agrégation d'une clôture Z : c'est le moment où une
    /// divergence entre le total et le détail des modes se verrait.
    fn agrege_cloture(modes: &[(&str, f64)]) -> (f64, f64, f64, f64, f64, f64) {
        let (mut especes, mut virement, mut cheque, mut cb, mut mobile) =
            (0.0, 0.0, 0.0, 0.0, 0.0);
        for (mode, s) in modes {
            match *mode {
                "especes" => especes += s,
                "virement" => virement += s,
                "cheque" => cheque += s,
                "cb" => cb += s,
                _ => mobile += s,
            }
        }
        let r2 = |v: f64| (v * 100.0).round() / 100.0;
        let (especes, virement, cheque, cb, mobile) =
            (r2(especes), r2(virement), r2(cheque), r2(cb), r2(mobile));
        let total = r2(especes + virement + cheque + cb + mobile);
        (total, especes, virement, cheque, cb, mobile)
    }

    #[test]
    fn cloture_z_total_egale_somme_des_modes() {
        let (total, e, v, c, b, m) = agrege_cloture(&[
            ("especes", 10_000.0),
            ("mobile", 4_500.55),
            ("cb", 12_000.0),
            ("virement", 33_333.33),
        ]);
        // Comparaison au centime : la somme de flottants arrondis peut
        // présenter un écart de l'ordre du bitnoise, jamais visible à l'affichage.
        let somme = e + v + c + b + m;
        assert!(
            (somme - total).abs() < 0.01,
            "le détail ({somme}) doit reconstituer le total ({total})"
        );
        assert!((total - 59_833.88).abs() < 0.01);
    }

    #[test]
    fn cloture_z_mode_inconnu_non_perdu() {
        // Un mode non prévu ne doit jamais faire disparaître de la recette.
        let (total, especes, _v, _c, _b, mobile) =
            agrege_cloture(&[("especes", 1_000.0), ("orange_money", 2_500.0)]);
        assert_eq!(especes, 1_000.0);
        assert_eq!(mobile, 2_500.0);
        assert_eq!(total, 3_500.0);
    }

    #[test]
    fn cloture_z_vide_ne_produit_pas_de_nan() {
        let (total, e, v, c, b, m) = agrege_cloture(&[]);
        assert_eq!((total, e, v, c, b, m), (0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        assert!(total.is_finite());
    }

    fn ligne(q: f64, pu: f64, tva: f64, remise: f64) -> crate::models::LigneInput {
        crate::models::LigneInput {
            produit_id: Some("p1".into()),
            designation: "Vis 4x40".into(),
            quantite: q,
            prix_unitaire_ht: pu,
            taux_tva: tva,
            remise,
        }
    }

    #[test]
    fn rejette_quantite_negative_ou_nulle() {
        assert!(valider_quantite(-1.0, "Quantité").is_err());
        assert!(valider_quantite(0.0, "Quantité").is_err());
        assert!(valider_quantite(1.0, "Quantité").is_ok());
    }

    #[test]
    fn rejette_prix_negatif_et_nan() {
        assert!(valider_montant(-100.0, "Prix").is_err());
        assert!(valider_montant(f64::NAN, "Prix").is_err());
        assert!(valider_montant(f64::INFINITY, "Prix").is_err());
        assert_eq!(valider_montant(1500.555, "Prix").unwrap(), 1500.56);
    }

    #[test]
    fn rejette_tva_et_remise_hors_bornes() {
        assert!(valider_tva(-1.0).is_err());
        assert!(valider_tva(120.0).is_err());
        assert!(valider_remise(101.0).is_err());
        assert_eq!(valider_tva(18.0).unwrap(), 18.0);
    }

    #[test]
    fn rejette_document_vide() {
        assert!(valider_document(&[]).is_err());
    }

    #[test]
    fn rejette_designation_vide() {
        let mut l = ligne(1.0, 100.0, 18.0, 0.0);
        l.designation = "   ".into();
        assert!(valider_document(&[l]).is_err());
    }

    #[test]
    fn calcule_ht_tva_ttc_correctement() {
        let (ht, tva, ttc) =
            valider_document(&[ligne(2.0, 1000.0, 18.0, 0.0), ligne(1.0, 500.0, 18.0, 0.0)]).unwrap();
        assert_eq!(ht, 2500.0);
        assert_eq!(tva, 450.0);
        assert_eq!(ttc, 2950.0);
    }

    #[test]
    fn applique_la_remise_avant_la_tva() {
        // 1000 HT, remise 10 % => 900 HT, TVA 18 % => 1062 TTC
        let (ht, _tva, ttc) = valider_document(&[ligne(1.0, 1000.0, 18.0, 10.0)]).unwrap();
        assert_eq!(ht, 900.0);
        assert_eq!(ttc, 1062.0);
    }

    #[test]
    fn refuse_stock_insuffisant() {
        assert!(verifier_stock(5.0, 3.0, "Ciment").is_ok());
        assert!(verifier_stock(5.0, 5.0, "Ciment").is_ok());
        assert!(verifier_stock(5.0, 6.0, "Ciment").is_err());
        assert!(verifier_stock(0.0, 0.001, "Ciment").is_err());
    }

    #[test]
    fn tolere_les_erreurs_de_flottant() {
        // 0.1 + 0.2 style : on ne doit pas rejeter à tort
        assert!(verifier_stock(3.0, 3.0 - 1e-9, "X").is_ok());
    }

    #[test]
    fn nettoie_les_champes_txt() {
        assert_eq!(nettoyer("  Bonjour\u{0}\n  ", 100), "Bonjour");
        assert_eq!(nettoyer("abcdef", 3), "abc");
    }

    // ---------------------------------------------------------------------
    // Cas dégradés : l'app doit survivre à des données sales ou hostiles.
    // ---------------------------------------------------------------------

    #[test]
    fn accepte_les_entrees_de_borne_sans_panique() {
        // Quantité fractionnaire (quincaillerie : 2,5 m de câble, 0,5 kg)
        assert_eq!(valider_quantite(2.5, "Quantité").unwrap(), 2.5);
        // Montant à 3 décimales : arrondi comptable à 2
        assert_eq!(valider_montant(0.005, "Prix").unwrap(), 0.01);
        // Bornes exactes
        assert!(valider_montant(0.0, "Prix").is_ok());
        assert!(valider_quantite(1.0, "Qté").is_ok());
        assert!(valider_tva(0.0).is_ok());
        assert!(valider_tva(100.0).is_ok());
        assert!(valider_remise(100.0).is_ok(), "remise totale autorisée");
    }

    #[test]
    fn refuse_les_quantites_astronomiques() {
        assert!(valider_quantite(1e12, "Qté").is_err());
        assert!(valider_montant(1e15, "Montant").is_err());
    }

    #[test]
    fn rejette_un_montant_negatif_meme_apres_arrondi() {
        // -0.001 arrondit à -0.0 : le rejet doit rester based sur la valeur brute.
        assert!(valider_montant(-0.001, "Montant").is_err());
    }

    #[test]
    fn stock_autorise_zero_mais_pas_de_sous_vente_autorisee() {
        assert!(verifier_stock(0.0, 0.0, "Rupture").is_ok());
        assert!(verifier_stock(0.0, 1.0, "Rupture").is_err());
    }

    #[test]
    fn document_avec_ligne_invalide_signale_son_numero() {
        let l1 = ligne(1.0, 100.0, 18.0, 0.0);
        let l2 = ligne(-5.0, 100.0, 18.0, 0.0);
        let err = valider_document(&[l1, l2]).expect_err("doit échouer");
        assert!(err.contains("Ligne 2"), "l'erreur doit pointer la ligne : {err}");
    }

    #[test]
    fn remise_de_100_pct_ne_produit_pas_de_total_negatif() {
        let (ht, _tva, ttc) = valider_document(&[ligne(3.0, 1000.0, 18.0, 100.0)]).unwrap();
        assert_eq!(ht, 0.0);
        assert_eq!(ttc, 0.0);
        assert!(ttc >= 0.0);
    }

    #[test]
    fn total_toujours_positif_ou_nul() {
        // Balayage : aucune combinaison ne doit produire de total négatif.
        for q in [0.1, 1.0, 3.7, 99.0] {
            for pu in [0.01, 1.0, 5500.0] {
                for tva in [0.0, 18.0] {
                    for r in [0.0, 50.0, 100.0] {
                        let (ht, _t, ttc) =
                            valider_document(&[ligne(q, pu, tva, r)]).expect("valide");
                        assert!(ht >= 0.0 && ttc >= 0.0, "q={q} pu={pu} r={r}");
                    }
                }
            }
        }
    }
}
