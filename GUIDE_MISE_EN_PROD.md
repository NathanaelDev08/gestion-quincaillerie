# Mise en production — Gestion Quincaillerie v1.0.0

## Correctifs appliqués
- Rust : warnings supprimés (`services/mod.rs`, `main.rs:12`, `numbering.rs:3` avec `#[allow(dead_code)]`)
- Chemins prod renommés : `Documents/GestionQuincaillerie/`, backups, exports PDF (`db/mod.rs:33`, `commands/documents.rs`, `commands/system.rs`)
- Capabilities : description `GestionQuincaillerie` (`capabilities/default.json`)
- Message navigateur corrigé vers « Gestion Quincaillerie » (`services/api.ts:10`)
- Caisse TTC : totaux HT/TVA/TTC cohérents avec backend (`pages/Caisse.tsx`, `commands/caisse.rs:46`)
- Version 1.0.0 (`package.json`, `tauri.conf.json`, `Cargo.toml`)

## Avant d'installer chez le client
1. Changer le mot de passe admin (seed : `admin` / `admin123`, `src-tauri/src/db/mod.rs:308`).
   Page Utilisateurs → modifier, ou supprimer le compte démo après création du vrai admin.
2. Paramètres → préfixes documents (FAC, DEV, BL…), TVA par défaut 18 %, devise XOF.
3. Page Quincaillerie → bouton Catalogue → contrôle stocks/seuils → 1 vente test → ticket → clôture Z.
4. Tester Sauvegarde / Restauration (`commands/system.rs`) et vérifier `Documents/GestionQuincaillerie/backups/auto-YYYYMMDD-*`.
5. Inventaire d'ouverture : Stock → ajustements, codes-barres, étiquettes.

## Build installateur
```bash
cd gestion-quincaillerie
npm.cmd install
npm.cmd run build        # tsc + vite (dist/)
npm.cmd run tauri build  # exe + MSI/NSIS dans src-tauri/target/release/bundle/
```
Distribuer le MSI/NSIS Windows. Webview2 inclus via `embedBootstrapper` (`tauri.conf.json:42`).

## Exploitation
- Données : `%APPDATA%/com.quincaillerie.app/gestion.db` (SQLite, WAL). Ne jamais supprimer pendant que l'app tourne.
- Sauvegarde auto quotidienne + bouton manuel. Conserver les 7 dernières, copie externe hebdo.
- Mises à jour : livrer nouveau MSI, la base est conservée (migrations idempotentes `run_migrations*`).
- Support : Page Paramètres → Diagnostic (`diagnostic_perf`), journal eprintln `[DB]`.

## Risques connus
- CSP `null` (local uniquement, pas d'URL distante) — acceptable Tauri local.
- Bundle JS ~860 kB : code-splitting possible plus tard (lazy routes), non bloquant.
- Anciens dossiers `Documents/GestionCommerciale/` : migrer les backups à la main si réinstall depuis l'ancienne app.
