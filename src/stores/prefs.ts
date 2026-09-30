import { api } from "../services/api";

export interface Prefs {
  default_tva: number;
  currency: string;
  prefixes: Record<string, string>;
}

export const DEFAULT_PREFS: Prefs = {
  default_tva: 20,
  currency: "XOF",
  prefixes: { devis: "DEV", facture: "FAC", livraison: "BL", commande: "BC", avoir: "AV", bulletin: "BUL" },
};

const LS_KEY = "gc-prefs";

function readLocal(): Prefs {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return { ...DEFAULT_PREFS, prefixes: { ...DEFAULT_PREFS.prefixes } };
    const p = JSON.parse(raw);
    return {
      default_tva: typeof p.default_tva === "number" ? p.default_tva : 20,
      currency: typeof p.currency === "string" && p.currency ? p.currency : "XOF",
      prefixes: { ...DEFAULT_PREFS.prefixes, ...(p.prefixes || {}) },
    };
  } catch {
    return { ...DEFAULT_PREFS, prefixes: { ...DEFAULT_PREFS.prefixes } };
  }
}

/** Lecture synchrone (formulaires, formatteurs) */
export function getPrefs(): Prefs {
  return readLocal();
}
export function getDefaultTva(): number {
  return readLocal().default_tva;
}
export function getCurrency(): string {
  return readLocal().currency;
}

/** Chargement depuis le store Tauri (source de vérité partagée) */
export async function loadPrefs(): Promise<Prefs> {
  try {
    const v = await api.settingsGet("preferences");
    if (v && typeof v === "object") {
      const merged: Prefs = {
        default_tva: typeof (v as any).default_tva === "number" ? (v as any).default_tva : 20,
        currency: typeof (v as any).currency === "string" && (v as any).currency ? (v as any).currency : "XOF",
        prefixes: { ...DEFAULT_PREFS.prefixes, ...((v as any).prefixes || {}) },
      };
      localStorage.setItem(LS_KEY, JSON.stringify(merged));
      return merged;
    }
  } catch { /* mode hors-ligne : garde le local */ }
  return readLocal();
}

export async function savePrefs(p: Prefs): Promise<void> {
  localStorage.setItem(LS_KEY, JSON.stringify(p));
  try {
    await api.settingsSet("preferences", p as any);
  } catch { /* persisté au moins en local */ }
}
