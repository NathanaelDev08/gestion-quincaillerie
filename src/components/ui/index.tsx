import React from "react";
import { createPortal } from "react-dom";
import { LucideIcon, X } from "lucide-react";

/** Infobulle affichée au survol, rendue en portail (jamais rognée) */
export function Tip({ label, children }: { label: string; children: React.ReactNode }) {
  const [pos, setPos] = React.useState<{ x: number; y: number } | null>(null);
  return (
    <span
      className="relative inline-flex"
      onMouseEnter={(e) => {
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
        setPos({ x: r.left + r.width / 2, y: r.bottom + 6 });
      }}
      onMouseLeave={() => setPos(null)}
    >
      {children}
      {pos &&
        createPortal(
          <span
            className="fixed z-[100] -translate-x-1/2 whitespace-nowrap rounded-md bg-slate-800 text-white text-[11px] px-2 py-0.5 pointer-events-none shadow-lg"
            style={{ left: pos.x, top: pos.y }}
          >
            {label}
          </span>,
          document.body
        )}
    </span>
  );
}

export function Button({ className = "", ...p }: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...p} className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-blue-700 text-white hover:bg-blue-800 disabled:opacity-50 text-sm font-medium ${className}`} />;
}
export function GhostButton({ className = "", ...p }: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...p} className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-sm ${className}`} />;
}
export function Input({ className = "", ...p }: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input {...p} className={`px-3 py-2 rounded-lg border border-slate-300 w-full text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 ${className}`} />;
}
export function Select({ className = "", ...p }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return <select {...p} className={`px-3 py-2 rounded-lg border border-slate-300 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 ${className}`} />;
}
export function Card({ className = "", ...p }: React.HTMLAttributes<HTMLDivElement>) {
  return <div {...p} className={`bg-white rounded-xl shadow-sm border border-slate-200 p-3 ${className}`} />;
}
export function Badge({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${className}`}>{children}</span>;
}
export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: React.ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
      <div className="min-w-0"><h1 className="text-lg font-bold truncate">{title}</h1>{subtitle && <p className="text-slate-500 text-xs">{subtitle}</p>}</div>
      <div className="flex gap-2 items-center flex-wrap">{actions}</div>
    </div>
  );
}
export function Table({ headers, children }: { headers: string[]; children: React.ReactNode }) {
  return (
    <div className="overflow-auto rounded-xl border border-slate-200 bg-white max-h-[62vh]">
      <table className="w-full text-sm">
        <thead className="sticky top-0"><tr className="bg-slate-50 text-left">{headers.map((h) => <th key={h} className="px-3 py-2 font-medium text-slate-600">{h}</th>)}</tr></thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}

/** Bouton icône simple avec infobulle */
export function IconBtn({ icon: Icon, title, onClick, tone = "slate", disabled }: {
  icon: LucideIcon; title: string; onClick?: () => void;
  tone?: "slate" | "blue" | "red" | "green" | "amber"; disabled?: boolean;
}) {
  const tones: Record<string, string> = {
    slate: "text-slate-500 hover:bg-slate-100 hover:text-slate-700",
    blue: "text-blue-600 hover:bg-blue-50",
    red: "text-red-500 hover:bg-red-50",
    green: "text-green-600 hover:bg-green-50",
    amber: "text-amber-600 hover:bg-amber-50",
  };
  return (
    <Tip label={title}>
      <button aria-label={title} disabled={disabled} onClick={onClick}
        className={`p-1.5 rounded-lg transition-colors disabled:opacity-40 ${tones[tone]}`}>
        <Icon size={16} />
      </button>
    </Tip>
  );
}

/** Fenêtre modale simple centrée (Échap pour fermer) */
export function Modal({ title, onClose, children, wide }: {
  title: string; onClose: () => void; children: React.ReactNode; wide?: boolean;
}) {
  React.useEffect(() => {
    const h = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onClose]);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" onClick={onClose}>
      <div className={`bg-white rounded-2xl shadow-xl w-full ${wide ? "max-w-3xl" : "max-w-lg"} max-h-[90vh] overflow-auto`}
        onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200">
          <h2 className="font-semibold">{title}</h2>
          <IconBtn icon={X} title="Fermer" onClick={onClose} />
        </div>
        <div className="p-4">{children}</div>
      </div>
    </div>
  );
}

/** Champ de formulaire avec vrai libellé */
export function Field({ label, children, className = "" }: {
  label: string; children: React.ReactNode; className?: string;
}) {
  return (
    <label className={`block text-sm ${className}`}>
      <span className="block mb-1 font-medium text-slate-700">{label}</span>
      {children}
    </label>
  );
}

/** Liste affichée en cartes responsives (remplace les tableaux) */
export function DataCards<T>({
  data, title, subtitle, fields, actions, empty = "Aucun élément à afficher",
}: {
  data: T[];
  title: (row: T) => React.ReactNode;
  subtitle?: (row: T) => React.ReactNode;
  fields: { label: string; value: (row: T) => React.ReactNode }[];
  actions?: (row: T) => React.ReactNode;
  empty?: string;
}) {
  if (!data || data.length === 0) {
    return <div className="bg-white rounded-xl border border-dashed border-slate-300 p-6 text-center text-sm text-slate-400">{empty}</div>;
  }
  return (
    <div className="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
      {data.map((row, i) => (
        <div key={i} className="bg-white rounded-xl border border-slate-200 px-2.5 py-2 shadow-sm hover:shadow transition-shadow">
          <div className="flex items-start justify-between gap-2 mb-1">
            <div className="min-w-0">
              <div className="font-semibold text-sm truncate">{title(row)}</div>
              {subtitle && <div className="text-[11px] text-slate-500 truncate">{subtitle(row)}</div>}
            </div>
            {actions && <div className="flex gap-0.5 shrink-0">{actions(row)}</div>}
          </div>
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1">
            {fields.map((f, j) => (
              <div key={j} className="min-w-0">
                <dt className="text-[10px] uppercase tracking-wide text-slate-400">{f.label}</dt>
                <dd className="text-[13px] font-medium truncate" title={typeof f.value(row) === "string" ? (f.value(row) as string) : undefined}>{f.value(row)}</dd>
              </div>
            ))}
          </dl>
        </div>
      ))}
    </div>
  );
}

import { ChevronLeft, ChevronRight, LayoutGrid, List as ListIcon, Table2 } from "lucide-react";

/** Mode d'affichage persistant (global à l'application) */
export type ViewMode = "cards" | "list" | "table";
const VIEW_KEY = "gc-view-mode";

export function useViewMode(): [ViewMode, (m: ViewMode) => void] {
  const [mode, setMode] = React.useState<ViewMode>(() =>
    (localStorage.getItem(VIEW_KEY) as ViewMode) || "cards"
  );
  const change = (m: ViewMode) => {
    localStorage.setItem(VIEW_KEY, m);
    setMode(m);
    window.dispatchEvent(new CustomEvent("gc-view-change", { detail: m }));
  };
  React.useEffect(() => {
    const h = (e: Event) => setMode((e as CustomEvent).detail as ViewMode);
    window.addEventListener("gc-view-change", h);
    return () => window.removeEventListener("gc-view-change", h);
  }, []);
  return [mode, change];
}

export function ViewToggle({ mode, onChange }: { mode: ViewMode; onChange: (m: ViewMode) => void }) {
  const btn = (m: ViewMode, Icon: LucideIcon, label: string) => (
    <Tip key={m} label={label}>
      <button onClick={() => onChange(m)}
        className={`p-1.5 rounded-lg transition-colors ${mode === m ? "bg-blue-100 text-blue-700" : "text-slate-400 hover:bg-slate-100"}`}>
        <Icon size={15} />
      </button>
    </Tip>
  );
  return (
    <div className="inline-flex gap-0.5 bg-white border border-slate-200 rounded-lg p-0.5">
      {btn("cards", LayoutGrid, "Affichage cartes")}
      {btn("list", ListIcon, "Affichage liste")}
      {btn("table", Table2, "Affichage tableau")}
    </div>
  );
}

type FieldDef<T> = { label: string; value: (row: T) => React.ReactNode };

/** Liste avec 3 affichages commutables : cartes, liste compacte, tableau */
export function DataView<T>({
  data, title, subtitle, fields, actions, empty = "Aucun élément à afficher",
  loading = false, error = null, onRetry, emptyAction,
}: {
  data: T[];
  title: (row: T) => React.ReactNode;
  subtitle?: (row: T) => React.ReactNode;
  fields: FieldDef<T>[];
  actions?: (row: T) => React.ReactNode;
  empty?: string;
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;
  emptyAction?: React.ReactNode;
}) {
  const [mode, setMode] = useViewMode();

  const toggle = (
    <div className="flex justify-end mb-2">
      <ViewToggle mode={mode} onChange={setMode} />
    </div>
  );

  if (loading) {
    return (<>
      {toggle}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {[0, 1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="bg-white rounded-xl border border-slate-200 p-3 animate-pulse">
            <div className="h-4 bg-slate-200 rounded w-2/3 mb-2" />
            <div className="h-3 bg-slate-100 rounded w-1/2 mb-3" />
            <div className="grid grid-cols-2 gap-2">
              <div className="h-3 bg-slate-100 rounded" />
              <div className="h-3 bg-slate-100 rounded" />
            </div>
          </div>
        ))}
      </div>
    </>);
  }

  if (error) {
    return (<>
      {toggle}
      <div className="bg-red-50 border border-red-200 rounded-xl p-4 text-sm text-red-700 flex items-center justify-between gap-2">
        <span>Impossible de charger les données. Vérifiez la connexion puis réessayez.</span>
        {onRetry && <button onClick={onRetry} className="px-3 py-1 rounded-lg bg-red-600 text-white text-xs shrink-0">Réessayer</button>}
      </div>
    </>);
  }

  if (!data || data.length === 0) {
    return (<>
      {toggle}
      <div className="bg-white rounded-xl border border-dashed border-slate-300 p-6 text-center">
        <p className="text-sm text-slate-400 mb-2">{empty}</p>
        {emptyAction}
      </div>
    </>);
  }

  if (mode === "table") {
    return (<>
      {toggle}
      <div className="overflow-auto rounded-xl border border-slate-200 bg-white max-h-[62vh]">
        <table className="w-full text-sm">
          <thead className="sticky top-0">
            <tr className="bg-slate-50 text-left">
              <th className="px-3 py-2 font-medium text-slate-600">Élément</th>
              {fields.map((f) => <th key={f.label} className="px-3 py-2 font-medium text-slate-600">{f.label}</th>)}
              {actions && <th className="px-3 py-2 font-medium text-slate-600">Actions</th>}
            </tr>
          </thead>
          <tbody>
            {data.map((row, i) => (
              <tr key={i} className="border-t hover:bg-slate-50">
                <td className="px-3 py-2">
                  <div className="font-medium">{title(row)}</div>
                  {subtitle && <div className="text-xs text-slate-500">{subtitle(row)}</div>}
                </td>
                {fields.map((f, j) => <td key={j} className="px-3 py-2">{f.value(row)}</td>)}
                {actions && <td className="px-3 py-2 whitespace-nowrap">{actions(row)}</td>}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>);
  }

  if (mode === "list") {
    return (<>
      {toggle}
      <div className="bg-white rounded-xl border border-slate-200 divide-y divide-slate-100">
        {data.map((row, i) => (
          <div key={i} className="flex items-center gap-3 px-2.5 py-1.5 hover:bg-slate-50">
            <div className="min-w-0 flex-1">
              <div className="font-medium truncate">{title(row)}</div>
              {subtitle && <div className="text-xs text-slate-500 truncate">{subtitle(row)}</div>}
            </div>
            <div className="hidden lg:flex items-center gap-4 text-sm">
              {fields.slice(0, 3).map((f, j) => (
                <span key={j} className="whitespace-nowrap"><span className="text-slate-400 text-xs">{f.label} : </span><b>{f.value(row)}</b></span>
              ))}
            </div>
            {actions && <div className="flex gap-0.5 shrink-0">{actions(row)}</div>}
          </div>
        ))}
      </div>
    </>);
  }

  return (<>
    {toggle}
    <DataCards data={data} title={title} subtitle={subtitle} fields={fields} actions={actions} empty={empty} />
  </>);
}

/** Pagination : « page X sur Y » + lignes par page */
export function Pagination({ page, perPage, total, onChange }: {
  page: number; perPage: number; total: number;
  onChange: (page: number, perPage: number) => void;
}) {
  const pages = Math.max(1, Math.ceil(total / perPage));
  const p = Math.min(Math.max(1, page), pages);
  const from = total === 0 ? 0 : (p - 1) * perPage + 1;
  const to = Math.min(total, p * perPage);
  return (
    <div className="flex items-center justify-between mt-2 text-sm text-slate-600">
      <span>{from}–{to} sur {total} élément{total > 1 ? "s" : ""}</span>
      <div className="flex items-center gap-1">
        <select className="border rounded px-1 py-0.5 text-xs" value={perPage} title="Éléments par page"
          onChange={(e) => onChange(1, +e.target.value)}>
          {[10, 20, 50, 100].map((n) => <option key={n} value={n}>{n} / page</option>)}
        </select>
        <button className="p-1.5 rounded hover:bg-slate-100 disabled:opacity-40" disabled={p <= 1}
          title="Page précédente" onClick={() => onChange(p - 1, perPage)}><ChevronLeft size={15} /></button>
        <span className="text-xs px-1">Page {p} / {pages}</span>
        <button className="p-1.5 rounded hover:bg-slate-100 disabled:opacity-40" disabled={p >= pages}
          title="Page suivante" onClick={() => onChange(p + 1, perPage)}><ChevronRight size={15} /></button>
      </div>
    </div>
  );
}

/** Découpe un tableau pour la pagination côté client */
export function paginate<T>(rows: T[], page: number, perPage: number): T[] {
  return rows.slice((page - 1) * perPage, page * perPage);
}

/** Barre d'onglets d'une page (un onglet = une fonctionnalité) */
export function Tabs<T extends string>({ tabs, active, onChange }: {
  tabs: { key: T; label: string; icon?: LucideIcon }[];
  active: T;
  onChange: (k: T) => void;
}) {
  return (
    <div className="flex gap-1 mb-3 border-b border-slate-200 overflow-auto">
      {tabs.map((t) => (
        <button key={t.key} onClick={() => onChange(t.key)}
          className={`flex items-center gap-1.5 px-3 py-2 text-sm font-medium whitespace-nowrap border-b-2 -mb-px transition-colors ${active === t.key ? "border-blue-700 text-blue-700" : "border-transparent text-slate-500 hover:text-slate-700"}`}>
          {t.icon && <t.icon size={15} />} {t.label}
        </button>
      ))}
    </div>
  );
}

/** Suppression avec confirmation native */
export function confirmDelete(label: string): boolean {
  return window.confirm(`Supprimer « ${label} » ?`);
}

/* ---------- Toasts (notifications non bloquantes) ---------- */
type ToastItem = { id: number; kind: "success" | "error" | "info"; msg: string };
const toastListeners = new Set<(t: ToastItem[]) => void>();
let toastStack: ToastItem[] = [];
let toastSeq = 1;
function emitToasts() { toastListeners.forEach((l) => l([...toastStack])); }
function pushToast(kind: ToastItem["kind"], msg: string) {
  const id = toastSeq++;
  toastStack.push({ id, kind, msg });
  emitToasts();
  setTimeout(() => { toastStack = toastStack.filter((t) => t.id !== id); emitToasts(); }, 4500);
}
export const toast = {
  success: (msg: string) => pushToast("success", msg),
  error: (msg: string) => pushToast("error", msg),
  info: (msg: string) => pushToast("info", msg),
};
export function Toaster() {
  const [list, setList] = React.useState<ToastItem[]>(toastStack);
  React.useEffect(() => {
    toastListeners.add(setList);
    return () => { toastListeners.delete(setList); };
  }, []);
  const styles: Record<string, string> = {
    success: "bg-green-700",
    error: "bg-red-700",
    info: "bg-slate-800",
  };
  return (
    <div className="fixed bottom-4 right-4 z-[70] space-y-2 max-w-sm">
      {list.map((t) => (
        <div key={t.id} className={`${styles[t.kind]} text-white text-sm px-4 py-2.5 rounded-xl shadow-lg`}>
          {t.msg}
        </div>
      ))}
    </div>
  );
}
