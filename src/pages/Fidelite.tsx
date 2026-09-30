import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Gift, Plus, Power, Star } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, IconBtn, Field, Select, DataView, Tabs, toast } from "../components/ui";
import { fmtMoney, fmtDate, todayISO } from "../utils/format";

export default function Fidelite() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"promos" | "points">("promos");
  const [form, setForm] = useState<any>({ code: "", type: "pourcent", valeur: 10, date_debut: todayISO(), date_fin: todayISO() });

  const { data: promos } = useQuery({ queryKey: ["promos"], queryFn: api.promosList });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 500) });

  const refresh = () => qc.invalidateQueries({ queryKey: ["promos"] });
  const create = useMutation({
    mutationFn: () => api.promosCreate(form),
    onSuccess: () => { refresh(); setForm({ code: "", type: "pourcent", valeur: 10, date_debut: todayISO(), date_fin: todayISO() }); toast.success("Code promo créé"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const toggle = useMutation({
    mutationFn: (id: string) => api.promosToggle(id),
    onSuccess: refresh,
    onError: (e: any) => toast.error(String(e)),
  });

  const topPoints = [...(clients?.data || [])]
    .filter((c: any) => (c.points || 0) > 0)
    .sort((a: any, b: any) => (b.points || 0) - (a.points || 0))
    .slice(0, 20);

  return (
    <div>
      <PageHeader title="Fidélité & promotions" subtitle="Points clients (1 pt / 1000 F, 1 pt = 10 F) et codes promo" />
      <Tabs<"promos" | "points">
        active={tab} onChange={setTab}
        tabs={[
          { key: "promos", label: "Codes promo", icon: Gift },
          { key: "points", label: "Points fidélité", icon: Star },
        ]}
      />
      {tab === "promos" && (
        <div className="space-y-4">
          <Card>
            <h3 className="font-semibold mb-2">Nouveau code promo</h3>
            <div className="grid grid-cols-2 lg:grid-cols-6 gap-2">
              <Input placeholder="CODE (ex SOLDES10)" value={form.code} onChange={(e) => setForm({ ...form, code: e.target.value.toUpperCase() })} className="uppercase" />
              <Select value={form.type} onChange={(e) => setForm({ ...form, type: e.target.value })} title="Type">
                <option value="pourcent">Pourcentage (%)</option>
                <option value="montant">Montant fixe (F CFA)</option>
              </Select>
              <Input type="number" placeholder={form.type === "pourcent" ? "% remise" : "Montant F"} value={form.valeur} onChange={(e) => setForm({ ...form, valeur: +e.target.value })} />
              <Input type="date" value={form.date_debut} onChange={(e) => setForm({ ...form, date_debut: e.target.value })} title="Début" />
              <Input type="date" value={form.date_fin} onChange={(e) => setForm({ ...form, date_fin: e.target.value })} title="Fin" />
              <Button onClick={() => create.mutate()} disabled={!form.code || !form.valeur}><Check size={15} /> Créer</Button>
            </div>
          </Card>
          <Card>
            <DataView
              data={promos || []}
              empty="Aucun code promo."
              title={(p) => p.code}
              subtitle={(p) => `${p.type === "pourcent" ? `${p.valeur} %` : fmtMoney(p.valeur)} • ${fmtDate(p.date_debut)} → ${fmtDate(p.date_fin)}`}
              fields={[{ label: "Statut", value: (p) => (p.actif ? "Actif" : "Inactif") }]}
              actions={(p) => (
                <IconBtn icon={Power} title={p.actif ? "Désactiver" : "Activer"} tone={p.actif ? "amber" : "green"} onClick={() => toggle.mutate(p.id)} />
              )}
            />
          </Card>
        </div>
      )}
      {tab === "points" && (
        <Card>
          <h3 className="font-semibold mb-2">Meilleurs clients fidèles</h3>
          <DataView
            data={topPoints}
            empty="Aucun point cumulé pour l'instant — les points se gagnent à chaque facture soldée."
            title={(c: any) => c.nom}
            subtitle={(c: any) => c.ville || c.email || ""}
            fields={[
              { label: "Points", value: (c: any) => `★ ${c.points || 0}` },
              { label: "Valeur", value: (c: any) => fmtMoney((c.points || 0) * 10) },
            ]}
          />
          <p className="text-xs text-slate-500 mt-2">Conversion en caisse : 1 point = 10 F CFA, dans la limite de 50 % du ticket.</p>
        </Card>
      )}
    </div>
  );
}
