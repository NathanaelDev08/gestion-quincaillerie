# Gestion Quincaillerie (Tauri + React + SQLite)

Application desktop de gestion de quincaillerie, clonée depuis `GestionCommerciale` et adaptée métier.

## Contenu
- Stock : articles, catégories quincaillerie, alertes rupture, mouvements, inventaire
- Ventes comptoir (Caisse) : panier rapide, codes-barres, ticket, clôture Z, espèces / mobile money
- Devis → BL → Factures, avoirs, relances
- Achats fournisseurs, commandes, réceptions
- Clients / Fournisseurs, fidélité, promos
- Comptabilité, dépenses, paie, utilisateurs, paramètres
- Base SQLite locale via `tauri-plugin-sql` + `sqlx` (voir `src-tauri/src/db/mod.rs`)

## Spécifique quincaillerie
- `src/data/quincaillerie.ts` :
  - `CATEGORIES_QUINCAILLERIE` (12 rayons : outillage, visserie, serrurerie, plomberie, électricité, peinture, maçonnerie...)
  - `UNITES_QUINCAILLERIE` (pièce, boîte, sachet, kg, m, sac, rouleau...)
  - `CATALOGUE_INITIAL` (20 articles avec prix XOF) importable en 1 clic depuis page `Quincaillerie`
- Page `src/pages/Produits.tsx` : titre Articles Quincaillerie, filtre catégories fusionné, bouton Catalogue, datalists
- Branding : `Gestion Quincaillerie`, `src/components/layout/Layout.tsx`, `src/App.tsx`, `index.html`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`

## Lancer
```bash
cd gestion-quincaillerie
npm install
npm run tauri dev
```

Build :
```bash
npm run tauri build
```

## Workflow boutique type
1. Page Quincaillerie → Catalogue → vérifie stocks/seuils
2. Caisse → vente comptoir → ticket + décrément stock auto
3. Stock → alertes → Achats → commande fournisseur → réception
4. Factures → règlements → Relances → Comptabilité
5. Clôture Z fin de journée (Caisse → Clôtures)
