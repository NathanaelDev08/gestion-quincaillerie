import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Check, Copy, FileDown, List, Pencil, Plus, Printer, Trash2, Truck, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Badge, IconBtn, confirmDelete, DataView, Select, Pagination, Tabs, toast } from "../components/ui";
import { DevisDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, fmtDate, todayISO, STATUTS_DEVIS } from "../utils/format";
import { getDefaultTva } from "../stores/prefs";

export default function Devis() {
  const qc = useQueryClient();
  const [clientId, setClientId] = useState("");
  const [editId, setEditId] = useState<string | null>(null);
  const [tab, setTab] = useState<"liste" | "nouveau">("liste");
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(20);
  const [lignes, setLignes] = useState<any[]>([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }]);
  const { data, isLoading, isError, refetch } = useQuery({ queryKey: ["devis", page, perPage], queryFn: () => api.devisList(page, perPage) });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 200) });

  const create = useMutation({
    mutationFn: () => editId
      ? api.devisUpdate(editId, { client_id: clientId, date_emission: todayISO(), date_validite: todayISO(), remise: 0, notes: "", lignes })
      : api.devisCreate({
        client_id: clientId, date_emission: todayISO(), date_validite: todayISO(),
        remise: 0, notes: "", lignes,
      }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["devis"] }); resetForm(); setTab("liste"); },
  });
  const resetForm = () => {
    setEditId(null); setClientId("");
    setLignes([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }]);
  };
  const openEdit = async (id: string) => {
    const [d, l] = await api.devisGet(id);
    setEditId(id); setClientId(d.client_id);
    setLignes(l.map((x: any) => ({ produit_id: x.produit_id, designation: x.designation, quantite: x.quantite, prix_unitaire_ht: x.prix_unitaire_ht, taux_tva: x.taux_tva, remise: x.remise })));
    setTab("nouveau");
    window.scrollTo({ top: 0 });
  };
  const toFacture = useMutation({ mutationFn: (id: string) => api.devisToFacture(id), onSuccess: () => { qc.invalidateQueries({ queryKey: ["devis"] }); qc.invalidateQueries({ queryKey: ["factures"] }); toast.success("Facture créée"); }, onError: (e: any) => toast.error(String(e)) });
  const toBl = useMutation({ mutationFn: (id: string) => api.blsFromDevis(id), onSuccess: () => { qc.invalidateQueries({ queryKey: ["devis"] }); qc.invalidateQueries({ queryKey: ["bls"] }); toast.success("Bon de livraison créé"); }, onError: (e: any) => toast.error(String(e)) });
  const del = useMutation({ mutationFn: (id: string) => api.devisDelete(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["devis"] }) });
  const statut = useMutation({ mutationFn: ({ id, s }: { id: string; s: string }) => api.devisStatut(id, s), onSuccess: () => qc.invalidateQueries({ queryKey: ["devis"] }) });
  const dupliquer = useMutation({ mutationFn: (id: string) => api.devisDupliquer(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["devis"] }) });
  const pdf = async (id: string) => {
    try {
      const p = await api.devisPdf(id);
      toast.success("Document : " + p);
    } catch (e: any) { toast.error(String(e)); }
  };
  const [printDevis, setPrintDevis] = useState<any | null>(null);
  const imprimer = async (id: string) => {
    const [d, l] = await api.devisGet(id);
    setPrintDevis({ d, l });
  };

  return (
    <div>
      <PageHeader title="Devis" subtitle={`${data?.total || 0} devis — montants en F CFA`} />
      {(clients?.total ?? -1) === 0 && (
        <Card className="mb-3 border-amber-300 bg-amber-50">
          <p className="text-sm text-amber-800">
            <b>Aucun client enregistré :</b> créez un client via la page Clients, ou chargez la démo via Paramètres.
          </p>
        </Card>
      )}
      <Tabs<"liste" | "nouveau">
        active={tab} onChange={setTab}
        tabs={[
          { key: "liste", label: "Devis", icon: List },
          { key: "nouveau", label: editId ? "Modifier le devis" : "Nouveau devis", icon: Plus },
        ]}
      />
      {tab === "nouveau" && (
        <Card>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-2">
          <select className="px-3 py-2 border rounded-lg text-sm" value={clientId} onChange={(e) => setClientId(e.target.value)}>
            <option value="">-- Client --</option>
            {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
          </select>
          <Button onClick={() => setLignes([...lignes, { designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }])}><Plus size={15} /> Ligne</Button>
          {editId ? (
            <>
              <Button onClick={() => create.mutate()} disabled={!clientId}><Check size={15} /> Enregistrer</Button>
              <Button className="bg-slate-500" onClick={resetForm}><X size={15} /> Annuler</Button>
            </>
          ) : (
            <Button onClick={() => create.mutate()} disabled={!clientId}><Check size={15} /> Créer devis</Button>
          )}
        </div>
        {lignes.map((l, i) => (
          <div key={i} className="grid grid-cols-2 lg:grid-cols-5 gap-2 mb-1">
            <Input placeholder="Désignation" value={l.designation} onChange={(e) => { const c = [...lignes]; c[i].designation = e.target.value; setLignes(c); }} />
            <Input type="number" placeholder="Qté" value={l.quantite} onChange={(e) => { const c = [...lignes]; c[i].quantite = +e.target.value; setLignes(c); }} />
            <Input type="number" placeholder="PU HT" value={l.prix_unitaire_ht} onChange={(e) => { const c = [...lignes]; c[i].prix_unitaire_ht = +e.target.value; setLignes(c); }} />
            <Input type="number" placeholder="TVA %" value={l.taux_tva} onChange={(e) => { const c = [...lignes]; c[i].taux_tva = +e.target.value; setLignes(c); }} />
            <button className="text-red-500" title="Retirer la ligne" onClick={() => setLignes(lignes.filter((_, j) => j !== i))}><X size={15} /></button>
          </div>
        ))}
        </Card>
      )}
      {tab === "liste" && (
        <div>
          <DataView
          data={data?.data ?? []}
          loading={isLoading}
          error={isError ? true : null}
          onRetry={() => refetch()}
          empty="Aucun devis — créez votre premier devis ci-dessus."
          title={(d) => d.numero}
          subtitle={(d) => `${d.client_nom ?? ""} • Émis le ${fmtDate(d.date_emission)}`}
          fields={[
            { label: "Total TTC", value: (d) => fmtMoney(d.total_ttc) },
            {
              label: "Statut", value: (d) => (
                <select className="text-xs border rounded px-1 py-0.5" value={d.statut} title="Statut du devis"
                  onChange={(e) => statut.mutate({ id: d.id, s: e.target.value })}>
                  {STATUTS_DEVIS.map((s) => <option key={s.code} value={s.code}>{s.label}</option>)}
                </select>
              ),
            },
            { label: "Validité", value: (d) => fmtDate(d.date_validite) },
            { label: "Total HT", value: (d) => fmtMoney(d.total_ht) },
          ]}
          actions={(d) => (<>
            <IconBtn icon={Pencil} title="Modifier" tone="blue" onClick={() => openEdit(d.id)} />
            <IconBtn icon={Truck} title="Créer le bon de livraison" tone="green" onClick={() => toBl.mutate(d.id)} />
            <IconBtn icon={Printer} title="Imprimer le devis" tone="blue" onClick={() => imprimer(d.id)} />
            <IconBtn icon={ArrowRight} title="Convertir en facture" tone="blue" onClick={() => toFacture.mutate(d.id)} />
            <IconBtn icon={Copy} title="Dupliquer" onClick={() => dupliquer.mutate(d.id)} />
            <IconBtn icon={FileDown} title="Générer le PDF" onClick={() => pdf(d.id)} />
            <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(d.numero)) del.mutate(d.id); }} />
          </>)}
        />
        <Pagination page={page} perPage={perPage} total={data?.total || 0}
          onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
        </div>
      )}
      {printDevis && (
        <PrintModal title={`Devis ${printDevis.d.numero}`} onClose={() => setPrintDevis(null)}>
          <DevisDoc devis={printDevis.d} lignes={printDevis.l} />
        </PrintModal>
      )}
    </div>
  );
}
