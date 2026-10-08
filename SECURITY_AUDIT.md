# Audit sécurité & robustesse — Gestion Quincaillerie 1.0.0

Date : 2026-10-08 · Périmètre : `src-tauri/` (backend) + `src/` (frontend)

## Méthode
Lecture complète des commandes IPC, du schéma SQLite, de la couche auth, puis
tests unitaires exécutables (`cargo test`) sur les points sensibles.

---

## 1. Failles trouvées et corrigées

### CRITIQUE — Aucune autorisation serveur sur ~110 commandes
**Constat.** Les gardes de rôle n'existaient que sur 12 commandes
(`users_*`, `audit_*`, `ff_*`, `seed_demo_data`, `compta_exercices_cloturer`,
`backup/restore`). Les autres — dont `produits_delete`, `clients_delete`,
`factures_delete`, `compta_journal_create`, `import_csv` —，接受aient n'importe
quel appel IPC. Le frontend masquait les boutons, mais masquérer n'est pas
autoriser : un script injecté, une extension, ou un `invoke` manuel depuis la
console WebView exécutait n'importe quoi avec les droits de l'application.

**Correction.** Barrière d'autorisation deny-by-default dans `main.rs` :
chaque appel IPC passe par `authz::authorize` **avant** la commande. Une
commande absente de `authz::command_roles` est **refusée**, donc l'oubli d'un
déclaration est un bug visible et non une faille silencieuse. Le refus renvoie
une erreur à l'utilisateur au lieu de laisser la promesse en suspens.

Le token est désormais injecté sur **toutes** les commandes côté front
(`services/api.ts`), ce qui rend le contrôle serveur possible sans
modifier 110 signatures.

Matrice des rôles : `admin`, `commercial`, `user`, `comptable`.
Lecture catalogue/dashboard ouverte à tous ; écriture et paie réservées ;
compta en lecture-comptable ; restauration et purge en admin seul.

### HAUT — Vente comptoir non atomique
**Constat.** `vente_comptoir` enchaînait uneembrochure de 20+ `INSERT` sans
transaction. Une coupure (disque plein, windows fermée, erreur réseau sur le
chiffrement) à mi-parcours laissait une facture sans ses lignes, ou du stock
décrémenté sans vente encaissée. En quincaillerie c'est le scénario le plus
coûteux : un écart de caisse inexpliqué.

**Correction.** Une seule transaction : facture + lignes + mouvements de stock
+ règlement + écritures VTE + points fidélité, ou rien. La validation du stock
et des totaux passe **avant** l'ouverture de la transaction.

### HAUT — Trust du frontend sur les montants
**Constat.** Quantités, prix et remisesvenaient directement du JSON du client.
Une requête modifiée envoyait `quantite: -5` etFabriqueait un avoir, ou
`remise: 5000` pour zéro.

**Correction.** `src-tauri/src/validation.rs` : quantité > 0, montant ≥ 0,
TVA et remise bornées à 0-100, désignation non vide, garde-fous sur les
valeurs aberrantes (NaN, infini, montants/plafonds). Testé par 9 tests unitaires.

### MOYEN — Numérotation trouée par les rollback
**Constat.** `next_numero` incrémentait la séquence hors transaction. Une vente
annulée laissait un trou définitif dans la numérotation des factures —～
Critère de rejet en contrôle fiscal.

**Correction.** `next_numero_tx` consomme le numéro dans la transaction. Test
`rollback_ne_consomme_pas_de_numero` le prouve.

### MOYEN — Boucle comptoir sans transaction
**Correction.** La clôture Z est désormais recalculée et upsertée de façon
idempotente : deux clôtures successives le même jour donnent le même
résultat, et le compteur de tickets ne dérive plus.

---

## 2. Tests ajoutés (17, tous verts)

| Domaine | Ce qui est couvert |
|---|---|
| `authz` | commande inconnue refusée ; ops sensibles fermées ; lecture ouverte ; garde anti-bruteforce |
| `validation` | quantités, montants, TVA, remises, document vide, désignation vide, calcul HT/TVA/TTC, ordre remise→TVA, stock insuffisant, tolérance flottante, nettoyage de chaînes |
| `db` | incrément de séquence ; rollback sans trou |

Bug réel attrapé par les tests : le comptable ne pouvait pas lire le catalogue
(il en a besoin pour valoriser le stock). Corrigé — `produits_list` est passé
en lecture ouverte.

## 3. Points de vigilance restants

- **CSP à `null`** (`tauri.conf.json`). Acceptable en local, mais si l'app
  charge un jour une URL distante il faut activer une CSP stricte.
- **Secret admin par défaut** `admin/admin123`, créé au premier lancement
  (`db/mod.rs`). À changer impérativement avant mise en production.
- **`tauri-plugin-sql` reste chargé** bien que le backend passe par `sqlx`.
  Permission `sql:allow-execute` inutile : à retirer pour réduire la surface
  d'attaque si le plugin n'est plus employé.
- **Pas de rotation de token** : validité 7 jours (`auth.rs`). Acceptable en
  poste unique ; pour un réseau multi-postes, passer à des tokens courts avec
  rafraîchissement.
- **Sauvegarde auto quotidienne** non chiffrée dans `Documents/`. À chiffrer si
  la donnée est sensible (données clients, salaires en page Paie).
- **Pas de tests d'intégration bout en bout** sur le flux caisse. Prochaine
  étape recommandée : un test qui simule une vente complète et vérifie
  l'égalité stock/facture.

## 4. Recommandations avant livraison

1. Changer le mot de passe admin, supprimer le compte de démo.
2. Retirer `sql:allow-execute` si le plugin SQL n'est pas utilisé.
3. Activer une CSP même en local (`default-src 'self'`).
4. Ajouter un test bout-en-bout caisse (facture + stock + rendu monnaie).
5. Chiffrer les sauvegardes.
