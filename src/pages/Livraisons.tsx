import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Check, Eye, EyeOff, List, Plus, Printer, Trash2, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, confirmDelete, DataView, Select, Field, Pagination, paginate, Tabs, toast } from "../components/ui";
import { BLDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, fmtDate, todayISO } from "../utils/format";
import { getDefaultTva } from "../stores/prefs";

const STATUTS_BL = [
  { code: "brouillon", label: "Brouillon" },
  { code: "prepare", label: "Préparé" },
  { code: "livre", label: "Livré" },
  { code: "facture", label: "Facturé" },
  { code: "annule", label: "Annulé" },
];
const labelBl = (s: string) => STATUTS_BL.find((x) => x.code === s)?.label ?? s;

export default function Livraisons() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"bons" | "nouveau">("bons");
  const [detailId, setDetailId] = useState<string | null>(null);
  const [printBl, setPrintBl] = useState<any | null>(null);
  const [form, setForm] = useState<any>({ client_id: "", notes: "" });
  const [lignes, setLignes] = useState<any[]>([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), produit_id: "" }]);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  const { data: bls } = useQuery({ queryKey: ["bls"], queryFn: api.blsList });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 200) });
  const { data: produits } = useQuery({ queryKey: ["produits-all"], queryFn: () => api.produitsList(1, 500) });
  const { data: detail } = useQuery({
    queryKey: ["bl-detail", detailId],
    queryFn: () => api.blsGet(detailId!),
    enabled: !!detailId,
  });

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ["bls"] });
    qc.invalidateQueries({ queryKey: ["bl-detail"] });
    qc.invalidateQueries({ queryKey: ["stock"] });
    qc.invalidateQueries({ queryKey: ["factures"] });
  };
  const create = useMutation({
    mutationFn: () => api.blsCreate({
      client_id: form.client_id, devis_id: null, date_livraison: todayISO(), notes: form.notes,
      lignes: lignes.map((l) => ({ produit_id: l.produit_id || null, designation: l.designation, quantite: l.quantite, prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: 0 })),
    }),
    onSuccess: () => {
      refresh();
      setTab("bons");
      setForm({ client_id: "", notes: "" });
      setLignes([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), produit_id: "" }]);
      toast.success("Bon de livraison créé");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const statut = useMutation({
    mutationFn: ({ id, s }: { id: string; s: string }) => api.blsStatut(id, s),
    onSuccess: refresh,
  });
  const toFacture = useMutation({
    mutationFn: (id: string) => api.blsToFacture(id),
    onSuccess: () => { refresh(); toast.success("Facture créée depuis le BL"); },
  });
  const fromDevis = async (devisId: string) => {
    try {
      await api.blsFromDevis(devisId);
      refresh();
      toast.success("Bon de livraison créé depuis le devis");
    } catch (e: any) { toast.error(String(e)); }
  };
  const del = useMutation({
    mutationFn: (b: any) => api.blsDelete(b.id),
    onSuccess: refresh,
  });
  const imprimer = async (id: string) => {
    const [b, l] = await api.blsGet(id);
    setPrintBl({ b, l });
  };

  return (
    <div>
      <PageHeader title="Bons de livraison" subtitle="Préparation, livraison (sortie stock) et facturation" />
      <Tabs<"bons" | "nouveau">
        active={tab} onChange={setTab}
        tabs={[
          { key: "bons", label: "Bons de livraison", icon: List },
          { key: "nouveau", label: "Nouveau BL", icon: Plus },
        ]}
      />
      {tab === "bons" && (
      <>
      <DataView
        data={paginate(bls || [], page, perPage)}
        empty="Aucun bon de livraison."
        title={(b) => b.numero}
        subtitle={(b) => `${b.client_nom ?? ""} • ${fmtDate(b.date_livraison)}`}
        fields={[
          {
            label: "Statut", value: (b) => (
              <select className="text-xs border rounded px-1 py-0.5" value={b.statut} title="Statut"
                onChange={(e) => statut.mutate({ id: b.id, s: e.target.value })}>
                {STATUTS_BL.map((s) => <option key={s.code} value={s.code}>{s.label}</option>)}
              </select>
            ),
          },
          {
            label: "Détail", value: (b) => (
              <button className="text-blue-700 underline text-sm" onClick={() => setDetailId(detailId === b.id ? null : b.id)}>
                {detailId === b.id ? "Masquer" : "Voir"}
              </button>
            ),
          },
        ]}
        actions={(b) => (<>
          <IconBtn icon={detailId === b.id ? EyeOff : Eye} title="Voir le détail" tone="blue" onClick={() => setDetailId(detailId === b.id ? null : b.id)} />
          <IconBtn icon={Printer} title="Imprimer le BL" tone="blue" onClick={() => imprimer(b.id)} />
          <IconBtn icon={ArrowRight} title="Convertir en facture" tone="green" onClick={() => toFacture.mutate(b.id)} />
          <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(b.numero)) del.mutate(b); }} />
        </>)}
      />
      <Pagination page={page} perPage={perPage} total={(bls || []).length}
        onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />

      {detail && detailId && (
        <Card className="mt-3">
          <h3 className="font-semibold mb-2">Contenu — {detail[0].numero} ({labelBl(detail[0].statut)})</h3>
          <DataView
            data={detail[1]}
            empty="Aucune ligne."
            title={(l) => l.designation}
            subtitle={(l) => `PU HT : ${fmtMoney(l.prix_unitaire_ht)}`}
            fields={[{ label: "Quantité", value: (l) => `${l.quantite}` }]}
          />
          <p className="text-xs text-slate-500 mt-2">Le passage au statut « Livré » décrémente le stock. « Convertir en facture » crée la facture + écritures.</p>
        </Card>
      )}
      </>
      )}
      {tab === "nouveau" && (
        <Card>
          <h3 className="font-semibold mb-2">Nouveau bon de livraison</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-2 max-w-2xl">
            <Field label="Client *">
              <Select value={form.client_id} onChange={(e) => setForm({ ...form, client_id: e.target.value })}>
                <option value="">-- Client --</option>
                {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
              </Select>
            </Field>
            <Field label="Notes"><Input value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} /></Field>
          </div>
          <p className="text-xs text-slate-500 mb-1">Astuce : vous pouvez aussi créer un BL directement depuis un devis (page Devis, icône camion).</p>
          {lignes.map((l, i) => (
            <div key={i} className="grid grid-cols-2 lg:grid-cols-5 gap-2 mb-1">
              <select className="px-3 py-2 border rounded-lg text-sm" value={l.produit_id}
                onChange={(e) => {
                  const c = [...lignes];
                  const p = (produits?.data || []).find((x: any) => x.id === e.target.value);
                  c[i] = { ...c[i], produit_id: e.target.value, designation: p ? p.designation : c[i].designation, prix_unitaire_ht: p ? p.prix_vente_ht : c[i].prix_unitaire_ht, taux_tva: p ? p.taux_tva : c[i].taux_tva };
                  setLignes(c);
                }}>
                <option value="">-- Catalogue --</option>
                {(produits?.data || []).map((p: any) => <option key={p.id} value={p.id}>{p.reference} — {p.designation}</option>)}
              </select>
              <Input placeholder="Désignation" value={l.designation} onChange={(e) => { const c = [...lignes]; c[i].designation = e.target.value; setLignes(c); }} />
              <Input type="number" placeholder="Qté" value={l.quantite} onChange={(e) => { const c = [...lignes]; c[i].quantite = +e.target.value; setLignes(c); }} />
              <Input type="number" placeholder="PU HT" value={l.prix_unitaire_ht} onChange={(e) => { const c = [...lignes]; c[i].prix_unitaire_ht = +e.target.value; setLignes(c); }} />
              <button className="text-red-500" title="Retirer" onClick={() => setLignes(lignes.filter((_, j) => j !== i))}><X size={15} /></button>
            </div>
          ))}
          <div className="flex justify-between mt-3">
            <Button className="bg-slate-500" onClick={() => setLignes([...lignes, { designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), produit_id: "" }])}><Plus size={15} /> Ligne</Button>
            <Button onClick={() => create.mutate()} disabled={!form.client_id}><Check size={15} /> Créer le BL</Button>
          </div>
        </Card>
      )}
      {printBl && (
        <PrintModal title={`BL ${printBl.b.numero}`} onClose={() => setPrintBl(null)}>
          <BLDoc bl={printBl.b} lignes={printBl.l} />
        </PrintModal>
      )}
    </div>
  );
}
