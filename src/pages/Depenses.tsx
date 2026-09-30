import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, List, PieChart as PieIcon, Plus, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, confirmDelete, DataView, Select, Field, Pagination, paginate, Tabs, toast } from "../components/ui";
import { fmtMoney, fmtDate, todayISO, MODES_REGLEMENT, labelOf, CATEGORIES_DEPENSE, catDepenseLabel } from "../utils/format";

export default function Depenses() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"historique" | "nouvelle" | "categories">("historique");
  const [form, setForm] = useState<any>({ libelle: "", categorie: "divers", montant: 0, date_depense: todayISO(), mode: "especes", piece_ref: "" });
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  const { data: depenses } = useQuery({ queryKey: ["depenses"], queryFn: api.depensesList });
  const { data: parCat } = useQuery({ queryKey: ["depenses-cat"], queryFn: api.depensesParCategorie });

  const totalMois = (parCat || []).reduce((s, x) => s + x.total, 0);

  const create = useMutation({
    mutationFn: () => api.depensesCreate({ ...form, fournisseur_id: null, notes: "" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["depenses"] });
      qc.invalidateQueries({ queryKey: ["depenses-cat"] });
      setForm({ libelle: "", categorie: "divers", montant: 0, date_depense: todayISO(), mode: "especes", piece_ref: "" });
      setTab("historique");
      toast.success("Dépense enregistrée");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const del = useMutation({
    mutationFn: (d: any) => api.depensesDelete(d.id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["depenses"] }); qc.invalidateQueries({ queryKey: ["depenses-cat"] }); },
  });

  return (
    <div>
      <PageHeader title="Dépenses" subtitle={`Total ce mois : ${fmtMoney(totalMois)}`} />
      <Tabs<"historique" | "nouvelle" | "categories">
        active={tab} onChange={setTab}
        tabs={[
          { key: "historique", label: "Historique", icon: List },
          { key: "nouvelle", label: "Nouvelle dépense", icon: Plus },
          { key: "categories", label: "Par catégorie", icon: PieIcon },
        ]}
      />
      {tab === "historique" && (
        <Card>
          <DataView
            data={paginate(depenses || [], page, perPage)}
            empty="Aucune dépense enregistrée."
            title={(d) => d.libelle}
            subtitle={(d) => `${catDepenseLabel(d.categorie)} • ${fmtDate(d.date_depense)}`}
            fields={[
              { label: "Montant", value: (d) => fmtMoney(d.montant) },
              { label: "Date", value: (d) => fmtDate(d.date_depense) },
              { label: "Mode", value: (d) => (<span className="whitespace-nowrap">{labelOf(MODES_REGLEMENT, d.mode)}</span>) },
            ]}
            actions={(d) => (
              <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(d.libelle)) del.mutate(d); }} />
            )}
          />
          <Pagination page={page} perPage={perPage} total={(depenses || []).length}
            onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
        </Card>
      )}
      {tab === "nouvelle" && (
        <Card>
          <h3 className="font-semibold mb-2">Nouvelle dépense</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-w-2xl">
            <Field label="Libellé *"><Input value={form.libelle} onChange={(e) => setForm({ ...form, libelle: e.target.value })} placeholder="Ex. Loyer bureau" /></Field>
            <Field label="Montant (F CFA) *"><Input type="number" value={form.montant} onChange={(e) => setForm({ ...form, montant: +e.target.value })} /></Field>
            <Field label="Catégorie">
              <Select value={form.categorie} onChange={(e) => setForm({ ...form, categorie: e.target.value })}>
                {CATEGORIES_DEPENSE.map((c) => <option key={c.code} value={c.code}>{c.label}</option>)}
              </Select>
            </Field>
            <Field label="Date"><Input type="date" value={form.date_depense} onChange={(e) => setForm({ ...form, date_depense: e.target.value })} /></Field>
            <Field label="Mode de paiement">
              <Select value={form.mode} onChange={(e) => setForm({ ...form, mode: e.target.value })}>
                {MODES_REGLEMENT.map((m) => <option key={m.code} value={m.code}>{m.label}</option>)}
              </Select>
            </Field>
            <Field label="Réf. pièce"><Input value={form.piece_ref} onChange={(e) => setForm({ ...form, piece_ref: e.target.value })} /></Field>
          </div>
          <div className="flex justify-end gap-2 mt-3 max-w-2xl">
            <Button onClick={() => create.mutate()} disabled={!form.libelle || !form.montant}><Check size={15} /> Enregistrer la dépense</Button>
          </div>
        </Card>
      )}
      {tab === "categories" && (
        <Card>
          <h3 className="font-semibold mb-2">Par catégorie (mois en cours — total {fmtMoney(totalMois)})</h3>
          <DataView
            data={parCat || []}
            empty="Aucune dépense ce mois-ci."
            title={(c) => catDepenseLabel(c.categorie)}
            subtitle={(c) => totalMois > 0 ? `${((c.total / totalMois) * 100).toFixed(1)} % des dépenses du mois` : ""}
            fields={[
              { label: "Total dépensé", value: (c) => fmtMoney(c.total) },
              {
                label: "Part du mois", value: (c) => {
                  const pct = totalMois > 0 ? (c.total / totalMois) * 100 : 0;
                  return (
                    <span className="flex items-center gap-1.5 min-w-[110px]">
                      <span className="flex-1 h-1.5 rounded bg-slate-100 overflow-hidden">
                        <span className="block h-full rounded bg-amber-500" style={{ width: `${Math.min(100, pct)}%` }} />
                      </span>
                      <b className="text-xs">{pct.toFixed(0)} %</b>
                    </span>
                  );
                },
              },
            ]}
          />
        </Card>
      )}
    </div>
  );
}
