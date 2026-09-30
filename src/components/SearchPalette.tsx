import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { FileText, Package, Receipt, Search, Users } from "lucide-react";
import { api } from "../services/api";
import { Modal } from "./ui";
import { fmtMoney } from "../utils/format";

type Hit = { kind: string; icon: any; title: string; sub: string; to: string };

function useDebounced(v: string, ms = 250) {
  const [d, setD] = useState(v);
  useEffect(() => {
    const t = setTimeout(() => setD(v), ms);
    return () => clearTimeout(t);
  }, [v, ms]);
  return d;
}

export default function SearchPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  const nav = useNavigate();
  const dq = useDebounced(q);

  useEffect(() => {
    if (!open) { setQ(""); setHits([]); return; }
  }, [open ]);

  useEffect(() => {
    if (!open || dq.trim().length < 2) { setHits([]); return; }
    (async () => {
      const query = dq.trim();
      const out: Hit[] = [];
      try {
        const [clients, produits, factures, devis] = await Promise.all([
          api.clientsSearch(query).catch(() => []),
          api.produitsSearch(query).catch(() => []),
          api.facturesList(1, 200).catch(() => null),
          api.devisList(1, 200).catch(() => null),
        ]);
        clients.slice(0, 5).forEach((c) =>
          out.push({ kind: "Client", icon: Users, title: `${c.nom} ${c.prenom ?? ""}`.trim(), sub: c.entreprise || c.ville || c.email || "", to: "/clients" }));
        produits.slice(0, 5).forEach((p) =>
          out.push({ kind: "Produit", icon: Package, title: p.designation, sub: `${p.reference} • ${fmtMoney(p.prix_vente_ht)}`, to: "/produits" }));
        const ql = query.toLowerCase();
        (factures?.data || [])
          .filter((f) => f.numero.toLowerCase().includes(ql) || (f.client_nom || "").toLowerCase().includes(ql))
          .slice(0, 5)
          .forEach((f) => out.push({ kind: "Facture", icon: Receipt, title: f.numero, sub: `${f.client_nom ?? ""} • ${fmtMoney(f.total_ttc)}`, to: "/factures" }));
        (devis?.data || [])
          .filter((d) => d.numero.toLowerCase().includes(ql) || (d.client_nom || "").toLowerCase().includes(ql))
          .slice(0, 5)
          .forEach((d) => out.push({ kind: "Devis", icon: FileText, title: d.numero, sub: `${d.client_nom ?? ""} • ${fmtMoney(d.total_ttc)}`, to: "/devis" }));
      } catch { /* hors-ligne partiel */ }
      setHits(out);
    })();
  }, [dq, open]);

  const grouped = useMemo(() => {
    const m = new Map<string, Hit[]>();
    hits.forEach((h) => { if (!m.has(h.kind)) m.set(h.kind, []); m.get(h.kind)!.push(h); });
    return [...m.entries()];
  }, [hits]);

  if (!open) return null;
  return (
    <Modal title="Recherche globale" onClose={onClose}>
      <div className="flex items-center gap-2 border rounded-lg px-3 py-2 mb-2">
        <Search size={16} className="text-slate-400" />
        <input autoFocus className="w-full text-sm outline-none" placeholder="Client, produit, facture, devis… (min. 2 lettres)"
          value={q} onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Escape") onClose(); }} />
      </div>
      {grouped.length === 0 && dq.trim().length >= 2 && (
        <p className="text-sm text-slate-400 text-center py-4">Aucun résultat.</p>
      )}
      {grouped.map(([kind, list]) => (
        <div key={kind} className="mb-2">
          <div className="text-[11px] uppercase text-slate-400 mb-1">{kind}s</div>
          {list.map((h, i) => (
            <button key={i} className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-slate-100 text-left"
              onClick={() => { nav(h.to); onClose(); }}>
              <h.icon size={15} className="text-slate-400 shrink-0" />
              <span className="font-medium text-sm truncate">{h.title}</span>
              <span className="text-xs text-slate-500 truncate ml-auto">{h.sub}</span>
            </button>
          ))}
        </div>
      ))}
    </Modal>
  );
}
