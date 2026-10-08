/**
 * Tests de la logique de caisse — l'argent de la boutique.
 *
 * Chaque test vérifie un comportement qu'un vendeur verrait immédiatement.
 * Si la TVA ou la monnaie se calculent mal, c'est le client qui le remarque.
 */
import { describe, it, expect } from "vitest";
import {
  calculerTotaux, prixTTC, renduMonnaie, peutEncaisser, stockSuffisant,
  rechercherArticle, ajouterAuPanier, ajusterQuantite, saisiePavé, arrondi2, arrondi3,
} from "./caisse";

const cement = { produit_id: "p1", designation: "Ciment CPJ 50kg", quantite: 2, prix_unitaire_ht: 5500, taux_tva: 18, remise: 0, stock: 100 };
const vis = { produit_id: "p2", designation: "Vis bois 4x40", quantite: 1, prix_unitaire_ht: 1800, taux_tva: 18, remise: 0, stock: 50 };

describe("Totaux du ticket", () => {
  it("calcule HT, TVA et TTC sur une vente simple", () => {
    // 2 × 5500 = 11 000 HT, TVA 18 % = 1 980, TTC = 12 980
    const t = calculerTotaux([{ ...cement }]);
    expect(t.ht).toBe(11000);
    expect(t.tva).toBe(1980);
    expect(t.ttc).toBe(12980);
    expect(t.nbArticles).toBe(2);
  });

  it("cumule plusieurs articles", () => {
    const t = calculerTotaux([{ ...cement }, { ...vis }]);
    expect(t.ht).toBe(12800);
    expect(t.ttc).toBe(15104);
  });

  it("compte le nombre d'articles, pas le nombre de lignes", () => {
    const t = calculerTotaux([{ ...cement }, { ...vis, quantite: 3 }]);
    expect(t.nbArticles).toBe(5);
  });

  it("renvoie des totaux à zéro pour un panier vide", () => {
    const t = calculerTotaux([]);
    expect(t).toMatchObject({ ht: 0, tva: 0, ttc: 0, nbArticles: 0 });
  });

  it("applique une remise de ligne avant la TVA", () => {
    // 1000 HT, remise 10 % → 900 HT → TVA 162 → TTC 1 062
    const t = calculerTotaux([{ ...vis, prix_unitaire_ht: 1000, remise: 10 }]);
    expect(t.ht).toBe(900);
    expect(t.tva).toBe(162);
    expect(t.ttc).toBe(1062);
  });

  it("applique une remise globale après les remises de ligne", () => {
    const t = calculerTotaux([{ ...vis, prix_unitaire_ht: 1000 }], 50);
    expect(t.ht).toBe(500);
  });

  it("reste exact sur les fractions de centime", () => {
    // 3 articles à 33,33 € : 99,99 — l'arrondi flottant donnerait 99.98999...
    const t = calculerTotaux([
      { ...vis, quantite: 3, prix_unitaire_ht: 3333, taux_tva: 0 },
    ]);
    expect(t.ht).toBe(9999);
  });

  it("gère le poids et le mètre sans fausser le total", () => {
    const t = calculerTotaux([
      { ...vis, quantite: 2.5, prix_unitaire_ht: 1200, taux_tva: 18 },
    ]);
    expect(t.ht).toBe(3000);
    expect(t.ttc).toBe(3540);
  });

  it("ne produit jamais de total négatif même avec une remise de 100 %", () => {
    const t = calculerTotaux([{ ...vis, remise: 100 }]);
    expect(t.ht).toBe(0);
    expect(t.ttc).toBe(0);
    expect(t.ttc).toBeGreaterThanOrEqual(0);
  });

  it("ignore une remise globale aberrante sans planter", () => {
    expect(() => calculerTotaux([{ ...vis }], 999)).not.toThrow();
    expect(calculerTotaux([{ ...vis }], 999).ttc).toBeGreaterThanOrEqual(0);
  });
});

describe("Prix affiché en caisse", () => {
  it("affiche le prix TTC avec la TVA", () => {
    expect(prixTTC(5500, 18)).toBe(6490);
  });

  it("gère un article à TVA nulle", () => {
    expect(prixTTC(1000, 0)).toBe(1000);
  });
});

describe("Monnaie à rendre", () => {
  it("calcule la monnaie rendue", () => {
    expect(renduMonnaie(12980, 20000)).toBe(7020);
  });

  it("donne zéro quand le client donne juste", () => {
    expect(renduMonnaie(12980, 12980)).toBe(0);
  });

  it("signale un montant insuffisant par une valeur négative", () => {
    expect(renduMonnaie(18408, 16000)).toBe(-2408);
  });
});

