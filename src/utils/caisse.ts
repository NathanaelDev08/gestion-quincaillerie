/**
 * Logique de calcul de la caisse, isolée du composant React.
 *
 * Extraire ces règles permet de les tester sans navigateur ni Tauri. C'est
 * ici que se décide l'argent : un total faux se retrouve sur le ticket, dans
 * la facture et en comptabilité. Aucune formule ne doit vivre uniquement
 * dans un `.tsx`.
 */

export type LigneCalcul = {
  produit_id?: string | null;
  designation: string;
  quantite: number;
  prix_unitaire_ht: number;
  taux_tva: number;
  remise: number;
};

export type Totaux = {
  ht: number;
  tva: number;
  ttc: number;
  nbArticles: number;
};

/** Arrondi comptable à 2 décimales, comme côté serveur. */
export function arrondi2(n: number): number {
  return Math.round((n + Number.EPSILON) * 100) / 100;
}

/** Arrondi à 3 décimales pour les quantités (câble au mètre, pesée). */
export function arrondi3(n: number): number {
  return Math.round((n + Number.EPSILON) * 1000) / 1000;
}

/**
 * Totaux HT / TVA / TTC d'un ticket.
 *
 * La remise s'applique sur le prix unitaire AVANT la TVA — l'ordre inverse
 * donnerait un résultat différent du backend et un écart de centimes.
 */
export function calculerTotaux(lignes: LigneCalcul[], remiseGlobalePct = 0): Totaux {
  // Une remise est un pourcentage : elle est bornée à 0-100. Sans ce garde-fou,
  // une valeur aberrante (code promo corrompu, réponse serveur tronquée)
  // produirait un ticket négatif — donc un remboursement en votre faveur.
  let pct = Number(remiseGlobalePct);
  if (!Number.isFinite(pct) || pct < 0) pct = 0;
  if (pct > 100) pct = 100;
  let ht = 0;
  let tva = 0;
  let nbArticles = 0;
  for (const l of lignes) {
    const q = Number(l.quantite) || 0;
    const pu = Number(l.prix_unitaire_ht) || 0;
    const taux = Number(l.taux_tva) || 0;
    const r = Number(l.remise) || 0;
    const totalLigne = q * pu * (1 - pct / 100) * (1 - r / 100);
    ht += totalLigne;
    tva += (totalLigne * taux) / 100;
    nbArticles += q;
  }
  const htA = arrondi2(ht);
  const tvaA = arrondi2(tva);
  return { ht: htA, tva: tvaA, ttc: arrondi2(htA + tvaA), nbArticles };
}

/** Prix TTC unitaire (affichage en caisse). */
export function prixTTC(prixHT: number, tauxTva: number): number {
  return arrondi2((Number(prixHT) || 0) * (1 + (Number(tauxTva) || 0) / 100));
}

/** Montant à rendre. Négatif = le client n'a pas assez donné. */
export function renduMonnaie(totalTTC: number, recu: number): number {
  return arrondi2((Number(recu) || 0) - (Number(totalTTC) || 0));
}

/** La vente peut-elle être encaissée ? */
export function peutEncaisser(lignes: LigneCalcul[], totalTTC: number, recu: number): boolean {
  if (lignes.length === 0) return false;
  if (!(totalTTC > 0)) return false;
  const donne = Number(recu) || 0;
  // Une marge d'un centime évite de bloquer à cause d'un flottant.
  return donne + 0.01 >= totalTTC;
}

/** Un article peut-il être ajouté au panier sans dépasser le stock ? */
export function stockSuffisant(stock: number, dejaAuPanier: number, quantiteAjout: number): boolean {
  return dejaAuPanier + quantiteAjout <= (Number(stock) || 0);
}

/**
 * Cherche un article par code-barres, référence ou désignation.
 * Priorité au code-barres (scan), puis correspondance exacte de la référence,
 * puis recherche floue. Une caisse doit rester utilisable avec des articles
 * mal étiquetés.
 */
export function rechercherArticle<T extends { reference?: string; designation?: string; code_barre?: string }>(
  recherche: string,
  catalogue: T[],
): T | undefined {
  const q = recherche.trim().toLowerCase();
  if (!q) return undefined;

  const parCode = catalogue.find((p) => p.code_barre && p.code_barre.toLowerCase() === q);
  if (parCode) return parCode;

  const parRef = catalogue.find((p) => p.reference && p.reference.toLowerCase() === q);
  if (parRef) return parRef;

  // Un scan peutcts shots avec un préfixe/suffixe parasite (espaces, tirets).
  const compact = q.replace(/[\s-]/g, "");
  if (compact !== q) {
    const parCodeCompact = catalogue.find((p) => p.code_barre && p.code_barre.replace(/[\s-]/g, "").toLowerCase() === compact);
    if (parCodeCompact) return parCodeCompact;
  }

  return catalogue.find((p) =>
    `${p.designation ?? ""} ${p.reference ?? ""}`.toLowerCase().includes(q),
  );
}

/** Ajoute (ou incrémente) une ligne dans le panier. */
export function ajouterAuPanier<T extends LigneCalcul>(
  panier: T[],
  article: T,
  quantite = 1,
): T[] {
  // Le premier ajout d'un article vaut toujours 1 unité : passer la quantité
  // dans l'appel serait une source d'erreur silencieuse (un double-clic rapide
  // sur une tuile ne doit pas doubler la quantité).
  const q = Math.max(1, Math.round(quantite));
  const existant = panier.find((l) => l.produit_id === article.produit_id);
  if (existant) {
    return panier.map((l) =>
      l.produit_id === article.produit_id ? { ...l, quantite: l.quantite + q } : l,
    );
  }
  return [...panier, { ...article, quantite: 1 }];
}

/** Incrémente / décrémente une ligne, sans descendre sous zéro. */
export function ajusterQuantite<T extends LigneCalcul>(
  panier: T[],
  produitId: string | null | undefined,
  delta: number,
): T[] {
  return panier
    .map((l) =>
      l.produit_id === produitId
        ? { ...l, quantite: Math.max(0, arrondi3(l.quantite + delta)) }
        : l,
    )
    .filter((l) => l.quantite > 0);
}

/** Concatène un pavé numérique sur le montant saisi. */
export function saisiePavé(valeurActuelle: string, touche: string): string {
  if (touche === "C") return "";
  if (touche === "⌫") return valeurActuelle.slice(0, -1);
  // Un montant de caisse reste raisonnable : on borne à 9 chiffres.
  return (valeurActuelle + touche).replace(/[^0-9]/g, "").slice(0, 9);
}
