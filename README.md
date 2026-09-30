# Gestion Commerciale — Tauri + React + Rust + SQLite

Application desktop de gestion commerciale offline-first.

## Modules
- Auth (admin auto au 1er compte, JWT + Argon2)
- Clients / Fournisseurs (CRUD, recherche)
- Produits (CRUD, stock, alertes seuil)
- Devis (création multi-lignes, conversion en facture, PDF HTML)
- Factures (TVA auto, écritures comptables auto, règlements, lettrage, PDF)
- Stock (état, mouvements, inventaire)
- Comptabilité (journal, grand livre, balance, exercices)
- Dashboard (CA, top produits/clients, impayés, retard)
- Exports CSV + Backup SQLite

## Prérequis
1. Rust : https://win.rustup.rs/x86_64 (puis `rustup default stable`)
2. Node 20+ + pnpm : `npm i -g pnpm`
3. Visual Studio Build Tools (Windows) : workload "Desktop C++"
4. WebView2 (déjà sur Win10/11)

## Lancement
```powershell
cd GestionCommerciale
pnpm install
pnpm tauri dev
```

Build prod :
```powershell
pnpm build
pnpm tauri build
```

## Compte initial
- Allez sur /register, créez `admin` / `admin123` → rôle admin automatique.
- Ensuite login.

## Base de données
- SQLite : `%APPDATA%\com.gestioncommerciale.app\gestion.db`
- Backup : `Documents\GestionCommerciale\backups\`
- PDF (HTML imprimable) : `Documents\GestionCommerciale\FAC-*.html`

## Numérotation
- DEV-AAAA-0001, FAC-AAAA-0001 via table `sequences`, sans trou par année.

## Comptabilité auto
- Facture → VTE : 411 (D: TTC) / 707 (C: HT) / 445710 (C: TVA)
- Règlement → MAJ montant_paye + statut payee/partiellement_payee
