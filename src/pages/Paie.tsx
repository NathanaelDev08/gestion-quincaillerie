import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Pencil, Plus, Printer, Trash2, UserX, Users, Wallet, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, Modal, confirmDelete, DataView, Select, Field, Pagination, paginate, Tabs, toast } from "../components/ui";
import { BulletinDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, todayISO } from "../utils/format";

const STATUTS_BUL = [
  { code: "brouillon", label: "Brouillon" },
  { code: "valide", label: "Validé" },
  { code: "paye", label: "Payé" },
];
const labelBul = (s: string) => STATUTS_BUL.find((x) => x.code === s)?.label ?? s;
const moisOptions = () => {
  const out: string[] = [];
  const d = new Date();
  for (let i = 0; i < 12; i++) {
    out.push(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`);
    d.setMonth(d.getMonth() - 1);
  }
  return out;
};

export default function Paie() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"employes" | "bulletins">("employes");
  const [modalEmp, setModalEmp] = useState(false);
  const [editEmp, setEditEmp] = useState<string | null>(null);
  const [emp, setEmp] = useState<any>({ nom: "", prenom: "", poste: "", telephone: "", salaire_base: 0, date_embauche: todayISO() });
  const [periode, setPeriode] = useState(new Date().toISOString().slice(0, 7));
  const [gen, setGen] = useState({ employe_id: "", brut: 0 });
  const [printBul, setPrintBul] = useState<any | null>(null);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  const { data: employes } = useQuery({ queryKey: ["employes"], queryFn: () => api.employesList(false) });
  const { data: bulletins } = useQuery({ queryKey: ["bulletins", periode], queryFn: () => api.bulletinsList(periode) });
  const { data: masse } = useQuery({ queryKey: ["masse", periode], queryFn: () => api.masseSalariale(periode) });

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ["employes"] });
    qc.invalidateQueries({ queryKey: ["bulletins"] });
    qc.invalidateQueries({ queryKey: ["masse"] });
    qc.invalidateQueries({ queryKey: ["depenses"] });
  };
  const saveEmp = useMutation({
    mutationFn: () => editEmp ? api.employesUpdate(editEmp, emp) : api.employesCreate(emp),
    onSuccess: () => { refresh(); setModalEmp(false); },
  });
  const toggleEmp = useMutation({ mutationFn: (id: string) => api.employesToggleActif(id), onSuccess: refresh });
  const genBul = useMutation({
    mutationFn: () => api.bulletinsCreate({ employe_id: gen.employe_id, periode, brut: gen.brut }),
    onSuccess: () => { refresh(); setGen({ employe_id: "", brut: 0 }); },
    onError: (e: any) => toast.error(String(e)),
  });
  const statutBul = useMutation({
    mutationFn: ({ id, s }: { id: string; s: string }) => api.bulletinsStatut(id, s),
    onSuccess: refresh,
  });
  const delBul = useMutation({ mutationFn: (id: string) => api.bulletinsDelete(id), onSuccess: refresh });

  const openEmp = (e?: any) => {
    if (e) { setEditEmp(e.id); setEmp({ ...e }); }
    else { setEditEmp(null); setEmp({ nom: "", prenom: "", poste: "", telephone: "", salaire_base: 0, date_embauche: todayISO() }); }
    setModalEmp(true);
  };

  return (
    <div>
      <PageHeader title="Paie & RH" subtitle={`Masse salariale ${periode} : ${fmtMoney(masse?.net || 0)} net (${masse?.nb || 0} bulletins)`}
        actions={<Button onClick={() => openEmp()}><Plus size={15} /> Employé</Button>} />
      <Tabs<"employes" | "bulletins">
        active={tab} onChange={setTab}
        tabs={[
          { key: "employes", label: "Employés", icon: Users },
          { key: "bulletins", label: "Bulletins de paie", icon: Wallet },
        ]}
      />
      {tab === "employes" && (
        <Card>
          <h3 className="font-semibold mb-2">Registre des employés ({(employes || []).length})</h3>
          <DataView
            data={employes || []}
            empty="Aucun employé — ajoutez le premier."
            title={(e) => `${e.nom} ${e.prenom ?? ""}`}
            subtitle={(e) => `${e.poste || "—"} • ${fmtMoney(e.salaire_base)} base`}
            fields={[{ label: "Statut", value: (e) => (e.actif ? "Actif" : "Inactif") }]}
            actions={(e) => (<>
              <IconBtn icon={Pencil} title="Modifier" tone="blue" onClick={() => openEmp(e)} />
              <IconBtn icon={UserX} title="Activer / désactiver" tone="amber" onClick={() => toggleEmp.mutate(e.id)} />
            </>)}
          />
        </Card>
      )}
      {tab === "bulletins" && (
        <Card>
          <div className="flex items-center gap-2 mb-2">
            <h3 className="font-semibold">Bulletins — paie du mois</h3>
            <Select value={periode} onChange={(e) => setPeriode(e.target.value)} title="Période">
              {moisOptions().map((m) => <option key={m} value={m}>{m}</option>)}
            </Select>
          </div>
          <div className="grid grid-cols-3 gap-2 mb-2">
            <Select value={gen.employe_id} onChange={(e) => {
              const em = (employes || []).find((x) => x.id === e.target.value);
              setGen({ employe_id: e.target.value, brut: em ? em.salaire_base : 0 });
            }} title="Employé">
              <option value="">-- Employé --</option>
              {(employes || []).filter((e) => e.actif).map((e) => <option key={e.id} value={e.id}>{e.nom} {e.prenom}</option>)}
            </Select>
            <Input type="number" placeholder="Brut (F CFA)" value={gen.brut || ""} onChange={(e) => setGen({ ...gen, brut: +e.target.value })} />
            <Button onClick={() => genBul.mutate()} disabled={!gen.employe_id || !gen.brut}><Plus size={15} /> Générer</Button>
          </div>
          <DataView
            data={paginate(bulletins || [], page, perPage)}
            empty="Aucun bulletin sur cette période."
            title={(b) => `${b.numero} — ${b.employe_nom}`}
            subtitle={(b) => `Brut ${fmtMoney(b.brut)} • CNPS ${fmtMoney(b.cnps)} • ITS ${fmtMoney(b.its)}`}
            fields={[
              { label: "Net à payer", value: (b) => fmtMoney(b.net) },
              {
                label: "Statut", value: (b) => (
                  <select className="text-xs border rounded px-1 py-0.5" value={b.statut} title="Statut"
                    onChange={(e) => statutBul.mutate({ id: b.id, s: e.target.value })}>
                    {STATUTS_BUL.map((s) => <option key={s.code} value={s.code}>{s.label}</option>)}
                  </select>
                ),
              },
            ]}
            actions={(b) => (<>
              <IconBtn icon={Printer} title="Imprimer le bulletin" tone="blue" onClick={() => setPrintBul(b)} />
              {b.statut === "brouillon" && <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => delBul.mutate(b.id)} />}
            </>)}
          />
          <Pagination page={page} perPage={perPage} total={(bulletins || []).length}
            onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
          <p className="text-xs text-slate-500 mt-1">CNPS salariale 6,3 % • ITS 10 %. Le passage à « Payé » crée la dépense salariale automatiquement.</p>
        </Card>
      )}

      {modalEmp && (
        <Modal title={editEmp ? "Modifier l'employé" : "Nouvel employé"} onClose={() => setModalEmp(false)}>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <Field label="Nom *"><Input value={emp.nom} onChange={(e) => setEmp({ ...emp, nom: e.target.value })} /></Field>
            <Field label="Prénom"><Input value={emp.prenom} onChange={(e) => setEmp({ ...emp, prenom: e.target.value })} /></Field>
            <Field label="Poste"><Input value={emp.poste} onChange={(e) => setEmp({ ...emp, poste: e.target.value })} placeholder="Ex. Caissier" /></Field>
            <Field label="Téléphone"><Input value={emp.telephone} onChange={(e) => setEmp({ ...emp, telephone: e.target.value })} /></Field>
            <Field label="Salaire de base (F CFA) *"><Input type="number" value={emp.salaire_base} onChange={(e) => setEmp({ ...emp, salaire_base: +e.target.value })} /></Field>
            <Field label="Date d'embauche"><Input type="date" value={emp.date_embauche} onChange={(e) => setEmp({ ...emp, date_embauche: e.target.value })} /></Field>
          </div>
          <div className="flex justify-end gap-2 mt-3">
            <Button className="bg-slate-500" onClick={() => setModalEmp(false)}>Annuler</Button>
            <Button onClick={() => saveEmp.mutate()} disabled={!emp.nom}><Check size={15} /> Enregistrer</Button>
          </div>
        </Modal>
      )}
      {printBul && (
        <PrintModal title={`Bulletin ${printBul.numero}`} onClose={() => setPrintBul(null)}>
          <BulletinDoc bulletin={printBul} />
        </PrintModal>
      )}
    </div>
  );
}
