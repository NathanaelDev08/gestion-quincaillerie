//! Chiffrement des sauvegardes (AES-256-GCM).
//!
//! La base contient des données commerciales, des coordonnées clients et les
//! salaires via la page Paie. Une sauvegarde en clair dans `Documents/` est
//! lisible par n'importe quel utilisateur du poste et se retrouve sur une clé
//! USB ou un cloud sans protection. Chaque sauvegarde est donc chiffrée, et
//! l'empreinte SHA256 est remplacée par une authentification du fichier.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use rand_core::RngCore;

const VERSION: u8 = 1;

/// Chiffre `contenu` avec la clé fournie.
/// Format : 1 octet de version + 12 octets de nonce + ciphertext + 16 octets de tag.
pub fn chiffrer(contenu: &[u8], cle: &[u8; 32]) -> Result<Vec<u8>, String> {
    if cle.len() != 32 {
        return Err("Clé de chiffrement invalide".to_string());
    }
    let cipher = Aes256Gcm::new_from_slice(cle).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, contenu)
        .map_err(|_| "Échec du chiffrement".to_string())?;
    let mut sortie = Vec::with_capacity(1 + 12 + ciphertext.len());
    sortie.push(VERSION);
    sortie.extend_from_slice(&nonce_bytes);
    sortie.extend_from_slice(&ciphertext);
    Ok(sortie)
}

/// Déchiffre un fichier produit par `chiffrer`.
/// Un fichier altéré ou malveillant est rejeté : AES-GCM vérifie l'authentification,
/// on ne peut donc pas substituer une sauvegarde sans connaître la clé.
pub fn dechiffrer(donnees: &[u8], cle: &[u8; 32]) -> Result<Vec<u8>, String> {
    if donnees.len() < 1 + 12 + 16 {
        return Err("Fichier de sauvegarde tronqué ou invalide".to_string());
    }
    if donnees[0] != VERSION {
        return Err(format!(
            "Version de sauvegarde non prise en charge ({})",
            donnees[0]
        ));
    }
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&donnees[1..13]);
    let cipher = Aes256Gcm::new_from_slice(cle).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, &donnees[13..])
        .map_err(|_| "Fichier de sauvegarde altéré ou clé incorrecte".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle() -> [u8; 32] {
        let mut c = [0u8; 32];
        for (i, b) in c.iter_mut().enumerate() {
            *b = i as u8;
        }
        c
    }

    #[test]
    fn aller_retour_restaure_le_contenu() {
        let original = b"SQLite format 3\0donnees de quincaillerie".to_vec();
        let chiffre = chiffrer(&original, &cle()).expect("chiffrement");
        assert_ne!(chiffre, original, "le contenu ne doit pas rester en clair");
        let relu = dechiffrer(&chiffre, &cle()).expect("déchiffrement");
        assert_eq!(relu, original);
    }

    #[test]
    fn le_contenu_chiffre_ne_laisse_pas_deviner_les_donnees() {
        let s = b"salaire de Jean: 350000".to_vec();
        let c = chiffrer(&s, &cle()).unwrap();
        let texte = String::from_utf8_lossy(&c);
        assert!(!texte.contains("salaire"));
        assert!(!texte.contains("Jean"));
        assert!(!texte.contains("350000"));
    }

    #[test]
    fn chaque_chiffrement_utilise_un_nonce_different() {
        let s = b"meme contenu".to_vec();
        let a = chiffrer(&s, &cle()).unwrap();
        let b = chiffrer(&s, &cle()).unwrap();
        assert_ne!(a, b, "deux chiffrements du même contenu doivent différer");
        assert_eq!(dechiffrer(&a, &cle()).unwrap(), dechiffrer(&b, &cle()).unwrap());
    }

    #[test]
    fn une_cle_mauvaise_est_rejetee() {
        let c = chiffrer(b"secret", &cle()).unwrap();
        let mut mauvaise = cle();
        mauvaise[0] ^= 0xFF;
        let r = dechiffrer(&c, &mauvaise);
        assert!(r.is_err(), "une mauvaise clé doit être refusée");
    }

    #[test]
    fn un_fichier_altere_est_detecte() {
        let mut c = chiffrer(b"donnees importantes", &cle()).unwrap();
        let dernier = c.len() - 1;
        c[dernier] ^= 0x01; // on flippe un bit du tag
        assert!(
            dechiffrer(&c, &cle()).is_err(),
            "une altération doit être détectée, pas déchiffrée en données fausses"
        );
    }

    #[test]
    fn un_contenu_ajoute_est_refuse() {
        let mut c = chiffrer(b"donnees", &cle()).unwrap();
        c.push(0x42);
        // L'ajout modifie le tag : doit être rejeté.
        assert!(dechiffrer(&c, &cle()).is_err());
    }

    #[test]
    fn un_fichier_trop_court_ou_vide_est_refuse_sans_panique() {
        assert!(dechiffrer(&[], &cle()).is_err());
        assert!(dechiffrer(&[1], &cle()).is_err());
        assert!(dechiffrer(&[1, 2, 3], &cle()).is_err());
        let quasi_vide = vec![0u8; 28];
        assert!(dechiffrer(&quasi_vide, &cle()).is_err());
    }

    #[test]
    fn une_version_inconnue_est_refusee() {
        let mut c = chiffrer(b"donnees", &cle()).unwrap();
        c[0] = 99;
        assert!(dechiffrer(&c, &cle()).is_err());
    }

    #[test]
    fn une_grande_base_est_chiffree_integralement() {
        let grosse = vec![0x5A; 3 * 1024 * 1024]; // 3 Mo
        let c = chiffrer(&grosse, &cle()).expect("chiffrement d'un gros fichier");
        assert!(c.len() > grosse.len(), "nonce + tag doivent être ajoutés");
        let r = dechiffrer(&c, &cle()).expect("déchiffrement");
        assert_eq!(r.len(), grosse.len());
        assert_eq!(r, grosse);
    }
}
