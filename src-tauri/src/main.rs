#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod auth;
mod authz;
mod commands;
mod config;
mod db;
mod error;
mod models;
mod services;

use authz::LoginGuard;
use db::DbPool;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").expect("no main window").set_focus();
        }))
        .manage(LoginGuard::default())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                match db::init_db(&handle).await {
                    Ok(pool) => {
                        handle.manage(pool);
                    }
                    Err(e) => eprintln!("Erreur init DB: {}", e),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_login,
            commands::auth_register,
            commands::auth_logout,
            commands::auth_get_current_user,
            commands::auth_refresh_token,
            commands::clients_list,
            commands::clients_get,
            commands::clients_create,
            commands::clients_update,
            commands::clients_delete,
            commands::clients_search,
            commands::fournisseurs_list,
            commands::fournisseurs_get,
            commands::fournisseurs_create,
            commands::fournisseurs_update,
            commands::fournisseurs_delete,
            commands::fournisseurs_search,
            commands::produits_list,
            commands::produits_get,
            commands::produits_create,
            commands::produits_update,
            commands::produits_delete,
            commands::produits_search,
            commands::produits_stock_mouvements,
            commands::produits_ajuster_stock,
            commands::devis_list,
            commands::devis_get,
            commands::devis_create,
            commands::devis_update,
            commands::devis_delete,
            commands::devis_dupliquer,
            commands::devis_changer_statut,
            commands::devis_generer_pdf,
            commands::factures_list,
            commands::factures_get,
            commands::factures_create,
            commands::factures_update,
            commands::factures_delete,
            commands::factures_dupliquer,
            commands::factures_changer_statut,
            commands::factures_generer_pdf,
            commands::factures_from_devis,
            commands::reglements_list,
            commands::reglements_create,
            commands::reglements_lettrage,
            commands::stock_list,
            commands::stock_mouvements,
            commands::stock_inventaire,
            commands::stock_alerte_seuil,
            commands::compta_journal_list,
            commands::compta_journal_create,
            commands::compta_grand_livre,
            commands::compta_balance,
            commands::compta_exercices_list,
            commands::compta_exercices_create,
            commands::compta_exercices_cloturer,
            commands::dashboard_stats,
            commands::dashboard_ca_mensuel,
            commands::dashboard_top_produits,
            commands::dashboard_top_clients,
            commands::dashboard_factures_en_retard,
            commands::dashboard_marges,
            commands::dashboard_depenses_mensuelles,
            commands::settings_get,
            commands::settings_set,
            commands::export_csv,
            commands::import_csv,
            commands::backup_database,
            commands::restore_database,
            commands::prochains_numeros,
            commands::diagnostic_perf,
            // Achats / Avoirs / Dépenses
            commands::avoirs_list,
            commands::avoirs_create,
            commands::avoirs_delete,
            commands::commandes_list,
            commands::commandes_get,
            commands::commandes_create,
            commands::commandes_changer_statut,
            commands::commandes_receptionner,
            commands::commandes_delete,
            commands::depenses_list,
            commands::depenses_create,
            commands::depenses_delete,
            commands::depenses_par_categorie,
            // Admin
            commands::users_list,
            commands::users_changer_role,
            commands::users_delete,
            commands::factures_relancer,
            commands::produits_categories,
            commands::seed_demo_data,
            // Livraison / Caisse / Paie / Notifications
            commands::bls_list,
            commands::bls_get,
            commands::bls_create,
            commands::bls_from_devis,
            commands::bls_changer_statut,
            commands::bls_to_facture,
            commands::bls_delete,
            commands::vente_comptoir,
            commands::cloture_z,
            commands::clotures_list,
            commands::employes_list,
            commands::employes_create,
            commands::employes_update,
            commands::employes_toggle_actif,
            commands::bulletins_list,
            commands::bulletins_create,
            commands::bulletins_changer_statut,
            commands::bulletins_delete,
            commands::masse_salariale,
            commands::notifications_list,
            commands::tout_relancer,
            // Pro : dettes, OHADA, fidélité
            commands::ff_list,
            commands::ff_create,
            commands::ff_delete,
            commands::ff_payer,
            commands::ff_reglements,
            commands::balance_agee,
            commands::ohada_bilan,
            commands::ohada_resultat,
            commands::ohada_tva,
            commands::fidelite_solde,
            commands::fidelite_utiliser,
            commands::promos_list,
            commands::promos_create,
            commands::promos_toggle,
            commands::promos_valider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run()
}
