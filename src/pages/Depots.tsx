import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowLeftRight, Plus, Warehouse } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Select, DataView, Tabs, toast } from "../components/ui";
import { fmtMoney } from "../utils/format";

export default function Depots() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"depots" | "stock" | "mouvements">("depots");
  const [depotId, setDepotId] = useState("");
  const [q, setQ] = useState("");
  const [nom, setNom] = useState("");
  const [adresse, setAdresse] = useState("");
  const [dest, setDest] = useState("");
  const [qte, setQte] = useState<Record<string, number>>({});

  const { data: depots } = useQuery({ queryKey: ["depots"], queryFn: api.depotsList });
  const { data: valeurs } = useQuery({ queryKey: ["depots-valeur"], queryFn: api.depotsValeur });
  const courant = depotId || (depots || [])[0]?.id || "";
  const { data: stock } = useQuery({
    queryKey: ["depot-stock", courant, q],
    queryFn: () => api.depotStock(courant, q),
    enabled: !!courant && tab === "stock",
  });
  const { data: mouvements } = useQuery({
    queryKey: ["depot-mvt", courant],
    queryFn: () => api.depotMouvements(courant),
    enabled: !!courant && tab === "mouvements",
  });

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ["depots"] });
    qc.invalidateQueries({ queryKey: ["depots-valeur"] });
    qc.invalidateQueries({ queryKey: ["depot-stock"] });
    qc.invalidateQueries({ queryKey: ["depot-mvt"] });
    qc.invalidateQueries({ queryKey: ["audit"] });
  };

  const createDepot = useMutation({
    mutationFn: () => api.depotsCreate({ nom, adresse }),
    onSuccess: () => {
      refresh();
      setNom("");
      setAdresse("");
      toast.success("Dépôt créé");
    },
    onError: (e: any) => toast.error(String(e)),
  });

  const transferer = useMutation({
    mutationFn: (row: any) =>
      api.depotTransferer({
        produit_id: row.produit_id,
        depot_origine: courant,
        depot_destination: dest,
        quantite: +qte[row.produit_id] || 0,
      }),
    onSuccess: () => {
      refresh();
      setQte({});
      toast.success("Transfert enregistré");
    },
    onError: (e: any) => toast.error(String(e)),
  });

  return (
    <div>
      <PageHeader title="Dépôts & stock multi-sites" subtitle="Répartis ton stock quincaillerie entre dépôt principal, magasin, rayon…" />

      <div className="grid grid-cols-1 md:grid-cols-3 gap-2.5 mb-3">
        {(valeurs || []).map((d) => (
          <Card key={d.id} className="!p-3">
            <div className="flex items-center gap-2 mb-1">
              <Warehouse size={15} className="text-blue-700" />
              <span className="font-semibold text-sm">{d.nom}</span>
            </div>
            <div className="text-lg font-bold text-blue-800">{fmtMoney(d.valeur)}</div>
            <div className="text-[11px] text-slate-400">{d.nb} référence(s) stockée(s)</div>
          </Card>
        ))}
      </div>

      <Tabs<"depots" | "stock" | "mouvements">
        active={tab} onChange={setTab}
        tabs={[
          { key: "depots", label: "Dépôts", icon: Warehouse },
          { key: "stock", label: "Stock & transferts", icon: ArrowLeftRight },
          { key: "mouvements", label: "Mouvements", icon: Plus },
        ]}
      />

      {tab === "depots" && (
        <Card>
          <h3 className="font-semibold mb-2">Nouveau dépôt</h3>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-3 max-w-2xl">
            <Input placeholder="Nom du dépôt *" value={nom} onChange={(e) => setNom(e.target.value)} />
            <Input placeholder="Adresse" value={adresse} onChange={(e) => setAdresse(e.target.value)} />
            <Button onClick={() => createDepot.mutate()} disabled={!nom.trim()}><Plus size={15} /> Créer</Button>
          </div>
          <DataView
            data={depots || []}
            empty="Aucun dépôt."
            title={(d) => d.nom}
            subtitle={(d) => d.adresse || "—"}
            fields={[
              { label: "ID", value: (d) => d.id },
              { label: "Statut", value: (d) => (d.actif ? "Actif" : "Inactif") },
            ]}
            actions={(d) => (
              <button className="text-xs text-blue-700 underline" onClick={() => { setDepotId(d.id); setTab("stock"); }}>
                Consulter
              </button>
            )}
          />
        </Card>
      )}

      {tab === "stock" && (
        <Card>
          <div className="flex gap-2 mb-2 flex-wrap items-center">
            <Select value={courant} onChange={(e) => setDepotId(e.target.value)} className="max-w-[220px]">
              {(depots || []).map((d) => <option key={d.id} value={d.id}>{d.nom}</option>)}
            </Select>
            <span className="text-slate-300">→</span>
            <Select value={dest} onChange={(e) => setDest(e.target.value)} className="max-w-[220px]">
              <option value="">Dépôt destination…</option>
              {(depots || []).filter((d) => d.id !== courant).map((d) => <option key={d.id} value={d.id}>{d.nom}</option>)}
            </Select>
            <Input placeholder="Rechercher un article…" value={q} onChange={(e) => setQ(e.target.value)} className="w-56" />
          </div>
          {!dest && (
            <p className="text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded-lg px-2 py-1.5 mb-2">
              Choisis un dépôt destination pour activer les transferts.
            </p>
          )}
          <DataView
            data={stock || []}
            empty="Aucun article dans ce dépôt."
            title={(s) => s.designation}
            subtitle={(s) => `${s.reference}${s.categorie ? ` • ${s.categorie}` : ""}`}
            fields={[{ label: "Quantité", value: (s) => `${s.quantite} ${s.unite ?? ""}` }]}
            actions={(s) => (
              <div className="flex items-center gap-1">
                <Input type="number" min={0} step="0.01" placeholder="Qté"
                  value={qte[s.produit_id] ?? ""}
                  onChange={(e) => setQte({ ...qte, [s.produit_id]: +e.target.value })}
                  className="!w-20 !py-1 text-sm" />
                <button
                  className="text-blue-700 disabled:opacity-40"
                  title="Transférer"
                  disabled={!dest || !qte[s.produit_id] || qte[s.produit_id] > s.quantite}
                  onClick={() => { if (window.confirm(`Transférer ${qte[s.produit_id]} × ${s.designation} ?`)) transferer.mutate(s); }}
                >
                  <ArrowLeftRight size={15} />
                </button>
              </div>
            )}
          />
        </Card>
      )}

      {tab === "mouvements" && (
        <Card>
          <div className="flex gap-2 mb-2">
            <Select value={courant} onChange={(e) => setDepotId(e.target.value)} className="max-w-[220px]">
              {(depots || []).map((d) => <option key={d.id} value={d.id}>{d.nom}</option>)}
            </Select>
          </div>
          <DataView
            data={mouvements || []}
            empty="Aucun mouvement de dépôt."
            title={(m) => m.designation ?? m.produit_id}
            subtitle={(m) => `${m.created_at} • ${m.motif || ""}`}
            fields={[
              { label: "Type", value: (m) => m.type_mvt },
              { label: "Quantité", value: (m) => `${m.quantite}` },
              { label: "Stock", value: (m) => `${m.stock_avant} → ${m.stock_apres}` },
            ]}
          />
        </Card>
      )}
    </div>
  );
}
