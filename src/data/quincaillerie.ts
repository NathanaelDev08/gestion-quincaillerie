// Presets métier pour quincaillerie — catégories, unités, catalogue de démarrage.
export const CATEGORIES_QUINCAILLERIE = [
  "Outillage à main",
  "Outillage électrique",
  "Visserie / Boulonnerie",
  "Serrurerie",
  "Plomberie",
  "Électricité",
  "Peinture / Droguerie",
  "Menuiserie / Bois",
  "Maçonnerie / Ciment",
  "Jardinage",
  "Quincaillerie bâtiment",
  "Divers",
];

export const UNITES_QUINCAILLERIE = [
  "pièce",
  "pcs",
  "boîte",
  "sachet",
  "kg",
  "m",
  "ml",
  "litre",
  "sac",
  "rouleau",
  "paire",
  "carton",
];

export interface ArticleSeed {
  reference: string;
  designation: string;
  categorie: string;
  unite: string;
  prix_achat_ht: number;
  prix_vente_ht: number;
  stock: number;
  stock_alerte: number;
}

// Catalogue de démarrage réaliste (prix en XOF HT)
export const CATALOGUE_INITIAL: ArticleSeed[] = [
  { reference: "CIM-50KG", designation: "Ciment CPJ 50kg", categorie: "Maçonnerie / Ciment", unite: "sac", prix_achat_ht: 4500, prix_vente_ht: 5500, stock: 100, stock_alerte: 20 },
  { reference: "FER-08", designation: "Fer à béton Ø8 (12m)", categorie: "Maçonnerie / Ciment", unite: "pièce", prix_achat_ht: 2800, prix_vente_ht: 3500, stock: 80, stock_alerte: 15 },
  { reference: "FER-10", designation: "Fer à béton Ø10 (12m)", categorie: "Maçonnerie / Ciment", unite: "pièce", prix_achat_ht: 3800, prix_vente_ht: 4600, stock: 60, stock_alerte: 10 },
  { reference: "VIS-4X40", designation: "Vis bois 4x40 (boîte 200)", categorie: "Visserie / Boulonnerie", unite: "boîte", prix_achat_ht: 1200, prix_vente_ht: 1800, stock: 50, stock_alerte: 10 },
  { reference: "CHEV-8", designation: "Chevilles nylon Ø8 (sachet 100)", categorie: "Visserie / Boulonnerie", unite: "sachet", prix_achat_ht: 900, prix_vente_ht: 1500, stock: 60, stock_alerte: 10 },
  { reference: "CAD-40", designation: "Cadenas 40mm", categorie: "Serrurerie", unite: "pièce", prix_achat_ht: 2500, prix_vente_ht: 3500, stock: 30, stock_alerte: 5 },
  { reference: "SER-BEC", designation: "Serrure bec-de-cane", categorie: "Serrurerie", unite: "pièce", prix_achat_ht: 4000, prix_vente_ht: 5500, stock: 20, stock_alerte: 4 },
  { reference: "MART-500", designation: "Marteau menuisier 500g", categorie: "Outillage à main", unite: "pièce", prix_achat_ht: 3000, prix_vente_ht: 4500, stock: 25, stock_alerte: 5 },
  { reference: "PERC-13", designation: "Perceuse 13mm 750W", categorie: "Outillage électrique", unite: "pièce", prix_achat_ht: 22000, prix_vente_ht: 29000, stock: 8, stock_alerte: 2 },
  { reference: "PEINT-VIN-15", designation: "Peinture vinylique 15L blanc", categorie: "Peinture / Droguerie", unite: "pièce", prix_achat_ht: 14000, prix_vente_ht: 18000, stock: 15, stock_alerte: 3 },
  { reference: "ROUL-180", designation: "Rouleau peinture 180mm", categorie: "Peinture / Droguerie", unite: "pièce", prix_achat_ht: 800, prix_vente_ht: 1300, stock: 40, stock_alerte: 8 },
  { reference: "TUY-PVC100", designation: "Tuyau PVC Ø100 (4m)", categorie: "Plomberie", unite: "pièce", prix_achat_ht: 5000, prix_vente_ht: 6500, stock: 20, stock_alerte: 5 },
  { reference: "ROB-15", designation: "Robinet d'arrêt 15/21", categorie: "Plomberie", unite: "pièce", prix_achat_ht: 1800, prix_vente_ht: 2700, stock: 30, stock_alerte: 6 },
  { reference: "CAB-2.5", designation: "Câble électrique 2.5mm (rouleau 100m)", categorie: "Électricité", unite: "rouleau", prix_achat_ht: 15000, prix_vente_ht: 19000, stock: 10, stock_alerte: 2 },
  { reference: "AMP-LED9", designation: "Ampoule LED 9W E27", categorie: "Électricité", unite: "pièce", prix_achat_ht: 700, prix_vente_ht: 1200, stock: 100, stock_alerte: 20 },
  { reference: "PELL-MAN", designation: "Pelle manche bois", categorie: "Jardinage", unite: "pièce", prix_achat_ht: 3500, prix_vente_ht: 5000, stock: 15, stock_alerte: 3 },
  { reference: "CHAR-43KG", designation: "Charnière 140mm (paire)", categorie: "Quincaillerie bâtiment", unite: "paire", prix_achat_ht: 1100, prix_vente_ht: 1700, stock: 40, stock_alerte: 8 },
  { reference: "POIG-ALU", designation: "Poignée porte alu", categorie: "Quincaillerie bâtiment", unite: "paire", prix_achat_ht: 4500, prix_vente_ht: 6000, stock: 18, stock_alerte: 4 },
  { reference: "COLLE-BOIS", designation: "Colle à bois 500g", categorie: "Menuiserie / Bois", unite: "pièce", prix_achat_ht: 1500, prix_vente_ht: 2200, stock: 25, stock_alerte: 5 },
  { reference: "GANT-CUIR", designation: "Gants cuir chantier", categorie: "Divers", unite: "paire", prix_achat_ht: 1200, prix_vente_ht: 2000, stock: 50, stock_alerte: 10 },
];
