export interface UserPublic { id: string; username: string; full_name: string; email: string; role: string; }
export interface Client { id: string; nom: string; prenom?: string; entreprise?: string; email?: string; telephone?: string; adresse?: string; ville?: string; code_postal?: string; pays?: string; siret?: string; tva_intra?: string; notes?: string; }
export interface Fournisseur extends Client {}
export interface Produit { id: string; reference: string; designation: string; description?: string; prix_achat_ht: number; prix_vente_ht: number; taux_tva: number; stock: number; stock_alerte: number; categorie?: string; unite?: string; code_barre?: string; actif: number; }
export interface LigneInput { produit_id?: string; designation: string; quantite: number; prix_unitaire_ht: number; taux_tva: number; remise: number; }
export interface LigneDocument extends LigneInput { id: string; document_id: string; document_type: string; total_ht: number; }
export interface Devis { id: string; numero: string; client_id: string; date_emission: string; date_validite: string; statut: string; total_ht: number; total_tva: number; total_ttc: number; remise: number; notes?: string; client_nom?: string; }
export interface Facture extends Devis { devis_id?: string; date_echeance: string; montant_paye: number; derniere_relance?: string; }
export interface Reglement { id: string; facture_id: string; montant: number; mode: string; date_reglement: string; reference?: string; notes?: string; }
export interface MouvementStock { id: string; produit_id: string; type: string; quantite: number; stock_avant: number; stock_apres: number; motif?: string; document_ref?: string; created_at?: string; designation?: string; }
export interface Paged<T> { data: T[]; total: number; page: number; per_page: number; }
export interface DashboardStats { ca_total: number; ca_mois: number; factures_impayees: number; nb_clients: number; nb_produits: number; nb_devis_en_cours: number; stock_valeur: number; alertes_stock: number; }
export interface CaMensuel { mois: string; ca: number; }
export interface TopItem { nom: string; total: number; quantite?: number; }
export interface Ecriture { id: string; journal_id: string; compte_id: string; date_ecriture: string; libelle: string; debit: number; credit: number; piece_ref?: string; compte_numero?: string; journal_code?: string; }
export interface Avoir { id: string; numero: string; facture_id?: string; client_id: string; date_emission: string; motif?: string; total_ht: number; total_tva: number; total_ttc: number; statut: string; client_nom?: string; facture_numero?: string; }
export interface CommandeFournisseur { id: string; numero: string; fournisseur_id: string; date_commande: string; date_livraison_prevue?: string; statut: string; total_ht: number; total_ttc: number; notes?: string; fournisseur_nom?: string; }
export interface LigneCommande { id: string; commande_id: string; produit_id?: string; designation: string; quantite: number; quantite_recue: number; prix_unitaire_ht: number; }
export interface Depense { id: string; libelle: string; categorie: string; montant: number; date_depense: string; mode: string; fournisseur_id?: string; piece_ref?: string; notes?: string; }
export interface BonLivraison { id: string; numero: string; client_id: string; devis_id?: string; date_livraison: string; statut: string; notes?: string; client_nom?: string; }
export interface LigneLivraison { id: string; bl_id: string; produit_id?: string; designation: string; quantite: number; prix_unitaire_ht: number; taux_tva: number; }
export interface Employe { id: string; nom: string; prenom?: string; poste?: string; telephone?: string; salaire_base: number; date_embauche?: string; actif: number; }
export interface Bulletin { id: string; numero: string; employe_id: string; periode: string; brut: number; cnps: number; its: number; net: number; statut: string; employe_nom?: string; }
export interface ClotureZ { id: string; date: string; nb_ventes: number; total: number; especes: number; virement: number; cheque: number; cb: number; mobile: number; }
export interface NotificationItem { categorie: string; titre: string; detail: string; lien: string; niveau: string; }
export interface FactureFournisseur { id: string; numero: string; fournisseur_id: string; date_emission: string; date_echeance: string; statut: string; total_ht: number; total_tva: number; total_ttc: number; montant_paye: number; notes?: string; fournisseur_nom?: string; }
export interface ReglementFournisseur { id: string; facture_id: string; montant: number; mode: string; date_reglement: string; reference?: string; }
export interface Promo { id: string; code: string; type: string; valeur: number; date_debut: string; date_fin: string; actif: number; }
export interface AuditRow { id: string; utilisateur: string; role: string; action: string; entite: string; entite_id: string; detail?: string; montant?: number; created_at: string; }
export interface Depot { id: string; nom: string; adresse?: string; actif: number; }
export interface Incident { id: number; horodatage: string; niveau: string; contexte: string; message: string; }
export interface DepotStockRow { depot_id: string; depot_nom: string; produit_id: string; designation: string; reference: string; categorie?: string; quantite: number; unite?: string; }
export interface TransfertInput { produit_id: string; depot_origine: string; depot_destination: string; quantite: number; }
