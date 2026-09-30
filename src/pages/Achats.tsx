import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, List, PackageCheck, Plus, Printer, Trash2, Wallet, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, confirmDelete, DataView, Select, Field, Pagination, paginate, Tabs, toast, Modal } from "../components/ui";
import { BonCommandeDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, fmtDate, todayISO } from "../utils/format";

const STATUTS_CMD = [
  { code: "brouillon", label: "Brouillon" },
  { code: "validee", label: "Validée" },
  { code: "partielle", label: "Partiellement livrée" },
  { code: "livree", label: "Livrée" },
  { code: "annulee", label: "Annulée" },
];
const labelCmd = (s: string) => STATUTS_CMD.find((x) => x.code === s)?.label ?? s;

export default function Achats() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"commandes" | "nouvelle" | "dettes">("commandes");
  const [detailId, setDetailId] = useState<string | null>(null);
  const [form, setForm] = useState<any>({ fournisseur_id: "", date_livraison_prevue: "", notes: "" });
  const [lignes, setLignes] = useState<any[]>([{ designation: "", quantite: 1, prix_unitaire_ht: 0, produit_id: "" }]);
  const [recues, setRecues] = useState<Record<string, number>>({});
  const [printBC, setPrintBC] = useState(false);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  const { data: commandes } = useQuery({ queryKey: ["commandes"], queryFn: api.commandesList });
  const { data: fournisseurs } = useQuery({ queryKey: ["fournisseurs-all"], queryFn: () => api.fournisseursList(1, 200) });
  const { data: produits } = useQuery({ queryKey: ["produits-all"], queryFn: () => api.produitsList(1, 500) });
  const { data: detail } = useQuery({
    queryKey: ["commande-detail", detailId],
    queryFn: () => api.commandesGet(detailId!),
    enabled: !!detailId,
  });

  const refresh = () => { qc.invalidateQueries({ queryKey: ["commandes"] }); qc.invalidateQueries({ queryKey: ["commande-detail"] }); qc.invalidateQueries({ queryKey: ["stock"] }); };
  const create = useMutation({
    mutationFn: () => api.commandesCreate({
      fournisseur_id: form.fournisseur_id, date_commande: todayISO(),
      date_livraison_prevue: form.date_livraison_prevue || null, notes: form.notes,
      lignes: lignes.map((l) => ({ produit_id: l.produit_id || null, designation: l.designation, quantite: l.quantite, prix_unitaire_ht: l.prix_unitaire_ht })),
    }),
    onSuccess: () => {
      refresh();
      setTab("commandes");
      setForm({ fournisseur_id: "", date_livraison_prevue: "", notes: "" });
      setLignes([{ designation: "", quantite: 1, prix_unitaire_ht: 0, produit_id: "" }]);
      toast.success("Bon de commande créé");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const statut = useMutation({
    mutationFn: ({ id, s }: { id: string; s: string }) => api.commandesStatut(id, s),
    onSuccess: refresh,
  });
  const receptionner = useMutation({
    mutationFn: (id: string) => api.commandesReceptionner(id, Object.entries(recues).map(([k, v]) => [k, v] as [string, number])),
    onSuccess: () => { refresh(); setRecues({}); toast.success("Réception enregistrée, stock mis à jour"); },
  });
  const del = useMutation({
    mutationFn: (c: any) => api.commandesDelete(c.id),
    onSuccess: refresh,
  });

  return (
    <div>
      <PageHeader title="Achats fournisseurs" subtitle="Bons de commande et réceptions — montants en F CFA" />
      <Tabs<"commandes" | "nouvelle" | "dettes">
        active={tab} onChange={setTab}
        tabs={[
          { key: "commandes", label: "Bons de commande", icon: List },
          { key: "nouvelle", label: "Nouveau bon", icon: Plus },
          { key: "dettes", label: "Dettes fournisseurs", icon: Wallet },
        ]}
      />
      {tab === "commandes" && (
      <>
      <DataView
        data={paginate(commandes || [], page, perPage)}
        empty="Aucun bon de commande — créez le premier ci-dessus."
        title={(c) => c.numero}
        subtitle={(c) => `${c.fournisseur_nom ?? ""} • Commandé le ${fmtDate(c.date_commande)}`}
        fields={[
          { label: "Livraison prévue", value: (c) => fmtDate(c.date_livraison_prevue) },
          { label: "Total TTC", value: (c) => fmtMoney(c.total_ttc) },
          {
            label: "Statut", value: (c) => (
              <select className="text-xs border rounded px-1 py-0.5" value={c.statut} title="Statut"
                onChange={(e) => statut.mutate({ id: c.id, s: e.target.value })}>
                {STATUTS_CMD.map((s) => <option key={s.code} value={s.code}>{s.label}</option>)}
              </select>
            ),
          },
          {
            label: "Détail", value: (c) => (
              <button className="text-blue-700 underline text-sm" onClick={() => { setDetailId(detailId === c.id ? null : c.id); setRecues({}); }}>
                {detailId === c.id ? "Masquer" : "Réceptionner"}
              </button>
            ),
          },
        ]}
        actions={(c) => (<>
          <IconBtn icon={PackageCheck} title="Voir / réceptionner" tone="green" onClick={() => { setDetailId(c.id); setRecues({}); }} />
          <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(c.numero)) del.mutate(c); }} />
        </>)}
      />
      <Pagination page={page} perPage={perPage} total={(commandes || []).length}
        onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />

      {detail && detailId && (
        <Card className="mt-3">
          <div className="flex items-center justify-between mb-2">
            <h3 className="font-semibold">Réception — {detail[0].numero} ({labelCmd(detail[0].statut)})</h3>
            <Button onClick={() => setPrintBC(true)}><Printer size={15} /> Imprimer le bon de commande</Button>
          </div>
          <DataView
            data={detail[1]}
            empty="Aucune ligne."
            title={(l) => l.designation}
            subtitle={(l) => `Commandé : ${l.quantite} • Déjà reçu : ${l.quantite_recue}`}
            fields={[
              {
                label: "Quantité à recevoir", value: (l) => (
                  <input type="number" className="w-24 px-2 py-1 border rounded text-sm"
                    value={recues[l.id] ?? ""} max={l.quantite - l.quantite_recue}
                    onChange={(e) => setRecues({ ...recues, [l.id]: +e.target.value })} />
                ),
              },
              { label: "Prix unitaire HT", value: (l) => fmtMoney(l.prix_unitaire_ht) },
            ]}
          />
          <Button className="mt-2" onClick={() => receptionner.mutate(detailId)} disabled={Object.keys(recues).length === 0}>
            <Check size={15} /> Valider la réception
          </Button>
        </Card>
      )}
      </>
      )}
      {printBC && detail && (
        <PrintModal title={`Bon de commande ${detail[0].numero}`} onClose={() => setPrintBC(false)}>
          <BonCommandeDoc commande={detail[0]} lignes={detail[1]} />
        </PrintModal>
      )}

      {tab === "nouvelle" && (
        <Card>
          <h3 className="font-semibold mb-2">Nouveau bon de commande</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-2 max-w-2xl">
            <Field label="Fournisseur *">
              <Select value={form.fournisseur_id} onChange={(e) => setForm({ ...form, fournisseur_id: e.target.value })}>
                <option value="">-- Fournisseur --</option>
                {(fournisseurs?.data || []).map((f) => <option key={f.id} value={f.id}>{f.nom}</option>)}
              </Select>
            </Field>
            <Field label="Livraison prévue le"><Input type="date" value={form.date_livraison_prevue} onChange={(e) => setForm({ ...form, date_livraison_prevue: e.target.value })} /></Field>
          </div>
          {lignes.map((l, i) => (
            <div key={i} className="grid grid-cols-2 lg:grid-cols-5 gap-2 mb-1">
              <select className="px-3 py-2 border rounded-lg text-sm" value={l.produit_id}
                onChange={(e) => {
                  const c = [...lignes];
                  const p = (produits?.data || []).find((x: any) => x.id === e.target.value);
                  c[i] = { ...c[i], produit_id: e.target.value, designation: p ? p.designation : c[i].designation, prix_unitaire_ht: p ? p.prix_achat_ht : c[i].prix_unitaire_ht };
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
            <Button className="bg-slate-500" onClick={() => setLignes([...lignes, { designation: "", quantite: 1, prix_unitaire_ht: 0, produit_id: "" }])}><Plus size={15} /> Ligne</Button>
            <Button onClick={() => create.mutate()} disabled={!form.fournisseur_id}><Check size={15} /> Commander</Button>
          </div>
        </Card>
      )}
      {tab === "dettes" && <DettesPanel />}
    </div>
  );
}

function DettesPanel() {
  const qc = useQueryClient();
  const [showForm, setShowForm] = useState(false);
  const [f, setF] = useState<any>({ fournisseur_id: "", total_ht: 0, taux_tva: 18, date_echeance: todayISO(), notes: "" });
  const [pay, setPay] = useState<any>({ facture_id: "", montant: 0, mode: "virement", date_reglement: todayISO() });
  const [page, setPage] = useState(1);
  const perPage = 10;

  const { data: dettes } = useQuery({ queryKey: ["dettes"], queryFn: api.ffList });
  const { data: fournisseurs } = useQuery({ queryKey: ["fournisseurs-all"], queryFn: () => api.fournisseursList(1, 200) });
  const { data: aging } = useQuery({ queryKey: ["balance-agee"], queryFn: api.balanceAgee });

  const refresh = () => { qc.invalidateQueries({ queryKey: ["dettes"] }); qc.invalidateQueries({ queryKey: ["balance-agee"] }); };
  const create = useMutation({
    mutationFn: () => api.ffCreate({ ...f, date_emission: todayISO() }),
    onSuccess: () => { refresh(); setShowForm(false); toast.success("Facture fournisseur enregistrée"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const payer = useMutation({
    mutationFn: () => api.ffPayer(pay),
    onSuccess: () => { refresh(); setPay({ facture_id: "", montant: 0, mode: "virement", date_reglement: todayISO() }); toast.success("Règlement fournisseur enregistré"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const del = useMutation({
    mutationFn: (id: string) => api.ffDelete(id),
    onSuccess: refresh,
    onError: (e: any) => toast.error(String(e)),
  });

  const totalDu = (dettes || []).reduce((s: number, d: any) => s + (d.total_ttc - d.montant_paye), 0);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
        {(aging?.tranches || []).map((t: any) => (
          <Card key={t.label} className="!p-2.5">
            <div className="text-[11px] text-slate-500">{t.label}</div>
            <div className="font-bold">{fmtMoney(t.total)}</div>
          </Card>
        ))}
      </div>
      <Card>
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold">Dettes fournisseurs — reste dû {fmtMoney(totalDu)}</h3>
          <Button onClick={() => setShowForm((v) => !v)}><Plus size={15} /> Facture d'achat</Button>
        </div>
        {showForm && (
          <div className="grid grid-cols-2 lg:grid-cols-6 gap-2 mb-3 p-2 border rounded-xl bg-slate-50">
            <Select value={f.fournisseur_id} onChange={(e) => setF({ ...f, fournisseur_id: e.target.value })} title="Fournisseur">
              <option value="">-- Fournisseur --</option>
              {(fournisseurs?.data || []).map((x: any) => <option key={x.id} value={x.id}>{x.nom}</option>)}
            </Select>
            <Input type="number" placeholder="Total HT" value={f.total_ht || ""} onChange={(e) => setF({ ...f, total_ht: +e.target.value })} />
            <Input type="number" placeholder="TVA %" value={f.taux_tva} onChange={(e) => setF({ ...f, taux_tva: +e.target.value })} />
            <Input type="date" value={f.date_echeance} onChange={(e) => setF({ ...f, date_echeance: e.target.value })} title="Échéance" />
            <Input placeholder="Notes" value={f.notes} onChange={(e) => setF({ ...f, notes: e.target.value })} />
            <Button onClick={() => create.mutate()} disabled={!f.fournisseur_id || !f.total_ht}><Check size={15} /> Enregistrer</Button>
          </div>
        )}
        <DataView
          data={paginate(dettes || [], page, perPage)}
          empty="Aucune dette fournisseur."
          title={(d: any) => `${d.numero} — ${d.fournisseur_nom ?? ""}`}
          subtitle={(d: any) => `Échéance : ${fmtDate(d.date_echeance)} • ${d.statut}`}
          fields={[
            { label: "Total TTC", value: (d: any) => fmtMoney(d.total_ttc) },
            { label: "Payé", value: (d: any) => fmtMoney(d.montant_paye) },
            { label: "Reste dû", value: (d: any) => fmtMoney(d.total_ttc - d.montant_paye) },
          ]}
          actions={(d: any) => (
            <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(d.numero)) del.mutate(d.id); }} />
          )}
        />
        <Pagination page={page} perPage={perPage} total={(dettes || []).length} onChange={(p) => setPage(p)} />
      </Card>
      <Card>
        <h3 className="font-semibold mb-2">Régler un fournisseur</h3>
        <div className="grid grid-cols-2 lg:grid-cols-5 gap-2">
          <Select value={pay.facture_id} onChange={(e) => setPay({ ...pay, facture_id: e.target.value })} title="Facture">
            <option value="">-- Facture --</option>
            {(dettes || []).filter((d: any) => d.total_ttc - d.montant_paye > 0.005).map((d: any) => (
              <option key={d.id} value={d.id}>{d.numero} ({fmtMoney(d.total_ttc - d.montant_paye)} dû)</option>
            ))}
          </Select>
          <Input type="number" placeholder="Montant" value={pay.montant || ""} onChange={(e) => setPay({ ...pay, montant: +e.target.value })} />
          <Select value={pay.mode} onChange={(e) => setPay({ ...pay, mode: e.target.value })} title="Mode">
            <option value="virement">Virement</option><option value="especes">Espèces</option>
            <option value="cheque">Chèque</option><option value="mobile">Mobile Money</option>
          </Select>
          <Input type="date" value={pay.date_reglement} onChange={(e) => setPay({ ...pay, date_reglement: e.target.value })} />
          <Button onClick={() => payer.mutate()} disabled={!pay.facture_id || !pay.montant}><Check size={15} /> Payer</Button>
        </div>
      </Card>
    </div>
  );
}
