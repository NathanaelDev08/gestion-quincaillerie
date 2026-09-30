import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, List, Plus, Trash2, Undo2, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, confirmDelete, DataView, Select, Field, Pagination, paginate, Tabs, toast } from "../components/ui";
import { fmtMoney, fmtDate, todayISO } from "../utils/format";
import { getDefaultTva } from "../stores/prefs";

export default function Avoirs() {
  const qc = useQueryClient();
  const [modal, setModal] = useState(false);
  const [tab, setTab] = useState<"liste" | "nouveau">("liste");
  const [form, setForm] = useState<any>({ facture_id: "", client_id: "", motif: "", retour_stock: true });
  const [lignes, setLignes] = useState<any[]>([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), produit_id: "" }]);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  const { data: avoirs } = useQuery({ queryKey: ["avoirs"], queryFn: api.avoirsList });
  const { data: produits } = useQuery({ queryKey: ["produits-all"], queryFn: () => api.produitsList(1, 500) });
  const { data: factures } = useQuery({ queryKey: ["factures-all"], queryFn: () => api.facturesList(1, 200) });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 200) });

  const create = useMutation({
    mutationFn: () => api.avoirsCreate({
      facture_id: form.facture_id || null,
      client_id: form.client_id,
      date_emission: todayISO(),
      motif: form.motif,
      retour_stock: form.retour_stock,
      lignes: lignes.map((l) => ({ produit_id: l.produit_id || null, designation: l.designation, quantite: l.quantite, prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: 0 })),
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["avoirs"] });
      qc.invalidateQueries({ queryKey: ["stock"] });
      setModal(false);
      setTab("liste");
      toast.success("Avoir émis");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const del = useMutation({
    mutationFn: (a: any) => api.avoirsDelete(a.id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["avoirs"] }),
  });

  return (
    <div>
      <PageHeader title="Avoirs clients" subtitle="Remboursements et retours marchandises — montants en F CFA" />
      <Tabs<"liste" | "nouveau">
        active={tab} onChange={setTab}
        tabs={[
          { key: "liste", label: "Avoirs", icon: List },
          { key: "nouveau", label: "Nouvel avoir", icon: Plus },
        ]}
      />
      {tab === "liste" && (
      <>
      <DataView
        data={paginate(avoirs || [], page, perPage)}
        empty="Aucun avoir émis."
        title={(a) => a.numero}
        subtitle={(a) => `${a.client_nom ?? ""}${a.facture_numero ? ` • Facture ${a.facture_numero}` : ""}`}
        fields={[
          { label: "Date d'émission", value: (a) => fmtDate(a.date_emission) },
          { label: "Motif", value: (a) => a.motif || "-" },
          { label: "Total TTC", value: (a) => fmtMoney(a.total_ttc) },
          { label: "Statut", value: (a) => a.statut === "emis" ? "Émis" : a.statut },
        ]}
        actions={(a) => (
          <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(a.numero)) del.mutate(a); }} />
        )}
      />
      <Pagination page={page} perPage={perPage} total={(avoirs || []).length}
        onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
      </>
      )}
      {tab === "nouveau" && (
        <Card>
          <h3 className="font-semibold mb-2">Nouvel avoir client</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-2">
            <Field label="Facture d'origine (optionnel)">
              <Select value={form.facture_id} onChange={(e) => {
                const fid = e.target.value;
                const f = (factures?.data || []).find((x: any) => x.id === fid);
                setForm({ ...form, facture_id: fid, client_id: f ? f.client_id : form.client_id });
              }}>
                <option value="">-- Aucune --</option>
                {(factures?.data || []).map((f: any) => <option key={f.id} value={f.id}>{f.numero} — {fmtMoney(f.total_ttc)}</option>)}
              </Select>
            </Field>
            <Field label="Client *">
              <Select value={form.client_id} onChange={(e) => setForm({ ...form, client_id: e.target.value })}>
                <option value="">-- Client --</option>
                {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
              </Select>
            </Field>
            <Field label="Motif de l'avoir"><Input value={form.motif} onChange={(e) => setForm({ ...form, motif: e.target.value })} placeholder="Ex. Retour marchandise défectueuse" /></Field>
            <label className="flex items-center gap-2 text-sm mt-6">
              <input type="checkbox" checked={form.retour_stock} onChange={(e) => setForm({ ...form, retour_stock: e.target.checked })} />
              Réintégrer au stock
            </label>
          </div>
          {lignes.map((l, i) => (
            <div key={i} className="grid grid-cols-2 lg:grid-cols-6 gap-2 mb-1">
              <select className="px-3 py-2 border rounded-lg text-sm" value={l.produit_id} title="Article du catalogue"
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
              <Input type="number" placeholder="TVA %" value={l.taux_tva} onChange={(e) => { const c = [...lignes]; c[i].taux_tva = +e.target.value; setLignes(c); }} />
              <button className="text-red-500" title="Retirer" onClick={() => setLignes(lignes.filter((_, j) => j !== i))}><X size={15} /></button>
            </div>
          ))}
          <div className="flex justify-between mt-3">
            <Button className="bg-slate-500" onClick={() => setLignes([...lignes, { designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), produit_id: "" }])}><Plus size={15} /> Ligne</Button>
            <Button onClick={() => create.mutate()} disabled={!form.client_id}><Check size={15} /> Émettre l'avoir</Button>
          </div>
        </Card>
      )}
    </div>
  );
}
