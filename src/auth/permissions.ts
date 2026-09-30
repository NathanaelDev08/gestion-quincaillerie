export type Role = "admin" | "commercial" | "comptable" | "user";

const ALL: Role[] = ["admin", "commercial", "comptable", "user"];

/** Pages autorisées par rôle */
export const PAGE_ACCESS: Record<string, Role[]> = {
  "/": ALL,
  "/clients": ALL,
  "/fournisseurs": ["admin", "user", "commercial"],
  "/produits": ALL,
  "/devis": ALL,
  "/factures": ALL,
  "/avoirs": ["admin", "commercial", "comptable"],
  "/achats": ["admin", "user"],
  "/livraisons": ["admin", "commercial", "user"],
  "/caisse": ["admin", "commercial", "user"],
  "/fidelite": ["admin", "commercial"],
  "/paie": ["admin", "comptable"],
  "/depenses": ["admin", "comptable"],
  "/relances": ["admin", "commercial", "comptable"],
  "/stock": ["admin", "user", "commercial"],
  "/comptabilite": ["admin", "comptable"],
  "/utilisateurs": ["admin"],
  "/parametres": ["admin"],
};

export function normalizeRole(r?: string): Role {
  if (r === "admin" || r === "commercial" || r === "comptable" || r === "user") return r;
  return "user";
}

export function canAccess(role: string | undefined, path: string): boolean {
  const allowed = PAGE_ACCESS[path] ?? [];
  return allowed.includes(normalizeRole(role));
}

export const ROLE_LABELS: Record<Role, string> = {
  admin: "Administrateur",
  commercial: "Commercial",
  comptable: "Comptable",
  user: "Utilisateur",
};
