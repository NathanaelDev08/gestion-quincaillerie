import { useEffect, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { LayoutDashboard, Users, Truck, Package, FileText, Receipt, Warehouse, BookOpen, Settings, LogOut, Store, Undo2, ShoppingCart, Wallet, BellRing, ShieldCheck, Search, Menu, X, ClipboardList, ShoppingBag, Contact, Gift } from "lucide-react";
import { useAuth } from "../../stores/useAuth";
import { canAccess } from "../../auth/permissions";
import { Tip } from "../ui";
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
  { to: "/depenses", label: "Dépenses", icon: Wallet, grp: "finance" },
  { to: "/comptabilite", label: "Comptabilité", icon: BookOpen, grp: "finance" },
  { to: "/paie", label: "Paie & RH", icon: Contact, grp: "finance" },
  { to: "/utilisateurs", label: "Utilisateurs", icon: ShieldCheck, grp: "systeme" },
  { to: "/parametres", label: "Paramètres", icon: Settings, grp: "systeme" },
];

export default function Layout() {
  const { user, logout } = useAuth();
  const nav = useNavigate();
  const [searchOpen, setSearchOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [notifOpen, setNotifOpen] = useState(false);
  const visible = links.filter((l) => canAccess(user?.role, l.to));
  const groupOrder = ["pilotage", "ventes", "achats", "finance", "systeme"];
  const grouped = groupOrder
    .map((gk) => visible.filter((l) => l.grp === gk))
    .filter((items) => items.length > 0);
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

  const mobileList = (
    <div className="p-2 space-y-0.5 overflow-auto">
      {visible.map((l) => (
        <NavLink key={l.to} to={l.to} end={l.to === "/"} onClick={() => setMenuOpen(false)}
          className={({ isActive }) => `flex items-center gap-2 px-3 py-2 rounded-lg text-sm ${isActive ? "bg-blue-50 text-blue-700 font-medium" : "text-slate-600"}`}>
          <l.icon size={16} /> {l.label}
        </NavLink>
      ))}
    </div>
  );

  return (
    <div className="flex flex-col h-screen bg-slate-100">
      {/* Barre de navigation horizontale */}
      <header className="bg-white border-b border-slate-200 px-3 flex items-center gap-2 h-[52px] shrink-0 relative z-30">
        <button className="md:hidden p-1.5 rounded-lg hover:bg-slate-100" onClick={() => setMenuOpen(true)} title="Menu">
          <Menu size={18} />
        </button>
        <div className="flex items-center gap-2 font-bold text-blue-800 mr-1 shrink-0">
          <span className="bg-blue-700 text-white rounded-lg p-1.5"><Store size={15} /></span>
          <span className="hidden lg:inline text-[15px]">Quincaillerie</span>
        </div>
        <nav className="hidden md:flex items-center gap-1.5 flex-1 min-w-0 justify-center overflow-x-auto no-scrollbar">
          {grouped.map((items) => (
            <span key={items[0].grp} className="flex items-center gap-px bg-slate-100/80 rounded-xl px-1 py-0.5 shrink-0">
              {items.map((l) => (
                <Tip key={l.to} label={l.label}>
                  <NavLink to={l.to} end={l.to === "/"}
                    className={({ isActive }) => `block p-[7px] rounded-lg transition-all shrink-0 ${isActive ? "bg-blue-600 text-white shadow-sm" : "text-slate-500 hover:bg-white hover:text-slate-800 hover:shadow-sm"}`}>
                    <l.icon size={18} />
                  </NavLink>
                </Tip>
              ))}
            </span>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-1.5 shrink-0">
          <button onClick={() => setSearchOpen(true)} title="Rechercher (Ctrl+K)"
            className="flex items-center gap-1.5 p-2 rounded-xl border border-slate-200 text-slate-400 hover:bg-slate-50">
            <Search size={16} />
          </button>
          <button className="relative p-2 rounded-xl hover:bg-slate-100" title="Notifications" onClick={() => setNotifOpen(true)}>
            <BellRing size={17} className={(notifs?.length || 0) > 0 ? "text-amber-600" : "text-slate-500"} />
            {(notifs?.length || 0) > 0 && (
              <span className="absolute top-0.5 right-0.5 bg-red-600 text-white text-[9px] rounded-full min-w-[15px] h-[15px] flex items-center justify-center px-0.5">
                {notifs!.length}
              </span>
            )}
          </button>
          <span className="hidden sm:flex items-center gap-1.5 text-xs font-medium text-slate-600 bg-slate-100 rounded-full pl-1 pr-2.5 py-1 max-w-[160px]" title={user?.full_name}>
            <span className="w-6 h-6 rounded-full bg-blue-700 text-white flex items-center justify-center text-[10px] font-bold shrink-0">
              {(user?.full_name || user?.username || "?").trim().charAt(0).toUpperCase()}
            </span>
            <span className="truncate">{user?.full_name}</span>
          </span>
          <button onClick={() => { logout(); nav("/login"); }} title="Déconnexion" className="p-2 rounded-xl hover:bg-slate-100 hover:text-red-600 text-slate-500">
            <LogOut size={16} />
          </button>
        </div>
      </header>

      {/* Tiroir mobile */}
      {menuOpen && (
        <div className="fixed inset-0 z-40 md:hidden">
          <div className="absolute inset-0 bg-black/40" onClick={() => setMenuOpen(false)} />
          <aside className="absolute left-0 top-0 bottom-0 w-64 bg-white flex flex-col shadow-xl">
            <div className="flex items-center gap-2 px-3 py-2 border-b border-slate-200 font-bold text-blue-800 text-sm">
              <Store size={16} /> Quincaillerie
              <button className="ml-auto p-1 rounded hover:bg-slate-100" onClick={() => setMenuOpen(false)} title="Fermer">
                <X size={16} />
              </button>
            </div>
            <div className="flex-1 overflow-auto">{mobileList}</div>
          </aside>
        </div>
      )}

      <main className="flex-1 overflow-auto p-3 sm:p-5 min-w-0 relative">
        {fetching > 0 && (
          <div className="absolute top-0 left-0 right-0 h-0.5 bg-blue-100 overflow-hidden">
            <div className="h-full w-1/3 bg-blue-600 rounded animate-[fetchslide_1s_ease-in-out_infinite]" />
          </div>
        )}
        <div className="max-w-[1400px] mx-auto"><Outlet /></div>
      </main>
      <SearchPalette open={searchOpen} onClose={() => setSearchOpen(false)} />
      <NotificationsCenter open={notifOpen} onClose={() => setNotifOpen(false)} />
    </div>
  );
}
