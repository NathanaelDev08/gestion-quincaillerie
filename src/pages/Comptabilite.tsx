import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { BookOpen, Check, ListOrdered, Lock, PieChart, Plus, Scale } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, DataView, Pagination, paginate, Tabs, toast } from "../components/ui";
import { fmtMoney, todayISO, JOURNAUX } from "../utils/format";

export default function Comptabilite() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"journal" | "balance" | "livre" | "exercices" | "ohada">("journal");
  const [compte, setCompte] = useState("411000");
  const [ecr, setEcr] = useState({ journal_code: "OD", compte_numero: "", date_ecriture: todayISO(), libelle: "", debit: 0, credit: 0 });
  const [exo, setExo] = useState({ libelle: "", dateDebut: todayISO(), dateFin: todayISO() });
  const [pageJ, setPageJ] = useState(1);
  const [pageB, setPageB] = useState(1);
  const perPage = 10;

  const { data: journal } = useQuery({ queryKey: ["journal"], queryFn: api.journalList });
  const { data: balance } = useQuery({ queryKey: ["balance"], queryFn: api.balance });
  const { data: livre } = useQuery({ queryKey: ["livre", compte], queryFn: () => api.grandLivre(compte) });
  const { data: exercices } = useQuery({ queryKey: ["exercices"], queryFn: api.exercicesList });
  const { data: bilan } = useQuery({ queryKey: ["ohada-bilan"], queryFn: api.ohadaBilan, enabled: tab === "ohada" });
  const { data: resultat } = useQuery({ queryKey: ["ohada-resultat"], queryFn: api.ohadaResultat, enabled: tab === "ohada" });
  const { data: tva } = useQuery({ queryKey: ["ohada-tva"], queryFn: api.ohadaTva, enabled: tab === "ohada" });

  const createEcr = useMutation({
    mutationFn: () => api.ecritureCreate({ ...ecr, piece_ref: "" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["journal"] });
      qc.invalidateQueries({ queryKey: ["balance"] });
      qc.invalidateQueries({ queryKey: ["livre"] });
      setEcr({ journal_code: "OD", compte_numero: "", date_ecriture: todayISO(), libelle: "", debit: 0, credit: 0 });
      toast.success("Écriture enregistrée");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const createExo = useMutation({
    mutationFn: () => api.exerciceCreate(exo.libelle, exo.dateDebut, exo.dateFin),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["exercices"] }); toast.success("Exercice créé"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const cloturer = useMutation({
    mutationFn: (id: string) => api.exerciceCloturer(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["exercices"] }); toast.success("Exercice clôturé"); },
  });

  return (
    <div>
      <PageHeader title="Comptabilité" subtitle="Journal, balance, grand livre, exercices" />
      <Tabs<"journal" | "balance" | "livre" | "exercices" | "ohada">
        active={tab} onChange={setTab}
        tabs={[
          { key: "journal", label: "Journal & saisie", icon: ListOrdered },
          { key: "balance", label: "Balance", icon: Scale },
          { key: "livre", label: "Grand livre", icon: BookOpen },
          { key: "ohada", label: "États OHADA", icon: PieChart },
          { key: "exercices", label: "Exercices", icon: Lock },
        ]}
      />
      {tab === "journal" && (
        <Card>
          <h3 className="font-semibold mb-2">Saisie d'écriture</h3>
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-2">
            <select className="px-3 py-2 border rounded-lg text-sm" value={ecr.journal_code} onChange={(e) => setEcr({ ...ecr, journal_code: e.target.value })}>
              {JOURNAUX.map((j) => <option key={j.code} value={j.code}>{j.code} — {j.label}</option>)}
            </select>
            <Input placeholder="N° compte (ex 411000)" value={ecr.compte_numero} onChange={(e) => setEcr({ ...ecr, compte_numero: e.target.value })} />
            <Input type="date" value={ecr.date_ecriture} onChange={(e) => setEcr({ ...ecr, date_ecriture: e.target.value })} />
            <Input placeholder="Libellé" value={ecr.libelle} onChange={(e) => setEcr({ ...ecr, libelle: e.target.value })} />
            <Input type="number" placeholder="Débit" value={ecr.debit} onChange={(e) => setEcr({ ...ecr, debit: +e.target.value })} />
            <Input type="number" placeholder="Crédit" value={ecr.credit} onChange={(e) => setEcr({ ...ecr, credit: +e.target.value })} />
          </div>
          <Button className="mt-2" onClick={() => createEcr.mutate()} disabled={!ecr.compte_numero || !ecr.libelle}><Check size={15} /> Enregistrer</Button>

          <h3 className="font-semibold mt-4 mb-2">Journal des écritures</h3>
          <DataView
            data={paginate(journal || [], pageJ, perPage)}
            empty="Aucune écriture."
            title={(e) => e.libelle}
            subtitle={(e) => `${e.date_ecriture} • Journal ${e.journal_code} • Compte ${e.compte_numero}`}
            fields={[
              { label: "Débit", value: (e) => (e.debit ? fmtMoney(e.debit) : "-") },
              { label: "Crédit", value: (e) => (e.credit ? fmtMoney(e.credit) : "-") },
            ]}
          />
          <Pagination page={pageJ} perPage={perPage} total={(journal || []).length}
            onChange={(p) => setPageJ(p)} />
        </Card>
      )}
      {tab === "balance" && (
        <Card>
          <h3 className="font-semibold mb-2">Balance des comptes</h3>
            <DataView
              data={paginate(balance || [], pageB, perPage)}
              empty="Aucune écriture comptable."
              title={(b: any) => `${b.numero} — ${b.intitule}`}
              subtitle={() => "Compte du plan comptable"}
              fields={[
                { label: "Total débit", value: (b: any) => fmtMoney(b.debit) },
                { label: "Total crédit", value: (b: any) => fmtMoney(b.credit) },
                { label: "Solde", value: (b: any) => fmtMoney(b.solde) },
              ]}
            />
            <Pagination page={pageB} perPage={perPage} total={(balance || []).length}
              onChange={(p) => setPageB(p)} />
          </Card>
        )}
        {tab === "livre" && (
          <Card>
            <h3 className="font-semibold mb-2">Grand livre</h3>
            <select className="px-3 py-2 border rounded-lg text-sm mb-2" value={compte} onChange={(e) => setCompte(e.target.value)}>
              {(balance || []).map((b: any) => <option key={b.numero} value={b.numero}>{b.numero} — {b.intitule}</option>)}
            </select>
            <DataView
              data={livre || []}
              empty="Aucune écriture sur ce compte."
              title={(e) => e.libelle}
              subtitle={(e) => `${e.date_ecriture} • Journal ${e.journal_code}`}
              fields={[
                { label: "Débit", value: (e) => (e.debit ? fmtMoney(e.debit) : "-") },
                { label: "Crédit", value: (e) => (e.credit ? fmtMoney(e.credit) : "-") },
              ]}
            />
          </Card>
        )}
        {tab === "exercices" && (
          <Card>
            <h3 className="font-semibold mb-2">Exercices comptables</h3>
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-2 mb-2">
              <Input placeholder="Libellé (ex 2026)" value={exo.libelle} onChange={(e) => setExo({ ...exo, libelle: e.target.value })} />
              <Input type="date" value={exo.dateDebut} onChange={(e) => setExo({ ...exo, dateDebut: e.target.value })} />
              <Input type="date" value={exo.dateFin} onChange={(e) => setExo({ ...exo, dateFin: e.target.value })} />
              <Button onClick={() => createExo.mutate()} disabled={!exo.libelle}><Plus size={15} /> Créer</Button>
            </div>
            {(exercices || []).map((x: any) => (
              <div key={x.id} className="flex justify-between text-sm py-1 border-b">
                <span>{x.libelle} ({x.date_debut} → {x.date_fin}) {x.cloture ? "🔒" : ""}</span>
                {!x.cloture && <button className="text-amber-600 inline-flex items-center gap-1 text-xs" onClick={() => cloturer.mutate(x.id)}><Lock size={13} /> Clôturer</button>}
              </div>
            ))}
          </Card>
        )}
        {tab === "ohada" && (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
            <Card>
              <h3 className="font-semibold mb-2">Bilan simplifié (SYSCOHADA)</h3>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <h4 className="text-xs font-bold uppercase text-slate-500 mb-1">Actif — {fmtMoney(bilan?.total_actif || 0)}</h4>
                  {(bilan?.detail_actif || []).map((a: any) => (
                    <div key={a.numero} className="flex justify-between py-0.5 border-b">
                      <span>{a.numero} {a.intitule}</span><b>{fmtMoney(a.montant)}</b>
                    </div>
                  ))}
                </div>
                <div>
                  <h4 className="text-xs font-bold uppercase text-slate-500 mb-1">Passif — {fmtMoney(bilan?.total_passif || 0)}</h4>
                  {(bilan?.detail_passif || []).map((a: any) => (
                    <div key={a.numero} className="flex justify-between py-0.5 border-b">
                      <span>{a.numero} {a.intitule}</span><b>{fmtMoney(a.montant)}</b>
                    </div>
                  ))}
                </div>
              </div>
              <p className="text-xs mt-2 text-slate-500">Équilibre : {(bilan?.total_actif || 0) - (bilan?.total_passif || 0) === 0 ? "✓ Actif = Passif" : "Écart (écritures non équilibrées ou hors plan)"}</p>
            </Card>
            <div className="space-y-4">
              <Card>
                <h3 className="font-semibold mb-2">Compte de résultat</h3>
                <div className="flex justify-between text-sm py-1 border-b"><span>Total produits (classe 7)</span><b className="text-green-700">{fmtMoney(resultat?.total_produits || 0)}</b></div>
                <div className="flex justify-between text-sm py-1 border-b"><span>Total charges (classe 6)</span><b className="text-red-600">{fmtMoney(resultat?.total_charges || 0)}</b></div>
                <div className="flex justify-between font-bold py-1"><span>Résultat net</span><span className={(resultat?.resultat_net || 0) >= 0 ? "text-green-700" : "text-red-600"}>{fmtMoney(resultat?.resultat_net || 0)}</span></div>
              </Card>
              <Card>
                <h3 className="font-semibold mb-2">État TVA</h3>
                <div className="flex justify-between text-sm py-1 border-b"><span>TVA collectée (ventes)</span><b>{fmtMoney(tva?.collectee || 0)}</b></div>
                <div className="flex justify-between text-sm py-1 border-b"><span>TVA déductible (achats)</span><b>{fmtMoney(tva?.deductible || 0)}</b></div>
                <div className="flex justify-between font-bold py-1"><span>TVA due</span><span>{fmtMoney(tva?.due || 0)}</span></div>
              </Card>
            </div>
          </div>
        )}
    </div>
  );
}
