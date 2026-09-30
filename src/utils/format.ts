import { getCurrency } from "../stores/prefs";

export const fmtMoney = (n: number, currency?: string) =>
  new Intl.NumberFormat("fr-FR", { style: "currency", currency: currency || getCurrency() }).format(n || 0);

export const fmtDate = (d?: string) => {
  if (!d) return "-";
  try { return new Date(d).toLocaleDateString("fr-FR"); } catch { return d; }
};

export const todayISO = () => new Date().toISOString().slice(0, 10);

/** Montants compacts : 4 665 720 → « 4,67 M F CFA » */
export const fmtCompact = (n: number, currency = "XOF") =>
  new Intl.NumberFormat("fr-FR", { style: "currency", currency, notation: "compact" }).format(n || 0);

const MOIS_FR = ["Janv.", "Févr.", "Mars", "Avr.", "Mai", "Juin", "Juil.", "Août", "Sept.", "Oct.", "Nov.", "Déc."];
/** "2026-08" → "Août 26" */
export const fmtMois = (ym: string) => {
  const m = /^(\d{4})-(\d{2})$/.exec(ym);
  if (!m) return ym;
  const idx = +m[2] - 1;
  return `${MOIS_FR[idx] ?? m[2]} ${m[1].slice(2)}`;
};

export const statutColor = (s: string) => {
  const m: Record<string, string> = {
    brouillon: "bg-slate-200 text-slate-700",
    envoye: "bg-blue-100 text-blue-700",
    emis: "bg-blue-100 text-blue-700",
    emise: "bg-blue-100 text-blue-700",
    accepte: "bg-green-100 text-green-700",
    payee: "bg-green-100 text-green-700",
    partiellement_payee: "bg-amber-100 text-amber-700",
    refuse: "bg-red-100 text-red-700",
    annulee: "bg-red-100 text-red-700",
    expire: "bg-orange-100 text-orange-700",
  };
  return m[s] || "bg-slate-200 text-slate-700";
};

/** Libellés lisibles pour les codes techniques */
export const STATUTS_DEVIS = [
  { code: "brouillon", label: "Brouillon" },
  { code: "envoye", label: "Envoyé" },
  { code: "accepte", label: "Accepté" },
  { code: "refuse", label: "Refusé" },
  { code: "expire", label: "Expiré" },
];
export const STATUTS_FACTURE = [
  { code: "brouillon", label: "Brouillon" },
  { code: "emise", label: "Émise" },
  { code: "partiellement_payee", label: "Partiellement payée" },
  { code: "payee", label: "Payée" },
  { code: "annulee", label: "Annulée" },
];
export const MODES_REGLEMENT = [
  { code: "virement", label: "Virement bancaire" },
  { code: "especes", label: "Espèces" },
  { code: "cheque", label: "Chèque" },
  { code: "cb", label: "Carte bancaire" },
  { code: "mobile", label: "Mobile Money" },
  { code: "prelevement", label: "Prélèvement" },
];
export const JOURNAUX = [
  { code: "VTE", label: "Ventes" },
  { code: "ACH", label: "Achats" },
  { code: "BQ", label: "Banque" },
  { code: "CAISSE", label: "Caisse" },
  { code: "OD", label: "Opérations diverses" },
];
export const TYPES_MOUVEMENT: Record<string, string> = {
  entree: "Entrée en stock",
  sortie: "Sortie de stock",
  inventaire: "Inventaire",
};

export const labelOf = (list: { code: string; label: string }[], code?: string) =>
  list.find((x) => x.code === code)?.label ?? code ?? "-";

/** Catégories de dépenses avec libellés propres */
export const CATEGORIES_DEPENSE = [
  { code: "loyer", label: "Loyer" },
  { code: "salaires", label: "Salaires" },
  { code: "transport", label: "Transport" },
  { code: "fournitures", label: "Fournitures" },
  { code: "communication", label: "Communication" },
  { code: "energie", label: "Énergie" },
  { code: "impots", label: "Impôts & taxes" },
  { code: "divers", label: "Divers" },
];
export const catDepenseLabel = (code?: string) => labelOf(CATEGORIES_DEPENSE, code);

export const statutLabel = (s: string) =>
  labelOf([...STATUTS_DEVIS, ...STATUTS_FACTURE], s);
