import { BrowserRouter, Routes, Route, Navigate, useLocation } from "react-router-dom";
import { QueryClient, QueryClientProvider, MutationCache } from "@tanstack/react-query";
import Layout from "./components/layout/Layout";
import Dashboard from "./pages/Dashboard";
import Clients from "./pages/Clients";
import Fournisseurs from "./pages/Fournisseurs";
import Produits from "./pages/Produits";
import Devis from "./pages/Devis";
import Factures from "./pages/Factures";
import Avoirs from "./pages/Avoirs";
import Achats from "./pages/Achats";
import Livraisons from "./pages/Livraisons";
import Caisse from "./pages/Caisse";
import Paie from "./pages/Paie";
import Fidelite from "./pages/Fidelite";
import Depenses from "./pages/Depenses";
import Relances from "./pages/Relances";
import Utilisateurs from "./pages/Utilisateurs";
import Stock from "./pages/Stock";
import Comptabilite from "./pages/Comptabilite";
import Parametres from "./pages/Parametres";
import Login, { Register } from "./pages/Login";
import { useAuth } from "./stores/useAuth";
import { isTauriRuntime } from "./services/api";
import { canAccess } from "./auth/permissions";
import { Toaster, toast } from "./components/ui";
import { loadPrefs } from "./stores/prefs";
import { useEffect } from "react";

const qc = new QueryClient({
  mutationCache: new MutationCache({
    onError: (e) => toast.error(String(e)),
    onSuccess: () => {
      // Tout le système est dynamique : chaque écriture rafraîchit
      // dashboard, graphiques, stock, compta et notifications.
      [
        "stats", "ca", "marges", "dep-mens", "topP", "topC", "retard",
        "stock", "alertes", "mvts", "journal", "balance", "livre",
        "factures", "factures-all", "devis", "commandes", "depenses",
        "depenses-cat", "notifications", "bls", "avoirs", "clotures",
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
  "/produits": "Articles Quincaillerie",
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
  useEffect(() => { loadPrefs(); }, []);
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
          <Route path="/" element={<Guard><Layout /></Guard>}>
            <Route index element={<Dashboard />} />
            <Route path="clients" element={<RequirePage page="/clients"><Clients /></RequirePage>} />
            <Route path="fournisseurs" element={<RequirePage page="/fournisseurs"><Fournisseurs /></RequirePage>} />
            <Route path="produits" element={<RequirePage page="/produits"><Produits /></RequirePage>} />
            <Route path="devis" element={<RequirePage page="/devis"><Devis /></RequirePage>} />
            <Route path="factures" element={<RequirePage page="/factures"><Factures /></RequirePage>} />
            <Route path="avoirs" element={<RequirePage page="/avoirs"><Avoirs /></RequirePage>} />
            <Route path="achats" element={<RequirePage page="/achats"><Achats /></RequirePage>} />
            <Route path="livraisons" element={<RequirePage page="/livraisons"><Livraisons /></RequirePage>} />
            <Route path="caisse" element={<RequirePage page="/caisse"><Caisse /></RequirePage>} />
            <Route path="paie" element={<RequirePage page="/paie"><Paie /></RequirePage>} />
            <Route path="fidelite" element={<RequirePage page="/fidelite"><Fidelite /></RequirePage>} />
            <Route path="depenses" element={<RequirePage page="/depenses"><Depenses /></RequirePage>} />
            <Route path="relances" element={<RequirePage page="/relances"><Relances /></RequirePage>} />
            <Route path="utilisateurs" element={<RequirePage page="/utilisateurs"><Utilisateurs /></RequirePage>} />
            <Route path="stock" element={<RequirePage page="/stock"><Stock /></RequirePage>} />
            <Route path="comptabilite" element={<RequirePage page="/comptabilite"><Comptabilite /></RequirePage>} />
            <Route path="parametres" element={<RequirePage page="/parametres"><Parametres /></RequirePage>} />
            <Route path="refuse" element={<Denied />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
