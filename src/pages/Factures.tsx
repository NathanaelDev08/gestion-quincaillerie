import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, Eye, EyeOff, FileDown, List, Plus, Printer, Trash2, Undo2, Wallet, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Badge, IconBtn, confirmDelete, DataView, Pagination, Tabs, toast, paginate } from "../components/ui";
import { FactureDoc, RecuDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, fmtDate, todayISO, STATUTS_FACTURE, MODES_REGLEMENT, labelOf } from "../utils/format";
import { getDefaultTva } from "../stores/prefs";

type Tab = "liste" | "nouvelle" | "reglements";

export default function Factures() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>("liste");
  const [clientId, setClientId] = useState("");
  const [lignes, setLignes] = useState<any[]>([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }]);
  const [reglement, setReglement] = useState<any>({ facture_id: "", montant: 0, mode: "virement", date_reglement: todayISO() });
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(20);
  const [pageR, setPageR] = useState(1);
  const { data, isLoading, isError, refetch } = useQuery({ queryKey: ["factures", page, perPage], queryFn: () => api.facturesList(page, perPage) });
  const [detailId, setDetailId] = useState<string | null>(null);
  const [printFacture, setPrintFacture] = useState(false);
  const [printRecu, setPrintRecu] = useState<any | null>(null);
  const { data: detail } = useQuery({
    queryKey: ["facture-detail", detailId],
    queryFn: () => api.facturesGet(detailId!),
    enabled: !!detailId,
  });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 200) });
  const { data: regsToutes } = useQuery({ queryKey: ["reglements-tout"], queryFn: () => api.reglementsList(null), enabled: tab === "reglements" });

  const create = useMutation({
    mutationFn: () => api.facturesCreate({ client_id: clientId, date_emission: todayISO(), date_echeance: todayISO(), remise: 0, lignes }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["factures"] });
      toast.success("Facture créée");
      setTab("liste");
      setLignes([{ designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }]);
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const payer = useMutation({
    mutationFn: () => api.reglementsCreate(reglement),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["factures"] });
      qc.invalidateQueries({ queryKey: ["reglements-tout"] });
      qc.invalidateQueries({ queryKey: ["facture-detail"] });
      toast.success("Règlement enregistré");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const pdf = async (id: string) => {
    try {
      const path = await api.facturesPdf(id);
      toast.success("Document généré : " + path);
    } catch (e: any) { toast.error(String(e)); }
  };
  const del = useMutation({ mutationFn: (id: string) => api.facturesDelete(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["factures"] }) });
  const statut = useMutation({ mutationFn: ({ id, s }: { id: string; s: string }) => api.facturesStatut(id, s), onSuccess: () => qc.invalidateQueries({ queryKey: ["factures"] }) });
  const dupliquer = useMutation({ mutationFn: (id: string) => api.facturesDupliquer(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["factures"] }) });
  const avoirExpress = useMutation({
    mutationFn: async (f: any) => {
      const [fac, lignes] = await api.facturesGet(f.id);
      return api.avoirsCreate({
        facture_id: f.id,
        client_id: fac.client_id,
        date_emission: todayISO(),
        motif: `Retour total ${fac.numero}`,
        lignes: (lignes || []).map((l: any) => ({
          produit_id: l.produit_id, designation: l.designation, quantite: l.quantite,
          prix_unitaire_ht: l.prix_unitaire_ht, taux_tva: l.taux_tva, remise: 0,
        })),
        retour_stock: true,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["factures"] });
      qc.invalidateQueries({ queryKey: ["avoirs"] });
      qc.invalidateQueries({ queryKey: ["stock"] });
      toast.success("Avoir total créé, stock réintégré");
    },
    onError: (e: any) => toast.error(String(e)),
  });

  return (
    <div>
      <PageHeader title="Factures" subtitle={`${data?.total || 0} factures — montants en F CFA`} />
      {(clients?.total ?? -1) === 0 && (
        <Card className="mb-3 border-amber-300 bg-amber-50">
          <p className="text-sm text-amber-800">
            <b>Aucun client enregistré :</b> impossible de créer une facture. Créez un client via la page Clients,
            ou chargez des données de démonstration via Paramètres → « Charger la démo ».
          </p>
        </Card>
      )}
      <Tabs<Tab>
        active={tab} onChange={setTab}
        tabs={[
          { key: "liste", label: "Factures", icon: List },
          { key: "nouvelle", label: "Nouvelle facture", icon: Plus },
          { key: "reglements", label: "Règlements", icon: Wallet },
        ]}
      />

      {tab === "nouvelle" && (
        <Card>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-2">
            <select className="px-3 py-2 border rounded-lg text-sm" value={clientId} onChange={(e) => setClientId(e.target.value)}>
              <option value="">-- Client --</option>
              {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
            </select>
            <Button onClick={() => setLignes([...lignes, { designation: "", quantite: 1, prix_unitaire_ht: 0, taux_tva: getDefaultTva(), remise: 0 }])}><Plus size={15} /> Ligne</Button>
            <Button onClick={() => create.mutate()} disabled={!clientId}><Check size={15} /> Créer facture</Button>
          </div>
          {lignes.map((l, i) => (
            <div key={i} className="grid grid-cols-2 lg:grid-cols-4 gap-2 mb-1">
              <Input placeholder="Désignation" value={l.designation} onChange={(e) => { const c = [...lignes]; c[i].designation = e.target.value; setLignes(c); }} />
              <Input type="number" value={l.quantite} onChange={(e) => { const c = [...lignes]; c[i].quantite = +e.target.value; setLignes(c); }} />
              <Input type="number" value={l.prix_unitaire_ht} onChange={(e) => { const c = [...lignes]; c[i].prix_unitaire_ht = +e.target.value; setLignes(c); }} />
              <button className="text-red-500" title="Retirer la ligne" onClick={() => setLignes(lignes.filter((_, j) => j !== i))}><X size={15} /></button>
            </div>
          ))}
        </Card>
      )}

      {tab === "reglements" && (
        <div className="space-y-4">
          <Card>
            <h3 className="font-semibold mb-2">Enregistrer un règlement</h3>
            <div className="grid grid-cols-2 lg:grid-cols-5 gap-2">
              <select className="px-3 py-2 border rounded-lg text-sm" value={reglement.facture_id} onChange={(e) => setReglement({ ...reglement, facture_id: e.target.value })}>
                <option value="">-- Facture --</option>
                {(data?.data || []).map((f) => <option key={f.id} value={f.id}>{f.numero} ({fmtMoney(f.total_ttc - f.montant_paye)} dû)</option>)}
              </select>
              <Input type="number" placeholder="Montant" value={reglement.montant} onChange={(e) => setReglement({ ...reglement, montant: +e.target.value })} />
              <select className="px-3 py-2 border rounded-lg text-sm" value={reglement.mode} onChange={(e) => setReglement({ ...reglement, mode: e.target.value })}>
                {MODES_REGLEMENT.map((m) => <option key={m.code} value={m.code}>{m.label}</option>)}
              </select>
              <Input type="date" value={reglement.date_reglement} onChange={(e) => setReglement({ ...reglement, date_reglement: e.target.value })} />
              <Button onClick={() => payer.mutate()} disabled={!reglement.facture_id || !reglement.montant}><Check size={15} /> Payer</Button>
            </div>
          </Card>
          <Card>
            <h3 className="font-semibold mb-2">Historique des règlements</h3>
            <DataView
              data={paginate(regsToutes || [], pageR, 10)}
              empty="Aucun règlement enregistré."
              title={(r: any) => fmtMoney(r.montant)}
              subtitle={(r: any) => `${labelOf(MODES_REGLEMENT, r.mode)} • ${fmtDate(r.date_reglement)}`}
              fields={[{ label: "Référence", value: (r: any) => r.reference || "-" }]}
            />
            <Pagination page={pageR} perPage={10} total={(regsToutes || []).length} onChange={(p) => setPageR(p)} />
          </Card>
        </div>
      )}

      {tab === "liste" && (
        <div>
          <DataView
            data={data?.data ?? []}
            loading={isLoading}
            error={isError ? true : null}
            onRetry={() => refetch()}
            empty="Aucune facture — créez votre première facture ci-dessus."
            title={(f) => f.numero}
            subtitle={(f) => `${f.client_nom ?? ""} • Échéance : ${fmtDate(f.date_echeance)}`}
            fields={[
              { label: "Total TTC", value: (f) => fmtMoney(f.total_ttc) },
              { label: "Montant réglé", value: (f) => fmtMoney(f.montant_paye) },
              { label: "Reste dû", value: (f) => fmtMoney(f.total_ttc - f.montant_paye) },
              {
                label: "Statut", value: (f) => (
                  <select className="text-xs border rounded px-1 py-0.5" value={f.statut} title="Statut de la facture"
                    onChange={(e) => statut.mutate({ id: f.id, s: e.target.value })}>
                    {STATUTS_FACTURE.map((s) => <option key={s.code} value={s.code}>{s.label}</option>)}
                  </select>
                ),
              },
            ]}
            actions={(f) => (<>
              <IconBtn icon={detailId === f.id ? EyeOff : Eye} title="Voir le détail" tone="blue" onClick={() => setDetailId(detailId === f.id ? null : f.id)} />
              <IconBtn icon={FileDown} title="Générer le PDF" onClick={() => pdf(f.id)} />
              <IconBtn icon={Copy} title="Dupliquer" onClick={() => dupliquer.mutate(f.id)} />
              <IconBtn icon={Undo2} title="Avoir total + retour stock en 1 clic" tone="green" onClick={() => { if (window.confirm(`Créer un avoir total pour ${f.numero} et réintégrer le stock ?`)) avoirExpress.mutate(f); }} />
              <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(f.numero)) del.mutate(f.id); }} />
            </>)}
          />
          <Pagination page={page} perPage={perPage} total={data?.total || 0}
            onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
          {detail && detailId && (
            <Card className="mt-3">
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-semibold">Détail {detail[0].numero} — lignes, règlements & impression</h3>
                <Button onClick={() => setPrintFacture(true)}><Printer size={15} /> Imprimer la facture</Button>
              </div>
              <DataView
                data={detail[1] || []}
                empty="Aucune ligne."
                title={(l: any) => l.designation}
                subtitle={() => "Ligne de facture"}
                fields={[
                  { label: "Quantité", value: (l: any) => `${l.quantite}` },
                  { label: "Prix unitaire HT", value: (l: any) => fmtMoney(l.prix_unitaire_ht) },
                  { label: "TVA", value: (l: any) => `${l.taux_tva} %` },
                  { label: "Total HT", value: (l: any) => fmtMoney(l.total_ht) },
                ]}
              />
              <h4 className="font-semibold mt-3 mb-2">Règlements ({(detail[2] || []).length}) — reçu imprimable</h4>
              <DataView
                data={detail[2] || []}
                empty="Aucun règlement enregistré."
                title={(r: any) => fmtMoney(r.montant)}
                subtitle={(r: any) => labelOf(MODES_REGLEMENT, r.mode)}
                fields={[
                  { label: "Date", value: (r: any) => fmtDate(r.date_reglement) },
                  { label: "Référence", value: (r: any) => r.reference || "-" },
                ]}
                actions={(r: any) => (
                  <IconBtn icon={Printer} title="Imprimer le reçu" tone="blue" onClick={() => setPrintRecu(r)} />
                )}
              />
            </Card>
          )}
        </div>
      )}
      {printFacture && detail && (
        <PrintModal title={`Facture ${detail[0].numero}`} onClose={() => setPrintFacture(false)}>
          <FactureDoc facture={detail[0]} lignes={detail[1]} />
        </PrintModal>
      )}
      {printRecu && detail && (
        <PrintModal title="Reçu de paiement" onClose={() => setPrintRecu(null)}>
          <RecuDoc reglement={printRecu} factureNumero={detail[0].numero} clientNom={detail[0].client_nom} />
        </PrintModal>
      )}
    </div>
  );
}
