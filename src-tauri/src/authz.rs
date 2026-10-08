use crate::auth::verify_token;
use crate::config::AppConfig;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Secret JWT persistant : généré aléatoirement à la première utilisation,
/// stocké dans settings.json (plus de secret codé en dur).
pub fn jwt_secret(app: &AppHandle) -> String {
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store("settings.json") {
        let _ = store.reload();
        if let Some(v) = store.get("jwt_secret").and_then(|v| v.as_str().map(|s| s.to_string())) {
            if v.len() >= 32 {
                return v;
            }
        }
        let fresh = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).replace('-', "");
        store.set("jwt_secret", serde_json::Value::String(fresh.clone()));
        let _ = store.save();
        return fresh;
    }
    AppConfig::default().jwt_secret
}

/// Vérifie le token et exige le rôle admin. Retourne l'id utilisateur.
pub fn require_admin(app: &AppHandle, token: &str) -> Result<String, String> {
    let secret = jwt_secret(app);
    let claims = verify_token(token, &secret).map_err(|e| e.to_string())?;
    if claims.role != "admin" {
        return Err("Action réservée aux administrateurs".to_string());
    }
    Ok(claims.sub)
}

/// Commandes accessibles sans session : connexion, création de compte,
/// vérification du nombre de comptes. Sans cette liste explicite, la barrière
/// refuserait la connexion elle-même (aucun jeton n'existe encore au moment
/// de se connecter).
fn est_publique(command: &str) -> bool {
    matches!(
        command,
        "auth_login" | "auth_register" | "auth_nb_utilisateurs" | "auth_logout"
    )
}

/// Préférences d'affichage (devise, TVA, préfixes de numérotation).
///
/// Accessible sans session : l'écran de connexion les charge pour afficher les
/// bonnes unités avant même l'authentification. Ces valeurs ne sont pas
/// sensibles — les secrets et les données sont dans d'autres commandes.
fn est_preferece_publique(command: &str) -> bool {
    matches!(command, "settings_get" | "prochains_numeros")
}

/// Vérifie le token et exige l'un des rôles autorisés. Retourne (user_id, rôle).
pub fn require_roles(
    app: &AppHandle,
    token: &str,
    allowed: &[&str],
) -> Result<(String, String), String> {
    if token.trim().is_empty() {
        return Err("Session expirée : reconnecte-toi.".to_string());
    }
    let secret = jwt_secret(app);
    let claims =
        verify_token(token, &secret).map_err(|_| "Session invalide ou expirée.".to_string())?;
    if !allowed.contains(&claims.role.as_str()) {
        return Err(format!(
            "Ton rôle ({}) ne permet pas cette action",
            claims.role
        ));
    }
    Ok((claims.sub, claims.role))
}

/// Contrôle d'accès effectif d'une commande. Utilisé par la barrière IPC.
pub fn authorize(
    app: &AppHandle,
    command: &str,
    token: &str,
) -> Result<(), String> {
    let roles = command_roles(command).ok_or_else(|| {
        format!("Commande non autorisée : {command}")
    })?;
    // Une commande publique s'exécute sans session. Elle reste déclarée dans
    // `command_roles` pour rester visible dans la matrice des droits.
    if est_publique(command) {
        return Ok(());
    }
    if autorise_preferences(command, token) {
        return Ok(());
    }
    require_roles(app, token, roles).map(|_| ())
}

/// Les préférences d'affichage sont lisibles sans session : l'écran de
/// connexion en a besoin (devise, TVA, préfixes). Aucune donnée sensible
/// n'est exposée : ce sont des préférences d'affichage, pas des secrets.
fn autorise_preferences(command: &str, token: &str) -> bool {
    est_preferece_publique(command) && token.trim().is_empty()
}

// =====================================================================
// BARRIÈRE D'AUTORISATION GLOBALE (deny-by-default)
// ---------------------------------------------------------------------
// Chaque commande exposée au frontend DOIT être déclarée ici avec les rôles
// autorisés. Une commande absente de cette table est REFUSÉE : impossible
// d'oublier une protection en ajoutant une nouvelle commande.
// =====================================================================

const ADMIN: &[&str] = &["admin"];
const CAISSE: &[&str] = &["admin", "commercial", "user"];
const VENTE: &[&str] = &["admin", "commercial", "user", "comptable"];
const ACHAT: &[&str] = &["admin", "user", "comptable"];
const TOUS: &[&str] = &["admin", "commercial", "user", "comptable"];
const LECTURE: &[&str] = &["admin", "commercial", "user", "comptable"];

