import { BrowserRouter, Routes, Route, Navigate, useLocation } from "react-router-dom";
import { QueryClient, QueryClientProvider, MutationCache, QueryCache } from "@tanstack/react-query";
import Layout from "./components/layout/Layout";
// Chargement paresseux : le vendeur n'ouvre qu'une page à la fois.
// Sans cela, tout le code (réactif, graphiques, caisse, paie) part dans un
// seul bloc téléchargé au démarrage, y compris sur un poste de boutique
// avec un disque lent.
import { lazy, Suspense } from "react";
const Dashboard = lazy(() => import("./pages/Dashboard"));
const Clients = lazy(() => import("./pages/Clients"));
const Fournisseurs = lazy(() => import("./pages/Fournisseurs"));
const Produits = lazy(() => import("./pages/Produits"));
const Devis = lazy(() => import("./pages/Devis"));
const Factures = lazy(() => import("./pages/Factures"));
const Avoirs = lazy(() => import("./pages/Avoirs"));
const Achats = lazy(() => import("./pages/Achats"));
const Livraisons = lazy(() => import("./pages/Livraisons"));
const Caisse = lazy(() => import("./pages/Caisse"));
const Paie = lazy(() => import("./pages/Paie"));
const Fidelite = lazy(() => import("./pages/Fidelite"));
const Depenses = lazy(() => import("./pages/Depenses"));
const Relances = lazy(() => import("./pages/Relances"));
const Utilisateurs = lazy(() => import("./pages/Utilisateurs"));
const Stock = lazy(() => import("./pages/Stock"));
const Depots = lazy(() => import("./pages/Depots"));
const Audit = lazy(() => import("./pages/Audit"));
const Incidents = lazy(() => import("./pages/Incidents"));
const Comptabilite = lazy(() => import("./pages/Comptabilite"));
const Parametres = lazy(() => import("./pages/Parametres"));
import Login, { Register } from "./pages/Login";
import { useAuth } from "./stores/useAuth";
import { isTauriRuntime } from "./services/api";
import { canAccess } from "./auth/permissions";
import { Toaster, toast } from "./components/ui";
import { loadPrefs } from "./stores/prefs";
import { useEffect } from "react";
import { Loader2 } from "lucide-react";

/** Une erreur d'authentification n'est pas une erreur métier : elle doit
 * fermer la session plutôt que d'afficher un message ou boucler des
 * requêtes sur une session morte.
 */
function estErreurSession(e: unknown): boolean {
  const t = String(e ?? "").toLowerCase();
  return t.includes("session invalide") || t.includes("session expirée") ||
         t.includes("expired") || t.includes("token invalide");
}

/**
 * Renouvellement automatique de la session.
 *
 * Le jeton d'accès est court (1 h). Sans ce mécanisme, le vendeur se
 * retrouverait déconnecté en pleine journée de caisse. On rafraîchit 5 min
 * avant l'expiration, et une seule fois à la fois pour éviter les rafales.
 */
let rafraichissementEnCours = false;
function planifierRenouvellement() {
  // 55 min : marge sur un jeton d'une heure.
  setTimeout(async () => {
    if (rafraichissementEnCours) {
      planifierRenouvellement();
      return;
    }
    rafraichissementEnCours = true;
    try {
      const refresh = localStorage.getItem("refresh");
      if (!refresh) return; // plus de session à renouveler
      const r = await import("./services/api").then((m) =>
        m.api.refreshToken(refresh),
      );
      localStorage.setItem("token", r.token);
      if (r.refresh) localStorage.setItem("refresh", r.refresh);
      if (r.user) localStorage.setItem("user", JSON.stringify(r.user));
      window.dispatchEvent(new Event("session-renouvelee"));
    } catch {
      // Renouvellement impossible : la prochaine requête fermera la session
      // proprement via `estErreurSession`.
    } finally {
      rafraichissementEnCours = false;
      planifierRenouvellement();
    }
  }, 55 * 60 * 1000);
}

/** Fermeture propre : vide le cache pour ne pas ré-afficher de données
 * belonging to the previous session, puis efface le jeton.
 */
function fermerSession(qc: QueryClient) {
  if (localStorage.getItem("token")) {
    localStorage.removeItem("token");
    localStorage.removeItem("user");
    qc.clear();
    // Un rechargement garantit que l'écran de connexion est affiché et
    // qu'aucune requête ne repart avec un jeton absent.
    if (!window.location.pathname.startsWith("/login")) {
      window.location.replace("/login");
    }
  }
}

const qc = new QueryClient({
  defaultOptions: {
    queries: {
      // Un refus d'autorisation ne se corrige pas en réessayant : on arrête.
      retry: (failureCount, error) => {
        if (estErreurSession(error)) return false;
        if (String(error ?? "").includes("refusées")) return false;
        return failureCount < 2;
      },
    },
    mutations: { retry: false },
  },
  queryCache: new QueryCache({
    onError: (e) => { if (estErreurSession(e)) fermerSession(qc); },
  }),
  mutationCache: new MutationCache({
    onError: (e) => {
      if (estErreurSession(e)) { fermerSession(qc); return; }
      toast.error(String(e));
    },
    onSuccess: () => {
      // Tout le système est dynamique : chaque écriture rafraîchit
      // dashboard, graphiques, stock, compta et notifications.
      [
        "stats", "ca", "marges", "dep-mens", "topP", "topC", "retard",
        "stock", "alertes", "mvts", "journal", "balance", "livre",
        "factures", "factures-all", "devis", "commandes", "depenses",
        "depenses-cat", "notifications", "bls", "avoirs", "clotures",
        "sante", "audit",
      ].forEach((k) => qc.invalidateQueries({ queryKey: [k] }));
    },
  }),
});