describe("Encaissement possible ou non", () => {
  it("refuse un panier vide", () => {
    expect(peutEncaisser([], 0, 10000)).toBe(false);
  });

  it("refuse un ticket à zéro (remise totale)", () => {
    expect(peutEncaisser([{ ...vis, remise: 100 }], 0, 10000)).toBe(false);
  });

  it("refuse un montant insuffisant", () => {
    expect(peutEncaisser([{ ...cement }], 12980, 10000)).toBe(false);
  });

  it("accepte le montant exact", () => {
    expect(peutEncaisser([{ ...cement }], 12980, 12980)).toBe(true);
  });

  it("accepte un montant supérieur et tolère l'arrondi flottant", () => {
    expect(peutEncaisser([{ ...vis }], 2124, 2124.0000001)).toBe(true);
  });
});

describe("Contrôle du stock", () => {
  it("autorise une quantité disponible", () => {
    expect(stockSuffisant(10, 3, 7)).toBe(true);
  });

  it("refuse un dépassement", () => {
    expect(stockSuffisant(10, 3, 8)).toBe(false);
  });
});

describe("Recherche d'article (scan et saisie)", () => {
  const catalogue = [
    { ...cement, reference: "CIM-50KG", code_barre: "6111234567890" },
    { ...vis, reference: "VIS-4X40", code_barre: "6119999999999" },
  ];

  it("trouve par code-barres exact", () => {
    expect(rechercherArticle("6111234567890", catalogue)?.designation).toBe("Ciment CPJ 50kg");
  });

  it("trouve par référence, peu importe la casse", () => {
    expect(rechercherArticle("cim-50kg", catalogue)?.designation).toBe("Ciment CPJ 50kg");
  });

  it("trouve par code-barres malgré les espaces et tirets", () => {
    expect(rechercherArticle("611 123-4567890", catalogue)?.designation).toBe("Ciment CPJ 50kg");
  });

  it("trouve par mot-clé de désignation", () => {
    expect(rechercherArticle("ciment", catalogue)?.designation).toBe("Ciment CPJ 50kg");
  });

  it("ne confond pas deux articles voisins", () => {
    // Un scan partiel ne doit pas tomber sur le mauvais article.
    expect(rechercherArticle("6119999999999", catalogue)?.designation).toBe("Vis bois 4x40");
  });

  it("renvoie undefined si rien ne correspond", () => {
    expect(rechercherArticle("inexistant", catalogue)).toBeUndefined();
  });

  it("renvoie undefined sur une recherche vide", () => {
    expect(rechercherArticle("   ", catalogue)).toBeUndefined();
  });
});

describe("Gestion du panier", () => {
  it("ajoute un nouvel article en une unité, quelle que soit la quantité fournie", () => {
    const panier = ajouterAuPanier([], cement);
    expect(panier).toHaveLength(1);
    expect(panier[0].quantite).toBe(1);
    // Garde-fou anti-double-clic : un second appel avec une quantité
    // farfelu ne doit pas fausser le ticket.
    const autre = ajouterAuPanier([], cement, 5);
    expect(autre[0].quantite).toBe(1);
  });

  it("incrémente un article déjà présent au lieu de le dupliquer", () => {
    let panier = ajouterAuPanier([], vis);
    panier = ajouterAuPanier(panier, vis);
    panier = ajouterAuPanier(panier, vis);
    expect(panier).toHaveLength(1);
    expect(panier[0].quantite).toBe(3);
  });

  it("ne descend jamais sous zéro", () => {
    const panier = ajusterQuantite([{ ...vis, quantite: 1 }], "p2", -5);
    expect(panier).toHaveLength(0);
  });

  it("supprime la ligne quand la quantité atteint zéro", () => {
    const panier = ajusterQuantite([{ ...vis, quantite: 1 }], "p2", -1);
    expect(panier).toHaveLength(0);
  });

  it("incrémente normalement", () => {
    const panier = ajusterQuantite([{ ...vis, quantite: 2 }], "p2", 3);
    expect(panier[0].quantite).toBe(5);
  });
});

describe("Pavé numérique", () => {
  it("ajoute les chiffres", () => {
    expect(saisiePavé("12", "3")).toBe("123");
  });

  it("efface tout avec C", () => {
    expect(saisiePavé("12345", "C")).toBe("");
  });

  it("efface le dernier chiffre", () => {
    expect(saisiePavé("123", "⌫")).toBe("12");
  });

  it("ignore tout ce qui n'est pas un chiffre", () => {
    expect(saisiePavé("12", "a")).toBe("12");
  });

  it("borne à 9 chiffres", () => {
    expect(saisiePavé("123456789", "9")).toHaveLength(9);
  });
});

describe("Arrondis", () => {
  it("arrondit à 2 décimales", () => {
    expect(arrondi2(1500.005)).toBe(1500.01);
    expect(arrondi2(2.345)).toBe(2.35);
  });

  it("arrondit à 3 décimales", () => {
    expect(arrondi3(1.0005)).toBe(1.001);
  });
});