/// Rôles autorisés par commande. Une commande absente = refusée.
pub fn command_roles(command: &str) -> Option<&'static [&'static str]> {
    Some(match command {
        // Auth (public : le token n'existe pas encore)
        "auth_login" | "auth_register" | "auth_logout" | "auth_nb_utilisateurs" => TOUS,

        // Tableau de bord & lectures générales
        "dashboard_stats" | "dashboard_ca_mensuel" | "dashboard_top_produits"
        | "dashboard_top_clients" | "dashboard_factures_en_retard" | "dashboard_marges"
        | "dashboard_depenses_mensuelles" | "notifications_list" | "diagnostic_perf"
        | "auth_get_current_user" | "auth_refresh_token" | "prochains_numeros"
        | "products_categories" | "produits_categories" => LECTURE,

        // Catalogue : lecture ouverte à tous (le comptable valorise le stock),
        // écriture réservée à l'équipe vente.
        "produits_list" | "produits_get" | "produits_search" | "stock_list"
        | "stock_mouvements" | "stock_alerte_seuil" | "produits_stock_mouvements"
        | "depot_stock" | "depot_mouvements" | "depots_list" | "depots_valeur"
        | "produits_caisse" => LECTURE,
        "produits_create" | "produits_update" | "produits_delete" | "stock_inventaire"
        | "produits_ajuster_stock" | "depots_create" | "depot_transferer" => CAISSE,

        // Tiers
        "clients_list" | "clients_get" | "clients_search" | "fournisseurs_list"
        | "fournisseurs_get" | "fournisseurs_search" => TOUS,
        "clients_create" | "clients_update" | "clients_delete" | "fournisseurs_create"
        | "fournisseurs_update" | "fournisseurs_delete" => CAISSE,

        // Ventes
        "caisse_vente" | "vente_comptoir" | "devis_list" | "devis_get" | "devis_create"
        | "devis_update" | "devis_delete" | "devis_dupliquer" | "devis_changer_statut"
        | "devis_generer_pdf" | "factures_list" | "factures_get" | "factures_create"
        | "factures_update" | "factures_delete" | "factures_dupliquer"
        | "factures_changer_statut" | "factures_generer_pdf" | "factures_from_devis"
        | "reglements_list" | "reglements_create" | "reglements_lettrage"
        | "bls_list" | "bls_get" | "bls_create" | "bls_from_devis" | "bls_changer_statut"
        | "bls_to_facture" | "bls_delete" | "avoirs_list" | "avoirs_create"
        | "avoirs_delete" | "factures_relancer" | "tout_relancer" | "fidelite_solde"
        | "fidelite_utiliser" | "promos_list" | "promos_valider" | "cloture_z"
        | "clotures_list" => VENTE,
        "promos_create" | "promos_toggle" => ADMIN,

        // Achats
        "commandes_list" | "commandes_get" | "commandes_create" | "commandes_changer_statut"
        | "commandes_receptionner" | "commandes_delete" | "ff_list" | "ff_create"
        | "ff_delete" | "ff_payer" | "ff_reglements" | "balance_agee" => ACHAT,

        // Dépenses & paie (encadrement)
        "depenses_list" | "depenses_create" | "depenses_delete" | "depenses_par_categorie"
        | "employes_list" | "employes_create" | "employes_update" | "employes_toggle_actif"
        | "bulletins_list" | "bulletins_create" | "bulletins_changer_statut"
        | "bulletins_delete" | "masse_salariale" => ADMIN,

        // Comptabilité
        "compta_journal_list" | "compta_grand_livre" | "compta_balance"
        | "compta_exercices_list" | "ohada_bilan" | "ohada_resultat" | "ohada_tva" => {
            &["admin", "comptable"]
        }
        "compta_journal_create" | "compta_exercices_create" | "compta_exercices_cloturer" => ADMIN,

        // Système / données
        "users_list" | "users_changer_role" | "users_delete" | "audit_list"
        | "audit_actions" | "audit_purge" | "backup_database" | "restore_database"
        | "settings_get" | "import_csv" | "seed_demo_data" | "settings_set" | "incidents_lister"
        | "incidents_vider" | "incidents_exporter" | "users_changer_mot_de_passe"
        | "users_reinitialiser_mot_de_passe" | "users_mot_de_passe_par_defaut"
        | "auth_revoquer_sessions" => ADMIN,

        // Hors-ligne : la caisse doit pouvoir vendre même si la base tombe.
        "sante_etat" | "hors_ligne_lister" | "hors_ligne_enregistrer" => CAISSE,
        "hors_ligne_rejouer" => &["admin", "comptable"],
        "export_csv" => &["admin", "comptable", "commercial"],
        // Exports comptables : données financières, donc pas au vendeur.
        "export_ventes_csv" | "export_tva_csv" | "export_balance_csv"
        | "export_journal_csv" => &["admin", "comptable"],

        // Toute autre commande inconnue
        _ => return None,
    })
}

/// Garde anti-bruteforce : 5 échecs => verrouillage 15 minutes par identifiant.
pub struct LoginGuard {
    fails: Mutex<HashMap<String, (u32, Instant)>>,
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self { fails: Mutex::new(HashMap::new()) }
    }
}

