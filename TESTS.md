# Tests

## Front (Vitest)
```bash
npm test              # une passe
npm run test:watch    # mode continu
npm run test:coverage # avec rapport de couverture
```

Couvre la logique métier pure, sans navigateur ni Tauri : calculs de la
caisse (HT/TVA/TTC, remises, monnaie), recherche par code-barres, gestion du
panier, pavé numérique.

Quand vous ajoutez une règle de calcul dans `src/utils/`, ajoutez son test
dans le fichier `*.test.ts` voisin. Une formule d'argent sans test est une
formule qui finira par être fausse en production.

## Rust
```bash
cd src-tauri
cargo test              # unitaires + intégration
cargo test --test flux_caisse   # scénario de panne en caisse
```

Couvre la sécurité (matrice d'autorisation, anti-bruteforce), la validation
des données, le chiffrement des sauvegardes, les migrations de schéma et le
flux de vente de bout en bout — y compris une panne simulée en plein
encaissement.

## Les deux
```bash
npm run test && npm run test:rust
```

## Avant toute livraison
```bash
npm run tauri build
```
Cela produit l'installateur MSI et NSIS dans
`src-tauri/target/release/bundle/`. La CI `.github/workflows/verification.yml`
fait de même à chaque push sur `main`.
