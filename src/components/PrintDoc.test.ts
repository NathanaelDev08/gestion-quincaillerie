/**
 * Tests des totaux imprimés sur le ticket de caisse.
 *
 * Un ticket dont les lignes ne s'additionnent pas est un ticket contesté par
 * le client et rejeté au contrôle. Ces tests verrouillent la cohérence entre
 * les lignes, le total HT, la TVA et le TTC affiché.
 */
import { describe, it, expect } from "vitest";
import { totauxTicket } from "./PrintDoc";

describe("Totaux du ticket imprimé", () => {
  it("additionne les lignes correctement", () => {
    const t = totauxTicket([
      { quantite: 2, prix_unitaire_ht: 5500, taux_tva: 18, remise: 0 },
      { quantite: 1, prix_unitaire_ht: 1800, taux_tva: 18, remise: 0 },
    ]);
    expect(t.ht).toBe(12800);
    expect(t.tva).toBe(2304);
    expect(t.ttc).toBe(15104);
  });

  it("reste cohérent avec une remise de ligne", () => {
    // 1000 HT − 10 % = 900 HT, TVA 162, TTC 1 062
    const t = totauxTicket([
      { quantite: 1, prix_unitaire_ht: 1000, taux_tva: 18, remise: 10 },
    ]);
    expect(t.ht + t.tva).toBe(t.ttc);
    expect(t.ht).toBe(900);
  });

  it("reste cohérent avec un code promo global", () => {
    const lignes = [{ quantite: 2, prix_unitaire_ht: 5000, taux_tva: 18, remise: 0 }];
    const sansPromo = totauxTicket(lignes);
    const avecPromo = totauxTicket(lignes, 10);
    expect(avecPromo.ht).toBe(9000);
    expect(avecPromo.ttc).toBe(10620);
    // L'identité doit tenir dans les deux cas.
    expect(Math.abs(avecPromo.ht + avecPromo.tva - avecPromo.ttc)).toBeLessThan(0.01);
    expect(sansPromo.ttc).toBe(11800);
  });

  it("cumule remise de ligne ET promo globale", () => {
    const t = totauxTicket(
      [{ quantite: 1, prix_unitaire_ht: 1000, taux_tva: 18, remise: 10 }],
      10,
    );
    // 1000 × 0,9 (ligne) × 0,9 (global) = 810 HT
    expect(t.ht).toBe(810);
    expect(t.ht + t.tva).toBe(t.ttc);
  });

  it("gère un ticket vide sans planter", () => {
    const t = totauxTicket([]);
    expect(t).toEqual({ ht: 0, tva: 0, ttc: 0 });
  });

  it("gère un article sans remise (champ absent)", () => {
    const t = totauxTicket([
      { quantite: 1, prix_unitaire_ht: 1000, taux_tva: 18 } as any,
    ]);
    expect(t.ht).toBe(1000);
  });

  it("produit des montants affichables (aucun flottant parasite)", () => {
    const t = totauxTicket([
      { quantite: 3, prix_unitaire_ht: 3333, taux_tva: 18, remise: 0 },
    ]);
    // Sans arrondi, « 9999 » s'afficherait 9999.000000001.
    expect(String(t.ht)).toBe("9999");
    expect(String(t.ttc)).toBe("11798.82");
  });
});
