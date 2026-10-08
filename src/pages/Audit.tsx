import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { History, ShieldCheck, Trash2, Filter } from "lucide-react";
import { api } from "../services/api";
import { Button, Card, PageHeader, Select, DataView, Tabs, toast } from "../components/ui";
import { fmtMoney } from "../utils/format";

const ACTION_LABELS: Record<string, string> = {
  vente_comptoir: "Vente comptoir",
  transfert_depot: "Transfert dépôt",
};

export default function Audit() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"journal" | "actions">("journal");
  const [action, setAction] = useState("");

  const { data: rows, isLoading, isError, refetch } = useQuery({
    queryKey: ["audit", action],
    queryFn: () => api.auditList(action, 300),
  });
  const { data: actions } = useQuery({ queryKey: ["audit-actions"], queryFn: api.auditActions });

  const purge = useMutation({
    mutationFn: () => api.auditPurge(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["audit"] });
      qc.invalidateQueries({ queryKey: ["audit-actions"] });
      toast.success("Journal purgé");
    },
    onError: (e: any) => toast.error(String(e)),
  });

  return (
    <div>
      <PageHeader title="Journal d'audit" subtitle="Qui a fait quoi, quand et pour quel montant — traçabilité complète" />
      <Tabs<"journal" | "actions">
        active={tab} onChange={setTab}
        tabs={[
          { key: "journal", label: "Historique", icon: History },
          { key: "actions", label: "Types d'actions", icon: Filter },
        ]}
      />

      {tab === "journal" && (
        <Card>
          <div className="flex items-center gap-2 mb-2 flex-wrap">
            <Select value={action} onChange={(e) => setAction(e.target.value)} className="max-w-[220px]">
              <option value="">Toutes les actions</option>
              {(actions || []).map((a) => <option key={a} value={a}>{ACTION_LABELS[a] ?? a}</option>)}
            </Select>
            <span className="text-xs text-slate-400 ml-auto">{(rows || []).length} entrée(s) — 300 max</span>
            <Button className="bg-red-600" onClick={() => { if (window.confirm("Purger tout le journal d'audit ?")) purge.mutate(); }}>
              <Trash2 size={15} /> Purger
            </Button>
          </div>
          <DataView
            data={rows || []}
            loading={isLoading}
            error={isError ? true : null}
            onRetry={() => refetch()}
            empty="Aucune action enregistrée pour l'instant. Les ventes en caisse et transferts apparaîtront ici."
            title={(r) => ACTION_LABELS[r.action] ?? r.action}
            subtitle={(r) => `${r.utilisateur} (${r.role}) • ${r.created_at}${r.entite_id ? ` • ${r.entite_id}` : ""}`}
            fields={[
              { label: "Détail", value: (r) => r.detail || "—" },
              { label: "Montant", value: (r) => (r.montant != null ? fmtMoney(r.montant) : "—") },
            ]}
          />
        </Card>
      )}

      {tab === "actions" && (
        <Card>
          <h3 className="font-semibold mb-2 flex items-center gap-1.5"><ShieldCheck size={16} /> Actions tracées</h3>
          <p className="text-sm text-slate-500 mb-2">
            Réservé aux administrateurs. Chaque action sensible est enregistrée avec l'utilisateur, son rôle et le montant concerné.
          </p>
          <div className="space-y-1 text-sm">
            {(actions || []).length === 0 && <p className="text-slate-400">Aucune action enregistrée.</p>}
            {(actions || []).map((a) => (
              <div key={a} className="flex justify-between py-1 border-b">
                <span className="font-medium">{ACTION_LABELS[a] ?? a}</span>
                <span className="text-slate-400 text-xs">{a}</span>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