const MAX_FAILS: u32 = 5;
const LOCK_TIME: Duration = Duration::from_secs(15 * 60);

impl LoginGuard {
    pub fn check(&self, login: &str) -> Result<(), String> {
        let key = login.trim().to_lowercase();
        let map = self.fails.lock().map_err(|_| "Erreur interne".to_string())?;
        if let Some((n, t)) = map.get(&key) {
            if *n >= MAX_FAILS && t.elapsed() < LOCK_TIME {
                let reste = (LOCK_TIME - t.elapsed()).as_secs() / 60 + 1;
                return Err(format!("Compte verrouillé après {MAX_FAILS} échecs. Réessayez dans ~{reste} min."));
            }
        }
        Ok(())
    }

    pub fn success(&self, login: &str) {
        if let Ok(mut map) = self.fails.lock() {
            map.remove(&login.trim().to_lowercase());
        }
    }

    pub fn failure(&self, login: &str) {
        if let Ok(mut map) = self.fails.lock() {
            let e = map.entry(login.trim().to_lowercase()).or_insert((0, Instant::now()));
            e.0 += 1;
            e.1 = Instant::now();
        }
    }
}

// =====================================================================
// TESTS DE SÉCURITÉ
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commande_inconnue_est_refusee() {
        // Deny-by-default : toute commande non déclarée doit être refusée.
        assert!(command_roles("commande_que_je_nai_pas_creee").is_none());
        assert!(command_roles("").is_none());
        assert!(command_roles("drop_table").is_none());
    }

    #[test]
    fn operations_critiques_reservees_a_admin() {
        for cmd in [
            "users_list",
            "users_delete",
            "users_changer_role",
            "audit_purge",
            "restore_database",
            "import_csv",
            "seed_demo_data",
            "settings_set",
            "compta_journal_create",
            "bulletins_create",
            "depenses_create",
            "promos_create",
        ] {
            let roles = command_roles(cmd).unwrap_or_else(|| panic!("{cmd} non déclaré"));
            assert!(
                roles.contains(&"admin"),
                "{cmd} doit rester accessible à l'admin"
            );
            assert!(
                !roles.contains(&"comptable") || cmd == "export_csv",
                "{cmd} ne doit pas être ouvert au comptable"
            );
            assert!(
                !roles.contains(&"commercial") && !roles.contains(&"user"),
                "{cmd} ne doit pas être ouvert à la vente"
            );
        }
    }

    #[test]
    fn lecture_ouverte_a_tous_les_roles() {
        for cmd in [
            "dashboard_stats",
            "produits_list",
            "clients_list",
            "notifications_list",
        ] {
            let roles = command_roles(cmd).expect("déclarée");
            for r in ["admin", "commercial", "user", "comptable"] {
                assert!(roles.contains(&r), "{cmd} doit être lisible par {r}");
            }
        }
    }

    #[test]
    fn auth_est_publique_mais_le_reste_est_ferme() {
        for c in ["auth_login", "auth_register", "auth_nb_utilisateurs", "auth_logout"] {
            assert!(est_publique(c), "{c} doit être accessible sans session");
            assert!(command_roles(c).is_some(), "{c} doit rester déclaré");
        }
        // Aucune commande métier ne doit être publique.
        for c in [
            "vente_comptoir", "produits_delete", "users_list", "restore_database",
            "dashboard_stats", "compta_journal_create", "backup_database",
        ] {
            assert!(!est_publique(c), "{c} ne doit jamais être publique");
        }
        let caisse = command_roles("vente_comptoir").expect("déclarée");
        assert!(caisse.contains(&"admin") && caisse.contains(&"commercial"));
    }

    /// Régression : la barrière bloquait `auth_login` faute de jeton,
    /// rendant l'application inutilisable. Seules les commandes publiques
    /// doivent passer sans session.
    #[test]
    fn la_connexion_nest_pas_bloquee_par_la_barriere() {
        // On vérifie la logique sans AppHandle : `est_publique` est la seule
        // porte d'entrée avant l'exigence de jeton.
        assert!(est_publique("auth_login"));
        assert!(est_publique("auth_nb_utilisateurs"));
        assert!(!est_publique("dashboard_stats"));
    }

    #[test]
    fn garde_bruteforce_bloque_apres_plusieurs_echecs() {
        let g = LoginGuard::default();
        // 4 échecs : encore autorisé
        for _ in 0..4 {
            g.check("admin").expect("encore ouvert");
            g.failure("admin");
        }
        g.check("admin").expect("4e échec : encore ouvert");
        // 5e échec : verrouillé
        g.failure("admin");
        let err = g.check("admin").expect_err("doit être verrouillé");
        assert!(err.contains("verrouillé"), "message inattendu : {err}");
        // Un autre compte n'est pas impacté
        g.check("caissier").expect("autre compte libre");
    }
}
