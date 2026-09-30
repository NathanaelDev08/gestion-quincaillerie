import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Boxes, Check, ClipboardCheck, History, ListOrdered } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Badge, DataView, Modal, IconBtn, Pagination, paginate, Tabs, toast } from "../components/ui";
import { fmtMoney, TYPES_MOUVEMENT } from "../utils/format";

type Tab = "etat" | "mouvements" | "inventaire";

export default function Stock() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>("etat");
  const [ajust, setAjust] = useState({ produit_id: "", quantite: 0, motif: "" });
  const [inv, setInv] = useState<Record<string, number>>({});
  const [histId, setHistId] = useState<string | null>(null);
  const [pageS, setPageS] = useState(1);
  const [pageM, setPageM] = useState(1);
  const perPage = 10;
  const { data: stock } = useQuery({ queryKey: ["stock"], queryFn: api.stockList, refetchInterval: 30000 });
  const { data: alertes } = useQuery({ queryKey: ["alertes"], queryFn: api.stockAlertes, refetchInterval: 30000 });
  const { data: mvts } = useQuery({ queryKey: ["mvts"], queryFn: api.stockMouvements, refetchInterval: 30000 });
  const { data: hist } = useQuery({
    queryKey: ["mvt-produit", histId],
    queryFn: () => api.stockMouvementsProduit(histId!),
    enabled: !!histId,
  });
  const histNom = (stock || []).find((p) => p.id === histId)?.designation ?? "";

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ["stock"] });
    qc.invalidateQueries({ queryKey: ["alertes"] });
    qc.invalidateQueries({ queryKey: ["mvts"] });
  };
  const ajuster = useMutation({
    mutationFn: () => api.ajusterStock(ajust.produit_id, ajust.quantite, ajust.motif || "Ajustement manuel"),
    onSuccess: () => { refresh(); setAjust({ produit_id: "", quantite: 0, motif: "" }); toast.success("Stock ajusté"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const inventaire = useMutation({
    mutationFn: () => api.stockInventaire(Object.entries(inv).map(([id, q]) => [id, q] as [string, number])),
    onSuccess: () => { refresh(); setInv({}); toast.success("Inventaire enregistré"); },
    onError: (e: any) => toast.error(String(e)),
  });

  return (
    <div>
      <PageHeader title="Stock" subtitle={`${alertes?.length || 0} alertes seuil`} />
      <Tabs<Tab>
        active={tab} onChange={setTab}
        tabs={[
          { key: "etat", label: "État du stock", icon: Boxes },
          { key: "mouvements", label: "Mouvements", icon: History },
          { key: "inventaire", label: `Inventaire${Object.keys(inv).length ? ` (${Object.keys(inv).length})` : ""}`, icon: ClipboardCheck },
        ]}
      />

      {tab === "etat" && (
        <div className="space-y-4">
          <Card>
            <h3 className="font-semibold mb-2">Ajustement rapide (entrée/sortie)</h3>
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-2">
              <select className="px-3 py-2 border rounded-lg text-sm" value={ajust.produit_id} onChange={(e) => setAjust({ ...ajust, produit_id: e.target.value })}>
                <option value="">-- Produit --</option>
                {(stock || []).map((p) => <option key={p.id} value={p.id}>{p.designation} ({p.stock})</option>)}
              </select>
              <Input type="number" placeholder="+/- quantité" value={ajust.quantite} onChange={(e) => setAjust({ ...ajust, quantite: +e.target.value })} />
              <Input placeholder="Motif" value={ajust.motif} onChange={(e) => setAjust({ ...ajust, motif: e.target.value })} />
              <Button onClick={() => ajuster.mutate()} disabled={!ajust.produit_id || !ajust.quantite}><Check size={15} /> Ajuster</Button>
            </div>
          </Card>
          <Card>
            <DataView
              data={paginate(stock || [], pageS, perPage)}
              empty="Aucun produit en stock."
              title={(p) => p.designation}
              subtitle={(p) => `Valeur d'achat : ${fmtMoney(p.stock * p.prix_achat_ht)}`}
              fields={[
                {
                  label: "Stock actuel", value: (p) => (
                    <Badge className={p.stock <= p.stock_alerte ? "bg-red-100 text-red-700" : "bg-green-100 text-green-700"}>{p.stock}</Badge>
                  ),
                },
                { label: "Seuil d'alerte", value: (p) => `${p.stock_alerte}` },
              ]}
              actions={(p) => (
                <IconBtn icon={History} title="Historique des mouvements" tone="blue" onClick={() => setHistId(p.id)} />
              )}
            />
            <Pagination page={pageS} perPage={perPage} total={(stock || []).length}
              onChange={(p) => setPageS(p)} />
          </Card>
        </div>
      )}

      {tab === "mouvements" && (
        <Card>
          <DataView
            data={paginate(mvts || [], pageM, perPage)}
            empty="Aucun mouvement."
            title={(m) => `${m.quantite > 0 ? "+" : ""}${m.quantite} — ${m.designation ?? "—"}`}
            subtitle={(m) => `${TYPES_MOUVEMENT[m.type] ?? m.type}${m.document_ref ? ` • ${m.document_ref}` : ""}${m.motif ? ` • ${m.motif}` : ""}`}
            fields={[
              { label: "Stock avant", value: (m) => `${m.stock_avant}` },
              { label: "Stock après", value: (m) => `${m.stock_apres}` },
            ]}
          />
          <Pagination page={pageM} perPage={perPage} total={(mvts || []).length}
            onChange={(p) => setPageM(p)} />
        </Card>
      )}

      {tab === "inventaire" && (
        <Card>
          <h3 className="font-semibold mb-1">Comptage d'inventaire</h3>
          <p className="text-xs text-slate-500 mb-2">Saisissez les quantités comptées : les écarts génèrent des mouvements d'inventaire.</p>
          <DataView
            data={paginate(stock || [], pageS, perPage)}
            empty="Aucun produit."
            title={(p) => p.designation}
            subtitle={(p) => `Stock théorique : ${p.stock}`}
            fields={[
              {
                label: "Quantité comptée", value: (p) => (
                  <input type="number" className="w-24 px-2 py-1 border rounded text-sm" placeholder="—"
                    value={inv[p.id] ?? ""} onChange={(e) => setInv({ ...inv, [p.id]: +e.target.value })} />
                ),
              },
              {
                label: "Écart", value: (p) => {
                  const c = inv[p.id];
                  if (c === undefined || c === null || c === ("" as any)) return "-";
                  const d = c - p.stock;
                  return <b className={d === 0 ? "text-green-600" : "text-red-600"}>{d > 0 ? `+${d}` : d}</b>;
                },
              },
            ]}
          />
          <div className="flex items-center gap-2 mt-2">
            <Button onClick={() => inventaire.mutate()} disabled={Object.keys(inv).length === 0}>
              <ClipboardCheck size={15} /> Valider l'inventaire ({Object.keys(inv).length})
            </Button>
            <Pagination page={pageS} perPage={perPage} total={(stock || []).length}
              onChange={(p) => setPageS(p)} />
          </div>
        </Card>
      )}

      {histId && (
        <Modal title={`Historique — ${histNom}`} onClose={() => setHistId(null)} wide>
          <DataView
            data={hist || []}
            empty="Aucun mouvement pour cet article."
            title={(m) => `${m.quantite > 0 ? "+" : ""}${m.quantite}`}
            subtitle={(m) => `${TYPES_MOUVEMENT[m.type] ?? m.type}${m.document_ref ? ` • ${m.document_ref}` : ""}`}
            fields={[
              { label: "Stock avant", value: (m) => `${m.stock_avant}` },
              { label: "Stock après", value: (m) => `${m.stock_apres}` },
              { label: "Motif", value: (m) => m.motif || "-" },
            ]}
          />
        </Modal>
      )}
    </div>
  );
}