function Guard({ children }: { children: JSX.Element }) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" />;
  return children;
}

function RequirePage({ page, children }: { page: string; children: JSX.Element }) {
  const { user } = useAuth();
  if (!canAccess(user?.role, page)) return <Navigate to="/" />;
  return children;
}

/** Enveloppe une page chargée à la demande. */
function Charge({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<Chargement />}>{children}</Suspense>;
}

function Chargement() {
  return (
    <div className="flex items-center justify-center py-16 text-slate-400 text-sm">
      <Loader2 size={18} className="animate-spin mr-2" /> Chargement…
    </div>
  );
}

function Denied() {
  return (
    <div className="bg-white rounded-xl border p-8 text-center">
      <h1 className="font-bold text-lg mb-1">Accès refusé</h1>
      <p className="text-sm text-slate-500">Votre rôle ne permet pas d'accéder à cette page.</p>
    </div>
  );
}

const TITLES: Record<string, string> = {
  "/": "Tableau de bord",
  "/clients": "Clients",
  "/fournisseurs": "Fournisseurs",
  "/produits": "Quincaillerie (Articles)",
  "/devis": "Devis",
  "/factures": "Factures",
  "/avoirs": "Avoirs",
  "/achats": "Achats fournisseurs",
  "/livraisons": "Bons de livraison",
  "/caisse": "Caisse",
  "/paie": "Paie et RH",
  "/fidelite": "Fidélité et promotions",
  "/depenses": "Dépenses",
  "/relances": "Relances clients",
  "/utilisateurs": "Utilisateurs",
  "/stock": "Stock",
  "/depots": "Dépôts",
  "/audit": "Journal d'audit",
  "/incidents": "Journal d'incidents",
  "/comptabilite": "Comptabilité",
  "/parametres": "Paramètres",
  "/login": "Connexion",
  "/register": "Créer un compte",
};

function RouteTitle() {
  const { pathname } = useLocation();
  const label = TITLES[pathname] ?? "Gestion Quincaillerie";
  document.title = `${label} — Gestion Quincaillerie (XOF)`;
  document.querySelector('meta[name="description"]')?.setAttribute(
    "content",
    `${label} : gérez votre quincaillerie en Franc CFA (XOF) — articles, stock, ventes comptoir, factures, fournisseurs.`
  );
  return null;
}

export default function App() {
  useEffect(() => {
    loadPrefs();
    planifierRenouvellement();
  }, []);
  return (
    <QueryClientProvider client={qc}>
      <BrowserRouter>
        <RouteTitle />
        <Toaster />
        {!isTauriRuntime() && (
          <div style={{ background: "#b45309", color: "#fff", padding: "8px 16px", fontSize: 13, textAlign: "center" }}>
            Mode navigateur : les fonctions métier sont désactivées. Utilisez la fenêtre « Gestion Quincaillerie ».
          </div>
        )}
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route path="/register" element={<Register />} />
          <Route
            path="/"
            element={
              <Guard>
                <Suspense fallback={<Chargement />}>
                  <Layout />
                </Suspense>
              </Guard>
            }
          >
            <Route index element={<Charge><Dashboard /></Charge>} />
            <Route path="clients" element={<Charge><RequirePage page="/clients"><Clients /></RequirePage></Charge>} />
            <Route path="fournisseurs" element={<Charge><RequirePage page="/fournisseurs"><Fournisseurs /></RequirePage></Charge>} />
            <Route path="produits" element={<Charge><RequirePage page="/produits"><Produits /></RequirePage></Charge>} />
            <Route path="devis" element={<Charge><RequirePage page="/devis"><Devis /></RequirePage></Charge>} />
            <Route path="factures" element={<Charge><RequirePage page="/factures"><Factures /></RequirePage></Charge>} />
            <Route path="avoirs" element={<Charge><RequirePage page="/avoirs"><Avoirs /></RequirePage></Charge>} />
            <Route path="achats" element={<Charge><RequirePage page="/achats"><Achats /></RequirePage></Charge>} />
            <Route path="livraisons" element={<Charge><RequirePage page="/livraisons"><Livraisons /></RequirePage></Charge>} />
            <Route path="caisse" element={<Charge><RequirePage page="/caisse"><Caisse /></RequirePage></Charge>} />
            <Route path="paie" element={<Charge><RequirePage page="/paie"><Paie /></RequirePage></Charge>} />
            <Route path="fidelite" element={<Charge><RequirePage page="/fidelite"><Fidelite /></RequirePage></Charge>} />
            <Route path="depenses" element={<Charge><RequirePage page="/depenses"><Depenses /></RequirePage></Charge>} />
            <Route path="relances" element={<Charge><RequirePage page="/relances"><Relances /></RequirePage></Charge>} />
            <Route path="utilisateurs" element={<Charge><RequirePage page="/utilisateurs"><Utilisateurs /></RequirePage></Charge>} />
            <Route path="stock" element={<Charge><RequirePage page="/stock"><Stock /></RequirePage></Charge>} />
            <Route path="depots" element={<Charge><RequirePage page="/depots"><Depots /></RequirePage></Charge>} />
            <Route path="audit" element={<Charge><RequirePage page="/audit"><Audit /></RequirePage></Charge>} />
            <Route path="incidents" element={<Charge><RequirePage page="/incidents"><Incidents /></RequirePage></Charge>} />
            <Route path="comptabilite" element={<Charge><RequirePage page="/comptabilite"><Comptabilite /></RequirePage></Charge>} />
            <Route path="parametres" element={<Charge><RequirePage page="/parametres"><Parametres /></RequirePage></Charge>} />
            <Route path="refuse" element={<Charge><Denied /></Charge>} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
