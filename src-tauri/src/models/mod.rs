use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ---------- User ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub email: String,
    pub role: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub role: String,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            email: u.email,
            role: u.role,
        }
    }
}

// ---------- Tiers ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate)]
pub struct Client {
    pub id: String,
    #[validate(length(min = 1))]
    pub nom: String,
    pub prenom: Option<String>,
    pub entreprise: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub adresse: Option<String>,
    pub ville: Option<String>,
    pub code_postal: Option<String>,
    pub pays: Option<String>,
    pub siret: Option<String>,
    pub tva_intra: Option<String>,
    pub notes: Option<String>,
    #[sqlx(default)]
    pub points: f64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate)]
pub struct Fournisseur {
    pub id: String,
    #[validate(length(min = 1))]
    pub nom: String,
    pub entreprise: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub adresse: Option<String>,
    pub ville: Option<String>,
    pub code_postal: Option<String>,
    pub pays: Option<String>,
    pub siret: Option<String>,
    pub tva_intra: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ---------- Catalogue ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Produit {
    pub id: String,
    pub reference: String,
    pub designation: String,
    pub description: Option<String>,
    pub prix_achat_ht: f64,
    pub prix_vente_ht: f64,
    pub taux_tva: f64,
    pub stock: f64,
    pub stock_alerte: f64,
    pub categorie: Option<String>,
    pub unite: Option<String>,
    pub code_barre: Option<String>,
    pub actif: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MouvementStock {
    pub id: String,
    pub produit_id: String,
    pub r#type: String,
    pub quantite: f64,
    pub stock_avant: f64,
    pub stock_apres: f64,
    pub motif: Option<String>,
    pub document_ref: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub designation: Option<String>,
}

// ---------- Documents ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Devis {
    pub id: String,
    pub numero: String,
    pub client_id: String,
    pub date_emission: String,
    pub date_validite: String,
    pub statut: String,
    pub total_ht: f64,
    pub total_tva: f64,
    pub total_ttc: f64,
    pub remise: f64,
    pub notes: Option<String>,
    pub conditions: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub client_nom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Facture {
    pub id: String,
    pub numero: String,
    pub client_id: String,
    pub devis_id: Option<String>,
    pub date_emission: String,
    pub date_echeance: String,
    pub statut: String,
    pub total_ht: f64,
    pub total_tva: f64,
    pub total_ttc: f64,
    pub montant_paye: f64,
    pub remise: f64,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub client_nom: Option<String>,
    #[sqlx(default)]
    pub telephone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LigneDocument {
    pub id: String,
    pub document_id: String,
    pub document_type: String,
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire_ht: f64,
    pub taux_tva: f64,
    pub remise: f64,
    pub total_ht: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LigneInput {
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire_ht: f64,
    pub taux_tva: f64,
    pub remise: f64,
}

// ---------- Règlements ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reglement {
    pub id: String,
    pub facture_id: String,
    pub montant: f64,
    pub mode: String,
    pub date_reglement: String,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ---------- Compta ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Exercice {
    pub id: String,
    pub libelle: String,
    pub date_debut: String,
    pub date_fin: String,
    pub cloture: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Compte {
    pub id: String,
    pub numero: String,
    pub intitule: String,
    pub classe: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Journal {
    pub id: String,
    pub code: String,
    pub libelle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Ecriture {
    pub id: String,
    pub journal_id: String,
    pub compte_id: String,
    pub date_ecriture: String,
    pub libelle: String,
    pub debit: f64,
    pub credit: f64,
    pub piece_ref: Option<String>,
    pub facture_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub compte_numero: Option<String>,
    #[sqlx(default)]
    pub journal_code: Option<String>,
}

// ---------- Dashboard ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub ca_total: f64,
    pub ca_mois: f64,
    pub factures_impayees: f64,
    pub nb_clients: i64,
    pub nb_produits: i64,
    pub nb_devis_en_cours: i64,
    pub stock_valeur: f64,
    pub alertes_stock: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaMensuel {
    pub mois: String,
    pub ca: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopItem {
    pub nom: String,
    pub total: f64,
    pub quantite: Option<f64>,
}

// ---------- Avoirs ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Avoir {
    pub id: String,
    pub numero: String,
    pub facture_id: Option<String>,
    pub client_id: String,
    pub date_emission: String,
    pub motif: Option<String>,
    pub total_ht: f64,
    pub total_tva: f64,
    pub total_ttc: f64,
    pub statut: String,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub client_nom: Option<String>,
    #[sqlx(default)]
    pub facture_numero: Option<String>,
}

// ---------- Achats ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommandeFournisseur {
    pub id: String,
    pub numero: String,
    pub fournisseur_id: String,
    pub date_commande: String,
    pub date_livraison_prevue: Option<String>,
    pub statut: String,
    pub total_ht: f64,
    pub total_ttc: f64,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub fournisseur_nom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LigneCommande {
    pub id: String,
    pub commande_id: String,
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub quantite_recue: f64,
    pub prix_unitaire_ht: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LigneCommandeInput {
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire_ht: f64,
}

// ---------- Dépenses ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Depense {
    pub id: String,
    pub libelle: String,
    pub categorie: String,
    pub montant: f64,
    pub date_depense: String,
    pub mode: String,
    pub fournisseur_id: Option<String>,
    pub piece_ref: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ---------- Bons de livraison ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BonLivraison {
    pub id: String,
    pub numero: String,
    pub client_id: String,
    pub devis_id: Option<String>,
    pub date_livraison: String,
    pub statut: String,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub client_nom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LigneLivraison {
    pub id: String,
    pub bl_id: String,
    pub produit_id: Option<String>,
    pub designation: String,
    pub quantite: f64,
    pub prix_unitaire_ht: f64,
    pub taux_tva: f64,
}

// ---------- Paie ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Employe {
    pub id: String,
    pub nom: String,
    pub prenom: Option<String>,
    pub poste: Option<String>,
    pub telephone: Option<String>,
    pub salaire_base: f64,
    pub date_embauche: Option<String>,
    pub actif: i64,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Bulletin {
    pub id: String,
    pub numero: String,
    pub employe_id: String,
    pub periode: String,
    pub brut: f64,
    pub cnps: f64,
    pub its: f64,
    pub net: f64,
    pub statut: String,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub employe_nom: Option<String>,
}

// ---------- Clôture de caisse ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClotureZ {
    pub id: String,
    pub date: String,
    pub nb_ventes: i64,
    pub total: f64,
    pub especes: f64,
    pub virement: f64,
    pub cheque: f64,
    pub cb: f64,
    pub mobile: f64,
    pub created_at: Option<DateTime<Utc>>,
}

// ---------- Factures fournisseurs (dettes) ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FactureFournisseur {
    pub id: String,
    pub numero: String,
    pub fournisseur_id: String,
    pub date_emission: String,
    pub date_echeance: String,
    pub statut: String,
    pub total_ht: f64,
    pub total_tva: f64,
    pub total_ttc: f64,
    pub montant_paye: f64,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub fournisseur_nom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReglementFournisseur {
    pub id: String,
    pub facture_id: String,
    pub montant: f64,
    pub mode: String,
    pub date_reglement: String,
    pub reference: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ---------- Fidélité & promos ----------
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Promo {
    pub id: String,
    pub code: String,
    pub r#type: String,
    pub valeur: f64,
    pub date_debut: String,
    pub date_fin: String,
    pub actif: i64,
}

// ---------- Pagination ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paged<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
