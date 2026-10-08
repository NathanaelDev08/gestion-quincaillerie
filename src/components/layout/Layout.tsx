import { useEffect, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { LayoutDashboard, Users, Truck, Package, FileText, Receipt, Warehouse, BookOpen, Settings, LogOut, Store, Undo2, ShoppingCart, Wallet, BellRing, ShieldCheck, Search, Menu, X, ClipboardList, ShoppingBag, Contact, Gift, History, ArrowLeftRight, AlertTriangle } from "lucide-react";
import { useAuth } from "../../stores/useAuth";
import { canAccess } from "../../auth/permissions";
import SearchPalette from "../SearchPalette";
import NotificationsCenter from "../NotificationsCenter";
import { api } from "../../services/api";
import { useIsFetching, useQuery } from "@tanstack/react-query";

const links = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, grp: "pilotage" },
  { to: "/clients", label: "Clients", icon: Users, grp: "ventes" },
  { to: "/devis", label: "Devis", icon: FileText, grp: "ventes" },
  { to: "/factures", label: "Factures", icon: Receipt, grp: "ventes" },
  { to: "/avoirs", label: "Avoirs", icon: Undo2, grp: "ventes" },
  { to: "/livraisons", label: "Livraisons", icon: ClipboardList, grp: "ventes" },
  { to: "/caisse", label: "Caisse", icon: ShoppingBag, grp: "ventes" },
  { to: "/relances", label: "Relances", icon: BellRing, grp: "ventes" },
  { to: "/fidelite", label: "Fidélité", icon: Gift, grp: "ventes" },
  { to: "/fournisseurs", label: "Fournisseurs", icon: Truck, grp: "achats" },
  { to: "/achats", label: "Achats", icon: ShoppingCart, grp: "achats" },
  { to: "/produits", label: "Quincaillerie", icon: Package, grp: "achats" },
  { to: "/stock", label: "Stock", icon: Warehouse, grp: "achats" },
  { to: "/depots", label: "Dépôts", icon: ArrowLeftRight, grp: "achats" },
  { to: "/depenses", label: "Dépenses", icon: Wallet, grp: "finance" },
  { to: "/comptabilite", label: "Comptabilité", icon: BookOpen, grp: "finance" },
  { to: "/paie", label: "Paie & RH", icon: Contact, grp: "finance" },
  { to: "/utilisateurs", label: "Utilisateurs", icon: ShieldCheck, grp: "systeme" },
  { to: "/audit", label: "Journal d'audit", icon: History, grp: "systeme" },
  { to: "/incidents", label: "Incidents", icon: AlertTriangle, grp: "systeme" },
  { to: "/parametres", label: "Paramètres", icon: Settings, grp: "systeme" },
];

const GROUP_LABELS: Record<string, string> = {
  pilotage: "Pilotage",
  ventes: "Ventes",
  achats: "Achats & Stock",
  finance: "Finance",
  systeme: "Système",
};

