use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use tauri::{AppHandle, State};
use uuid::Uuid;

fn day(offset: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::days(offset))
        .format("%Y-%m-%d")
        .to_string()
}
fn uid() -> String {
    Uuid::new_v4().to_string()
}

#[tauri::command]
pub async fn seed_demo_data(pool: State<'_, DbPool>, app: AppHandle, token: String, force: bool) -> Result<String, String> {
    crate::authz::require_admin(&app, &token)?;
    seed_inner(&pool, force).await.map_err(|e| e.to_string())
}

async fn seed_inner(pool: &DbPool, force: bool) -> AppResult<String> {
    let (nb,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients")
        .fetch_one(pool)
        .await?;
    if nb > 0 && !force {
        return Err(AppError::Validation(
            "Des données existent déjà. Activez « Forcer » pour tout réinitialiser.".into(),
        ));
    }
    if force {
        for t in [
            "reglements",
            "lignes_document",
            "ecritures",
            "lignes_avoir",
            "avoirs",
            "factures",
            "devis",
            "lignes_commande",
            "commandes_fournisseurs",
            "mouvements_stock",
            "depenses",
            "produits",
            "clients",
            "fournisseurs",
            "sequences",
        ] {
            sqlx::query(&format!("DELETE FROM {t}"))
                .execute(pool)
                .await?;
        }
    }

    // ---------- Fournisseurs ----------
    let fournisseurs = vec![
        ("SONACO Distribution", "contact@sonaco.ci", "+225 27 20 31 44 55", "Abidjan", "Treichville"),
        ("Ivoire Import SARL", "info@ivoireimport.ci", "+225 27 20 22 10 09", "Abidjan", "Port-Bouët"),
        ("ETS Koffi & Fils", "koffi.fils@example.ci", "+225 07 47 88 12 30", "Bouaké", "Commerce"),
        ("Abidjan Gros Market", "contact@agm.ci", "+225 27 20 55 66 77", "Abidjan", "Adjamé"),
    ];
    let mut f_ids = vec![];
    for (nom, email, tel, ville, adresse) in fournisseurs {
        let id = uid();
        sqlx::query("INSERT INTO fournisseurs (id,nom,email,telephone,ville,adresse,pays) VALUES (?,?,?,?,?,?,?)")
            .bind(&id).bind(nom).bind(email).bind(tel).bind(ville).bind(adresse).bind("Côte d'Ivoire")
            .execute(pool).await?;
        f_ids.push(id);
    }

    // ---------- Clients ----------
    let clients = vec![
        ("Kouassi", "Jean-Baptiste", "", "kouassi.jb@example.ci", "+225 07 08 11 22 33", "Cocody", "Abidjan"),
        ("Diallo", "Awa", "", "awa.diallo@example.ci", "+225 05 44 55 66 77", "Plateau", "Abidjan"),
        ("", "", "SARL Ivoire Tech", "contact@ivoiretech.ci", "+225 27 20 33 44 55", "Treichville", "Abidjan"),
        ("Yao", "Kouamé", "", "yao.kouame@example.ci", "+225 07 09 12 34 56", "Commerce", "Bouaké"),
        ("Ndiaye", "Fatou", "", "fatou.ndiaye@example.ci", "+225 05 66 77 88 99", "Marcory", "Abidjan"),
        ("", "", "ETS Sankara", "ets.sankara@example.ci", "+225 07 11 22 33 44", "Centre-ville", "Yamoussoukro"),
        ("Koné", "Mariam", "", "mariam.kone@example.ci", "+225 07 55 66 77 88", "Yopougon", "Abidjan"),
        ("", "", "SARL Lagune Distribution", "contact@lagunedist.ci", "+225 27 20 77 88 99", "Port-Bouët", "Abidjan"),
    ];
    let mut c_ids = vec![];
    for (nom, prenom, ent, email, tel, ville, adresse) in clients {
        let id = uid();
        sqlx::query("INSERT INTO clients (id,nom,prenom,entreprise,email,telephone,ville,adresse,pays) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(nom).bind(prenom).bind(ent).bind(email).bind(tel).bind(ville).bind(adresse).bind("Côte d'Ivoire")
            .execute(pool).await?;
        c_ids.push(id);
    }

    // ---------- Produits (ref, designation, pa, pv, tva, stock, alerte, categorie, unite) ----------
    let produits = vec![
        ("RIZ-25KG", "Riz parfumé 25 kg", 11000.0, 13500.0, 18.0, 120.0, 20.0, "Alimentaire", "sac"),
        ("HUILE-5L", "Huile végétale 5 L", 4800.0, 5900.0, 18.0, 80.0, 15.0, "Alimentaire", "pcs"),
        ("SUCRE-50", "Sucre en poudre 50 kg", 28000.0, 32500.0, 18.0, 40.0, 10.0, "Alimentaire", "sac"),
        ("SAVON-240", "Savon 240 g (carton de 50)", 9000.0, 11500.0, 18.0, 60.0, 12.0, "Hygiène", "carton"),
        ("CIMENT-50", "Ciment CPJ 50 kg", 5200.0, 6100.0, 18.0, 200.0, 50.0, "BTP", "sac"),
        ("FER-12", "Fer à béton 12 mm (12 m)", 6500.0, 7800.0, 18.0, 150.0, 30.0, "BTP", "barre"),
        ("TOLE-3M", "Tôle bac alu 3 m", 9500.0, 11500.0, 18.0, 70.0, 15.0, "BTP", "feuille"),
        ("PATES-500", "Pâtes 500 g (lot de 20)", 6000.0, 7400.0, 18.0, 90.0, 20.0, "Alimentaire", "lot"),
        ("LAIT-1KG", "Lait en poudre 1 kg", 3200.0, 3900.0, 18.0, 100.0, 25.0, "Alimentaire", "pcs"),
        ("EAU-6X15", "Eau minérale 1,5 L (pack de 6)", 1500.0, 1900.0, 18.0, 300.0, 60.0, "Boissons", "pack"),
        ("JUS-24X33", "Jus d'ananas 33 cl (pack de 24)", 5500.0, 6800.0, 18.0, 110.0, 20.0, "Boissons", "pack"),
        ("FARINE-50", "Farine de blé 50 kg", 22000.0, 25500.0, 18.0, 45.0, 10.0, "Alimentaire", "sac"),
    ];
    let mut p_ids = vec![];
    for (i, (reference, designation, pa, pv, tva, stock, alerte, cat, unite)) in produits.iter().enumerate() {
        let id = uid();
        sqlx::query("INSERT INTO produits (id,reference,designation,prix_achat_ht,prix_vente_ht,taux_tva,stock,stock_alerte,categorie,unite,actif) VALUES (?,?,?,?,?,?,?,?,?,?,1)")
            .bind(&id).bind(reference).bind(designation).bind(pa).bind(pv).bind(tva)
            .bind(stock).bind(alerte).bind(cat).bind(unite)
            .execute(pool).await?;
        let mid = uid();
        sqlx::query("INSERT INTO mouvements_stock (id,produit_id,type,quantite,stock_avant,stock_apres,motif) VALUES (?,?,?,?,?,?,?)")
            .bind(&mid).bind(&id).bind("entree").bind(stock).bind(0.0).bind(stock).bind("Stock initial (démo)")
            .execute(pool).await?;
        p_ids.push(id);
        let _ = i;
    }

    // Helper lignes : (produit_idx, designation, qte, pu_ht, tva)
    async fn add_lignes(
        pool: &DbPool,
        doc_id: &str,
        dtype: &str,
        lignes: &[(usize, &str, f64, f64, f64)],
        p_ids: &[String],
    ) -> Result<(f64, f64), sqlx::Error> {
        let mut ht = 0.0; let mut tva = 0.0;
        for (pi, des, qte, pu, ttva) in lignes {
            let lt = qte * pu;
            ht += lt; tva += lt * ttva / 100.0;
            sqlx::query("INSERT INTO lignes_document (id,document_id,document_type,produit_id,designation,quantite,prix_unitaire_ht,taux_tva,remise,total_ht) VALUES (?,?,?,?,?,?,?,?,?,?)")
                .bind(uid()).bind(doc_id).bind(dtype).bind(&p_ids[*pi]).bind(des)
                .bind(qte).bind(pu).bind(ttva).bind(0.0).bind(lt)
                .execute(pool).await?;
        }
        Ok((ht, tva))
    }

    let annee = chrono::Utc::now().format("%Y").to_string();

    // ---------- Devis ----------
    let devis_defs: Vec<(&str, usize, String, String, &str, Vec<(usize, &str, f64, f64, f64)>)> = vec![
        ("brouillon", 0, day(-2), day(28), "Remise hangar", vec![(4, "Ciment CPJ 50 kg", 100.0, 6100.0, 18.0), (5, "Fer à béton 12 mm", 60.0, 7800.0, 18.0)]),
        ("envoye", 2, day(-6), day(24), "Stock boutique", vec![(0, "Riz parfumé 25 kg", 50.0, 13500.0, 18.0), (1, "Huile végétale 5 L", 40.0, 5900.0, 18.0), (7, "Pâtes 500 g x20", 30.0, 7400.0, 18.0)]),
        ("accepte", 5, day(-12), day(18), "Chantier Yamoussoukro", vec![(6, "Tôle bac alu 3 m", 80.0, 11500.0, 18.0), (4, "Ciment CPJ 50 kg", 150.0, 6100.0, 18.0)]),
        ("expire", 1, day(-40), day(-10), "", vec![(9, "Eau minérale pack", 100.0, 1900.0, 18.0)]),
    ];
    let mut dev_compteur = 0;
    for (statut, ci, dem, dval, notes, lignes) in &devis_defs {
        dev_compteur += 1;
        let id = uid();
        let numero = format!("DEV-{annee}-{dev_compteur:04}");
        let (ht, tva) = add_lignes(pool, &id, "devis", lignes, &p_ids).await?;
        sqlx::query("INSERT INTO devis (id,numero,client_id,date_emission,date_validite,statut,total_ht,total_tva,total_ttc,remise,notes) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(&numero).bind(&c_ids[*ci]).bind(dem).bind(dval).bind(statut)
            .bind(ht).bind(tva).bind(ht + tva).bind(0.0).bind(notes)
            .execute(pool).await?;
    }

    // ---------- Comptes pour écritures ----------
    let jvte: (String,) = sqlx::query_as("SELECT id FROM journaux WHERE code='VTE'").fetch_one(pool).await?;
    let c411: (String,) = sqlx::query_as("SELECT id FROM comptes WHERE numero='411000'").fetch_one(pool).await?;
    let c707: (String,) = sqlx::query_as("SELECT id FROM comptes WHERE numero='707000'").fetch_one(pool).await?;
    let ctva: (String,) = sqlx::query_as("SELECT id FROM comptes WHERE numero='445710'").fetch_one(pool).await?;

    // ---------- Factures (client_idx, dem_offset, ech_offset, statut, lignes, reglements) ----------
    let fact_defs: Vec<(usize, i64, i64, &str, Vec<(usize, &str, f64, f64, f64)>, Vec<(f64, &str, i64)>)> = vec![
        (2, -20, 10, "payee",
            vec![(0, "Riz parfumé 25 kg", 80.0, 13500.0, 18.0), (2, "Sucre 50 kg", 20.0, 32500.0, 18.0)],
            vec![(0.0, "virement", -5)]), // 0.0 = solde total (calculé)
        (0, -15, 15, "partiellement_payee",
            vec![(4, "Ciment CPJ 50 kg", 120.0, 6100.0, 18.0)],
            vec![(300000.0, "especes", -10)]),
        (5, -50, -20, "emise",
            vec![(6, "Tôle bac alu 3 m", 60.0, 11500.0, 18.0)],
            vec![]),
        (7, -8, 22, "emise",
            vec![(10, "Jus d'ananas pack", 50.0, 6800.0, 18.0), (9, "Eau minérale pack", 120.0, 1900.0, 18.0)],
            vec![]),
        (1, -35, -5, "payee",
            vec![(8, "Lait en poudre 1 kg", 60.0, 3900.0, 18.0)],
            vec![(0.0, "cheque", -30), (0.0, "virement", -12)]),
    ];
    let mut fac_compteur = 0;
    for (ci, dem_off, ech_off, statut, lignes, regs) in &fact_defs {
        fac_compteur += 1;
        let id = uid();
        let numero = format!("FAC-{annee}-{fac_compteur:04}");
        let dem = day(*dem_off); let ech = day(*ech_off);
        let (ht, tva) = add_lignes(pool, &id, "facture", lignes, &p_ids).await?;
        let ttc = ht + tva;
        sqlx::query("INSERT INTO factures (id,numero,client_id,date_emission,date_echeance,statut,total_ht,total_tva,total_ttc,montant_paye,remise) VALUES (?,?,?,?,?,?,?,?,?,?,0)")
            .bind(&id).bind(&numero).bind(&c_ids[*ci]).bind(&dem).bind(&ech).bind(statut)
            .bind(ht).bind(tva).bind(ttc).bind(0.0)
            .execute(pool).await?;
        // Décrément stock
        for (pi, _, qte, _, _) in lignes {
            sqlx::query("UPDATE produits SET stock = stock - ? WHERE id=?").bind(qte).bind(&p_ids[*pi])
                .execute(pool).await?;
        }
        // Règlements (montant 0.0 = partage équitable du solde restant)
        let mut paye = 0.0;
        let zeros = regs.iter().filter(|r| r.0 == 0.0).count().max(1) as f64;
        let mut zero_done = 0i64;
        for (m, mode, off) in regs {
            let montant = if *m == 0.0 {
                zero_done += 1;
                if zero_done as f64 >= zeros {
                    ttc - paye
                } else {
                    ((ttc - paye) / (zeros - zero_done as f64 + 1.0)).round()
                }
            } else {
                *m
            };
            sqlx::query("INSERT INTO reglements (id,facture_id,montant,mode,date_reglement) VALUES (?,?,?,?,?)")
                .bind(uid()).bind(&id).bind(montant).bind(mode).bind(day(*off))
                .execute(pool).await?;
            paye += montant;
        }
        let final_statut = if paye >= ttc - 0.01 { "payee" } else if paye > 0.0 { "partiellement_payee" } else { *statut };
        sqlx::query("UPDATE factures SET montant_paye=?, statut=? WHERE id=?").bind(paye).bind(final_statut).bind(&id)
            .execute(pool).await?;
        // Écritures VTE : 411 débit TTC / 707 crédit HT / 445710 crédit TVA
        for (cid, debit, credit) in [(&c411.0, ttc, 0.0), (&c707.0, 0.0, ht), (&ctva.0, 0.0, tva)] {
            sqlx::query("INSERT INTO ecritures (id,journal_id,compte_id,date_ecriture,libelle,debit,credit,piece_ref,facture_id) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(uid()).bind(&jvte.0).bind(cid).bind(&dem).bind(format!("Facture {numero}")).bind(debit).bind(credit).bind(&numero).bind(&id)
                .execute(pool).await?;
        }
    }

    // ---------- Dépenses ----------
    let depenses = vec![
        ("Loyer bureau Plateau", "loyer", 150000.0, -3, "virement", "LOY-01"),
        ("Salaires du mois", "salaires", 450000.0, -5, "virement", "SAL-01"),
        ("Carburant livraison", "transport", 60000.0, -6, "especes", ""),
        ("Fournitures bureau", "fournitures", 25000.0, -9, "especes", ""),
        ("Facture CIE électricité", "energie", 45000.0, -12, "virement", "CIE-11"),
        ("Abonnement internet", "communication", 30000.0, -15, "virement", "NET-11"),
        ("Impôt BIC acompte", "impots", 120000.0, -20, "virement", "DGI-03"),
    ];
    for (lib, cat, montant, off, mode, piece) in depenses {
        sqlx::query("INSERT INTO depenses (id,libelle,categorie,montant,date_depense,mode,piece_ref) VALUES (?,?,?,?,?,?,?)")
            .bind(uid()).bind(lib).bind(cat).bind(montant).bind(day(off)).bind(mode).bind(piece)
            .execute(pool).await?;
    }

    // ---------- Exercice + séquences ----------
    sqlx::query("INSERT OR IGNORE INTO exercices (id,libelle,date_debut,date_fin,cloture) VALUES ('exo-demo',?,?,?,0)")
        .bind(format!("Exercice {annee}")).bind(format!("{annee}-01-01")).bind(format!("{annee}-12-31"))
        .execute(pool).await?;
    for (prefix, n) in [("DEV", dev_compteur), ("FAC", fac_compteur), ("BC", 0), ("AV", 0)] {
        let code = format!("{prefix}-{annee}");
        sqlx::query("INSERT INTO sequences (code, annee, compteur) VALUES (?,?,?) ON CONFLICT(code) DO UPDATE SET compteur=excluded.compteur")
            .bind(&code).bind(&annee).bind(n as i64)
            .execute(pool).await?;
    }

    Ok(format!(
        "Démo chargée : {} clients, {} fournisseurs, {} produits, {} devis, {} factures, {} dépenses.",
        c_ids.len(), f_ids.len(), p_ids.len(), devis_defs.len(), fact_defs.len(), 7
    ))
}
