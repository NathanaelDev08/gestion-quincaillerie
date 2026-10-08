import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { Client, Fournisseur, Produit, Devis, Facture, Paged, DashboardStats, CaMensuel, TopItem, LigneInput, Reglement, MouvementStock, Ecriture, UserPublic, Avoir, CommandeFournisseur, LigneCommande, Depense, BonLivraison, LigneLivraison, Employe, Bulletin, ClotureZ, NotificationItem, FactureFournisseur, ReglementFournisseur, Promo, AuditRow, Depot, DepotStockRow, TransfertInput, Incident } from "../types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw "Tauri indisponible : ouvrez l'application via la fenêtre « Gestion Quincaillerie », pas dans le navigateur (http://localhost:1420).";
  }
  // Le token est injecté sur TOUTES les commandes : la barrière serveur
  // (authz::COMMAND_ROLES) refuse par défaut tout appel non authentifié.
  return tauriInvoke<T>(cmd, { token: token(), ...args });
}

const token = () => localStorage.getItem("token") || "";

export const api = {
  // Auth
  login: (username: string, password: string) =>
    invoke<{ user: UserPublic; token: string; refresh?: string }>("auth_login", { username, password }),
  refreshToken: (refresh: string) =>
    invoke<{ user: UserPublic; token: string; refresh?: string }>("auth_refresh_token", { refresh }),
  logout: (refresh?: string) => invoke<boolean>("auth_logout", { refresh }),
  revoquerSessions: (userId: string) => invoke<boolean>("auth_revoquer_sessions", { token: token(), userId }),
  register: (username: string, password: string, fullName: string, email: string) =>
    invoke<UserPublic>("auth_register", { username, password, fullName, email }),

  // Clients
  clientsList: (page = 1, perPage = 50) => invoke<Paged<Client>>("clients_list", { page, perPage }),
  clientsSearch: (q: string) => invoke<Client[]>("clients_search", { q }),
  clientsCreate: (input: Partial<Client>) => invoke<Client>("clients_create", { input }),
  clientsUpdate: (id: string, input: Partial<Client>) => invoke<Client>("clients_update", { id, input }),
  clientsDelete: (id: string) => invoke<boolean>("clients_delete", { id }),

  // Fournisseurs
  fournisseursList: (page = 1, perPage = 50) => invoke<Paged<Fournisseur>>("fournisseurs_list", { page, perPage }),
  fournisseursSearch: (q: string) => invoke<Fournisseur[]>("fournisseurs_search", { q }),
  fournisseursCreate: (input: Partial<Fournisseur>) => invoke<Fournisseur>("fournisseurs_create", { input }),
  fournisseursUpdate: (id: string, input: Partial<Fournisseur>) => invoke<Fournisseur>("fournisseurs_update", { id, input }),
  fournisseursDelete: (id: string) => invoke<boolean>("fournisseurs_delete", { id }),

  // Produits
  produitsList: (page = 1, perPage = 50) => invoke<Paged<Produit>>("produits_list", { page, perPage }),
  produitsSearch: (q: string) => invoke<Produit[]>("produits_search", { q }),
  produitsCaisse: (q = "", categorie = "", limite = 120) => invoke<Produit[]>("produits_caisse", { token: token(), q, categorie, limite }),
  produitsCreate: (input: Partial<Produit>) => invoke<Produit>("produits_create", { input }),
  produitsUpdate: (id: string, input: Partial<Produit>) => invoke<Produit>("produits_update", { id, input }),
  produitsDelete: (id: string) => invoke<boolean>("produits_delete", { id }),
  ajusterStock: (produitId: string, quantite: number, motif: string) =>
    invoke<Produit>("produits_ajuster_stock", { produitId, quantite, motif }),

  // Devis
  devisList: (page = 1, perPage = 50) => invoke<Paged<Devis>>("devis_list", { page, perPage }),
  devisGet: (id: string) => invoke<[Devis, any[]]>("devis_get", { id }),
  devisCreate: (input: any) => invoke<Devis>("devis_create", { input }),
  devisUpdate: (id: string, input: any) => invoke<Devis>("devis_update", { id, input }),
  devisDelete: (id: string) => invoke<boolean>("devis_delete", { id }),
  devisStatut: (id: string, statut: string) => invoke<Devis>("devis_changer_statut", { id, statut }),
  devisToFacture: (devisId: string) => invoke<Facture>("factures_from_devis", { devisId }),
  devisDupliquer: (id: string) => invoke<Devis>("devis_dupliquer", { id }),
  devisPdf: (id: string) => invoke<string>("devis_generer_pdf", { id }),

  // Factures
  facturesList: (page = 1, perPage = 50) => invoke<Paged<Facture>>("factures_list", { page, perPage }),
  facturesGet: (id: string) => invoke<[Facture, any[], Reglement[]]>("factures_get", { id }),
  facturesCreate: (input: any) => invoke<Facture>("factures_create", { input }),
  facturesUpdate: (id: string, input: any) => invoke<Facture>("factures_update", { id, input }),
  facturesDelete: (id: string) => invoke<boolean>("factures_delete", { id }),
  facturesStatut: (id: string, statut: string) => invoke<Facture>("factures_changer_statut", { id, statut }),
  facturesPdf: (id: string) => invoke<string>("factures_generer_pdf", { id }),
  facturesDupliquer: (id: string) => invoke<Facture>("factures_dupliquer", { id }),

  // Règlements
  reglementsCreate: (input: any) => invoke<Reglement>("reglements_create", { input }),
  reglementsList: (factureId?: string | null) =>
    invoke<Reglement[]>("reglements_list", { factureId: factureId ?? null }),

  // Stock
  stockList: () => invoke<Produit[]>("stock_list"),
  stockAlertes: () => invoke<Produit[]>("stock_alerte_seuil"),
  stockMouvements: () => invoke<MouvementStock[]>("stock_mouvements", { limit: 100 }),
  stockMouvementsProduit: (produitId: string) =>
    invoke<MouvementStock[]>("produits_stock_mouvements", { produitId }),
  stockInventaire: (items: [string, number][]) => invoke<boolean>("stock_inventaire", { items }),

  // Compta
  journalList: () => invoke<Ecriture[]>("compta_journal_list", { journalCode: null, limit: 200 }),
  balance: () => invoke<any[]>("compta_balance"),
  grandLivre: (compteNumero: string) => invoke<Ecriture[]>("compta_grand_livre", { compteNumero }),
  ecritureCreate: (input: { journal_code: string; compte_numero: string; date_ecriture: string; libelle: string; debit: number; credit: number; piece_ref?: string }) =>
    invoke<Ecriture>("compta_journal_create", { input }),
  exercicesList: () => invoke<any[]>("compta_exercices_list"),
  exerciceCreate: (libelle: string, dateDebut: string, dateFin: string) =>
    invoke<any>("compta_exercices_create", { libelle, dateDebut, dateFin }),
  exerciceCloturer: (id: string) => invoke<boolean>("compta_exercices_cloturer", { token: token(), id }),

  // Dashboard
  dashboardStats: () => invoke<DashboardStats>("dashboard_stats"),
  caMensuel: () => invoke<CaMensuel[]>("dashboard_ca_mensuel"),
  topProduits: () => invoke<TopItem[]>("dashboard_top_produits"),
  topClients: () => invoke<TopItem[]>("dashboard_top_clients"),
  facturesRetard: () => invoke<Facture[]>("dashboard_factures_en_retard"),
  marges: () => invoke<{ marge_totale: number; marge_mois: number; taux_marge_mois: number }>("dashboard_marges"),
  depensesMensuelles: () => invoke<CaMensuel[]>("dashboard_depenses_mensuelles"),

  // System
  exportCsv: (entity: string) => invoke<string>("export_csv", { entity }),
  exportVentesCsv: (dateDebut = "1970-01-01", dateFin = "2999-12-31") => invoke<string>("export_ventes_csv", { token: token(), dateDebut, dateFin }),
  exportTvaCsv: (mois: string) => invoke<string>("export_tva_csv", { token: token(), mois }),
  exportBalanceCsv: () => invoke<string>("export_balance_csv", { token: token() }),
  exportJournalCsv: (dateDebut: string, dateFin: string) => invoke<string>("export_journal_csv", { token: token(), dateDebut, dateFin }),
  backup: () => invoke<string>("backup_database", { token: token() }),
  settingsGet: (key: string) => invoke<any>("settings_get", { key }),
  settingsSet: (key: string, value: any) => invoke<boolean>("settings_set", { key, value }),
  importCsv: (entity: string, csvText: string) => invoke<number>("import_csv", { entity, csvText }),
  restoreDb: (backupPath: string) => invoke<boolean>("restore_database", { token: token(), backupPath }),
  prochainsNumeros: () => invoke<{ key: string; label: string; prefix: string; prochain: string }[]>("prochains_numeros"),
  seedDemo: (force: boolean) => invoke<string>("seed_demo_data", { token: token(), force }),
  diagnostic: () => invoke<{ tables: Record<string, number>; mesures: { requete: string; ms: number; lignes: number; ok: boolean }[]; index_perso: number }>("diagnostic_perf"),

  // Avoirs
  avoirsList: () => invoke<Avoir[]>("avoirs_list"),
  avoirsCreate: (input: any) => invoke<Avoir>("avoirs_create", { input }),
  avoirsDelete: (id: string) => invoke<boolean>("avoirs_delete", { id }),

  // Commandes fournisseurs
  commandesList: () => invoke<CommandeFournisseur[]>("commandes_list"),
  commandesGet: (id: string) => invoke<[CommandeFournisseur, LigneCommande[]]>("commandes_get", { id }),
  commandesCreate: (input: any) => invoke<CommandeFournisseur>("commandes_create", { input }),
  commandesStatut: (id: string, statut: string) => invoke<CommandeFournisseur>("commandes_changer_statut", { id, statut }),
  commandesReceptionner: (id: string, recues: [string, number][]) =>
    invoke<CommandeFournisseur>("commandes_receptionner", { id, recues }),
  commandesDelete: (id: string) => invoke<boolean>("commandes_delete", { id }),

  // Dépenses
  depensesList: () => invoke<Depense[]>("depenses_list", { limit: 200 }),
  depensesCreate: (input: any) => invoke<Depense>("depenses_create", { input }),
  depensesDelete: (id: string) => invoke<boolean>("depenses_delete", { id }),
  depensesParCategorie: () => invoke<{ categorie: string; total: number }[]>("depenses_par_categorie"),

  // Admin
  usersList: () => invoke<UserPublic[]>("users_list", { token: token() }),
  usersChangerRole: (id: string, role: string) => invoke<boolean>("users_changer_role", { token: token(), id, role }),
  // Public : sert à détecter une installation neuve avant toute session.
  nbUtilisateurs: () => invoke<number>("auth_nb_utilisateurs"),

  usersChangerMotDePasse: (ancien: string, nouveau: string) => invoke<boolean>("users_changer_mot_de_passe", { token: token(), ancien, nouveau }),
  usersReinitialiserMotDePasse: (id: string) => invoke<string>("users_reinitialiser_mot_de_passe", { token: token(), id }),
  usersMotDePasseParDefaut: (id: string) => invoke<boolean>("users_mot_de_passe_par_defaut", { token: token(), id }),
  usersDelete: (id: string) => invoke<boolean>("users_delete", { token: token(), id }),
  facturesRelancer: (id: string) => invoke<Facture>("factures_relancer", { id }),
  produitsCategories: () => invoke<string[]>("produits_categories"),

  // Bons de livraison
  blsList: () => invoke<BonLivraison[]>("bls_list"),
  blsGet: (id: string) => invoke<[BonLivraison, LigneLivraison[]]>("bls_get", { id }),
  blsCreate: (input: any) => invoke<BonLivraison>("bls_create", { input }),
  blsFromDevis: (devisId: string) => invoke<BonLivraison>("bls_from_devis", { devisId }),
  blsStatut: (id: string, statut: string) => invoke<BonLivraison>("bls_changer_statut", { id, statut }),
  blsToFacture: (id: string) => invoke<Facture>("bls_to_facture", { id }),
  blsDelete: (id: string) => invoke<boolean>("bls_delete", { id }),

  // Caisse
  venteComptoir: (input: any) => invoke<{ facture_id: string; numero: string; total_ttc: number; montant_recu: number; rendu: number }>("vente_comptoir", { token: token(), input }),
  clotureZ: (date: string) => invoke<ClotureZ>("cloture_z", { date }),
  cloturesList: () => invoke<ClotureZ[]>("clotures_list"),

  // Paie
  employesList: (actifsOnly = false) => invoke<Employe[]>("employes_list", { actifsOnly }),
  employesCreate: (input: any) => invoke<Employe>("employes_create", { input }),
  employesUpdate: (id: string, input: any) => invoke<Employe>("employes_update", { id, input }),
  employesToggleActif: (id: string) => invoke<Employe>("employes_toggle_actif", { id }),
  bulletinsList: (periode?: string | null) => invoke<Bulletin[]>("bulletins_list", { periode: periode ?? null }),
  bulletinsCreate: (input: any) => invoke<Bulletin>("bulletins_create", { input }),
  bulletinsStatut: (id: string, statut: string) => invoke<Bulletin>("bulletins_changer_statut", { id, statut }),
  bulletinsDelete: (id: string) => invoke<boolean>("bulletins_delete", { id }),
  masseSalariale: (periode: string) => invoke<{ brut: number; charges: number; net: number; nb: number }>("masse_salariale", { periode }),

  // Notifications
  notificationsList: () => invoke<NotificationItem[]>("notifications_list"),
  toutRelancer: () => invoke<number>("tout_relancer"),

  // Dettes fournisseurs
  ffList: () => invoke<FactureFournisseur[]>("ff_list"),
  ffCreate: (input: any) => invoke<FactureFournisseur>("ff_create", { token: token(), input }),
  ffDelete: (id: string) => invoke<boolean>("ff_delete", { token: token(), id }),
  ffPayer: (input: any) => invoke<ReglementFournisseur>("ff_payer", { input }),
  ffReglements: (factureId: string) => invoke<ReglementFournisseur[]>("ff_reglements", { factureId }),
  balanceAgee: () => invoke<{ tranches: { label: string; total: number }[]; dettes: any[] }>("balance_agee"),

  // OHADA
  ohadaBilan: () => invoke<any>("ohada_bilan"),
  ohadaResultat: () => invoke<any>("ohada_resultat"),
  ohadaTva: () => invoke<any>("ohada_tva"),

  // Fidélité & promos
  fideliteSolde: (clientId: string) => invoke<number>("fidelite_solde", { clientId }),
  fideliteUtiliser: (clientId: string, points: number) => invoke<number>("fidelite_utiliser", { clientId, points }),
  promosList: () => invoke<Promo[]>("promos_list"),
  promosCreate: (input: any) => invoke<Promo>("promos_create", { token: token(), input }),
  promosToggle: (id: string) => invoke<boolean>("promos_toggle", { token: token(), id }),
  promosValider: (code: string, totalTtc: number) => invoke<{ code: string; type: string; valeur: number; remise: number }>("promos_valider", { code, totalTtc }),

  // Journal d'audit
  auditList: (action = "", limit = 200) => invoke<AuditRow[]>("audit_list", { token: token(), action, limit }),
  auditActions: () => invoke<string[]>("audit_actions"),
  auditPurge: () => invoke<boolean>("audit_purge", { token: token() }),

  // Mode hors-ligne
  santeEtat: () => invoke<{ etat: string; en_attente: number; libelle: string }>("sante_etat", { token: token() }),
  horsLigneLister: () => invoke<any[]>("hors_ligne_lister", { token: token() }),
  horsLigneEnregistrer: (vente: any) => invoke<boolean>("hors_ligne_enregistrer", { token: token(), vente }),
  horsLigneRejouer: () => invoke<{ rejouees: number; restantes: number; numeros: string[] }>("hors_ligne_rejouer", { token: token() }),

  // Journal d'incidents (robustesse)
  incidentsLister: (niveau = "") => invoke<Incident[]>("incidents_lister", { niveau }),
  incidentsVider: () => invoke<boolean>("incidents_vider"),
  incidentsExporter: () => invoke<string>("incidents_exporter"),

  // Multi-dépôts
  depotsList: () => invoke<Depot[]>("depots_list"),
  depotsCreate: (input: { nom: string; adresse?: string }) => invoke<Depot>("depots_create", { input }),
  depotsValeur: () => invoke<{ id: string; nom: string; valeur: number; nb: number }[]>("depots_valeur"),
  depotStock: (depotId: string, q = "") => invoke<DepotStockRow[]>("depot_stock", { depotId, q }),
  depotTransferer: (input: TransfertInput) => invoke<boolean>("depot_transferer", { token: token(), input }),
  depotMouvements: (depotId: string) => invoke<any[]>("depot_mouvements", { depotId }),
};