export default function Layout() {
  const { user, logout, refresh } = useAuth();
  const nav = useNavigate();
  const [searchOpen, setSearchOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [notifOpen, setNotifOpen] = useState(false);
  const visible = links.filter((l) => canAccess(user?.role, l.to));
  const groupOrder = ["pilotage", "ventes", "achats", "finance", "systeme"];
  const grouped = groupOrder
    .map((gk) => ({ grp: gk, items: visible.filter((l) => l.grp === gk) }))
    .filter((g) => g.items.length > 0);
  const { data: notifs } = useQuery({ queryKey: ["notifications"], queryFn: api.notificationsList, refetchInterval: 60000 });
  const fetching = useIsFetching();

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearchOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  const sidebarContent = (
    <div className="flex-1 overflow-y-auto px-2 pb-4 space-y-4">
      {grouped.map((g) => (
        <div key={g.grp}>
          <p className="px-3 pt-3 pb-1 text-[11px] font-semibold uppercase tracking-wide text-slate-400">
            {GROUP_LABELS[g.grp] ?? g.grp}
          </p>
          <div className="space-y-0.5">
            {g.items.map((l) => (
              <NavLink
                key={l.to}
                to={l.to}
                end={l.to === "/"}
                onClick={() => setMenuOpen(false)}
                className={({ isActive }) =>
                  `flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors ${
                    isActive
                      ? "bg-blue-600 text-white font-medium shadow-sm"
                      : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
                  }`
                }
              >
                <l.icon size={17} className="shrink-0" />
                <span className="truncate">{l.label}</span>
              </NavLink>
            ))}
          </div>
        </div>
      ))}
    </div>
  );

  return (
    <div className="flex h-screen bg-slate-100">
      {/* Sidebar desktop */}
      <aside className="hidden md:flex w-60 shrink-0 bg-white border-r border-slate-200 flex-col">
        <div className="flex items-center gap-2 px-4 h-[56px] border-b border-slate-200 font-bold text-blue-800 shrink-0">
          <span className="bg-blue-700 text-white rounded-lg p-1.5"><Store size={16} /></span>
          <span className="text-[15px]">Quincaillerie</span>
        </div>
        {sidebarContent}
        <div className="p-3 border-t border-slate-200 shrink-0">
          <div className="flex items-center gap-2 text-xs text-slate-600">
            <span className="w-8 h-8 rounded-full bg-blue-700 text-white flex items-center justify-center text-sm font-bold shrink-0">
              {(user?.full_name || user?.username || "?").trim().charAt(0).toUpperCase()}
            </span>
            <span className="truncate font-medium">{user?.full_name}</span>
          </div>
        </div>
      </aside>

      {/* Colonne principale */}
      <div className="flex-1 flex flex-col min-w-0">
        <header className="bg-white border-b border-slate-200 px-3 flex items-center gap-2 h-[56px] shrink-0">
          <button className="md:hidden p-1.5 rounded-lg hover:bg-slate-100" onClick={() => setMenuOpen(true)} title="Menu">
            <Menu size={18} />
          </button>
          <button
            onClick={() => setSearchOpen(true)}
            title="Rechercher (Ctrl+K)"
            className="flex items-center gap-2 px-3 py-1.5 rounded-xl border border-slate-200 text-slate-400 hover:bg-slate-50 text-sm w-48 sm:w-64"
          >
            <Search size={15} />
            <span className="hidden sm:inline">Rechercher…</span>
            <kbd className="hidden sm:inline ml-auto text-[10px] bg-slate-100 rounded px-1">Ctrl+K</kbd>
          </button>
          <div className="ml-auto flex items-center gap-1.5 shrink-0">
            <button className="relative p-2 rounded-xl hover:bg-slate-100" title="Notifications" onClick={() => setNotifOpen(true)}>
              <BellRing size={17} className={(notifs?.length || 0) > 0 ? "text-amber-600" : "text-slate-500"} />
              {(notifs?.length || 0) > 0 && (
                <span className="absolute top-0.5 right-0.5 bg-red-600 text-white text-[9px] rounded-full min-w-[15px] h-[15px] flex items-center justify-center px-0.5">
                  {notifs!.length}
                </span>
              )}
            </button>
            <button
              onClick={async () => {
                // Déconnexion réelle : la session serveur est révoquée, sans
                // quoi le jeton de rafraîchissement resterait valable 7 jours.
                try { await api.logout(refresh || undefined); } catch { /* local quand même */ }
                logout();
                nav("/login");
              }}
              title="Déconnexion"
              className="p-2 rounded-xl hover:bg-slate-100 hover:text-red-600 text-slate-500"
            >
              <LogOut size={16} />
            </button>
          </div>
        </header>

        <main className="flex-1 overflow-auto p-3 sm:p-5 min-w-0 relative">
          {fetching > 0 && (
            <div className="absolute top-0 left-0 right-0 h-0.5 bg-blue-100 overflow-hidden">
              <div className="h-full w-1/3 bg-blue-600 rounded animate-[fetchslide_1s_ease-in-out_infinite]" />
            </div>
          )}
          <div className="max-w-[1400px] mx-auto"><Outlet /></div>
        </main>
      </div>

      {/* Tiroir mobile = sidebar */}
      {menuOpen && (
        <div className="fixed inset-0 z-40 md:hidden">
          <div className="absolute inset-0 bg-black/40" onClick={() => setMenuOpen(false)} />
          <aside className="absolute left-0 top-0 bottom-0 w-64 bg-white flex flex-col shadow-xl">
            <div className="flex items-center gap-2 px-3 h-[56px] border-b border-slate-200 font-bold text-blue-800 text-sm shrink-0">
              <Store size={16} /> Quincaillerie
              <button className="ml-auto p-1 rounded hover:bg-slate-100" onClick={() => setMenuOpen(false)} title="Fermer">
                <X size={16} />
              </button>
            </div>
            {sidebarContent}
          </aside>
        </div>
      )}

      <SearchPalette open={searchOpen} onClose={() => setSearchOpen(false)} />
      <NotificationsCenter open={notifOpen} onClose={() => setNotifOpen(false)} />
    </div>
  );
}
