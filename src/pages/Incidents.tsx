import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Download, ShieldAlert, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { Button, Card, PageHeader, Select, DataView, Tabs, toast } from "../components/ui";
import type { Incident } from "../types";

const NIVEAUX = [
  { code: "tout", label: "Tous" },
  { code: "erreur", label: "Erreurs" },
  { code: "avertissement", label: "Avertissements" },
  { code: "securite", label: "Sécurité" },
];

const COULEURS: Record<string, string> = {
  erreur: "bg-red-50 text-red-700",
  avertissement: "bg-amber-50 text-amber-700",
  securite: "bg-purple-50 text-purple-700",
};

export default function Incidents() {
  const qc = useQueryClient();
  const [tab, setTab] = useState<"journal" | "aide">("journal");
  const [niveau, setNiveau] = useState("tout");

  const { data: lignes, isLoading, isError, refetch } = useQuery({
    queryKey: ["incidents", niveau],
    queryFn: () => api.incidentsLister(niveau),
    refetchInterval: 15000,
  });

  const vider = useMutation({
    mutationFn: () => api.incidentsVider(),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["incidents"] }); toast.success("Journal vidé"); },
    onError: (e: any) => toast.error(String(e)),
  });
  const exporter = useMutation({
    mutationFn: () => api.incidentsExporter(),
    onSuccess: (p) => toast.success("Journal exporté : " + p),
    onError: (e: any) => toast.error(String(e)),
  });

  const compte = (n: string) => (lignes || []).filter((i: Incident) => i.niveau === n).length;

  return (
    <div>
      <PageHeader title="Journal d'incidents" subtitle="Ce qui a échoué dans l'application — à transmettre au support" />
      <Tabs<"journal" | "aide">
        active={tab} onChange={setTab}
        tabs={[
          { key: "journal", label: "Incidents", icon: AlertTriangle },
          { key: "aide", label: "Diagnostic", icon: ShieldAlert },
        ]}
      />

      {tab === "journal" && (
        <Card>
          <div className="flex items-center gap-2 mb-2 flex-wrap">
            <Select value={niveau} onChange={(e) => setNiveau(e.target.value)} className="max-w-[220px]">
              {NIVEAUX.map((n) => <option key={n.code} value={n.code}>{n.label}</option>)}
            </Select>
            <span className="text-xs text-slate-400">
              {(lignes || []).length} entrée(s) — 500 maximum conservées
            </span>
            <div className="ml-auto flex gap-2">
              <Button onClick={() => exporter.mutate()} disabled={exporter.isPending}>
                <Download size={15} /> Exporter pour le support
              </Button>
              <Button className="bg-slate-500" onClick={() => { if (window.confirm("Vider le journal ?")) vider.mutate(); }}>
                <Trash2 size={15} /> Vider
              </Button>
            </div>
          </div>

          <div className="flex gap-2 mb-3 text-xs">
            <span className="px-2 py-1 rounded-lg bg-red-50 text-red-700">{compte("erreur")} erreur(s)</span>
            <span className="px-2 py-1 rounded-lg bg-amber-50 text-amber-700">{compte("avertissement")} avertissement(s)</span>
            <span className="px-2 py-1 rounded-lg bg-purple-50 text-purple-700">{compte("securite")} refus(s)</span>
          </div>

          <DataView
            data={lignes || []}
            loading={isLoading}
            error={isError ? true : null}
            onRetry={() => refetch()}
            empty="Aucun incident enregistré. Tout fonctionne normalement."
            title={(i) => i.message}
            subtitle={(i) => `${i.horodatage} • ${i.contexte}`}
            fields={[
              {
                label: "Niveau", value: (i) => (
                  <span className={`text-[11px] px-2 py-0.5 rounded-full ${COULEURS[i.niveau] || "bg-slate-100 text-slate-600"}`}>
                    {i.niveau}
                  </span>
                ),
              },
            ]}
          />
        </Card>
      )}

      {tab === "aide" && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 max-w-[900px]">
          <Card>
            <h3 className="font-semibold mb-2">Que faire quand ça bloque ?</h3>
            <ol className="text-sm space-y-2 list-decimal pl-4">
              <li>Notez le message affiché et l'heure.</li>
              <li>Si une vente a échoué : le stock n'a <b>pas</b> été modifié, c'est normal et sans conséquence.</li>
              <li>Vérifiez que l'application est bien seule ouverte (une restauration exige de fermer la fenêtre).</li>
              <li>Si l'erreur persiste : <b>Sauvegarder</b> puis <b>Paramètres → Sauvegarde</b> pour conserver une copie saine.</li>
              <li>Exportez ce journal et envoyez-le au support : il contient la cause exacte.</li>
            </ol>
          </Card>
          <Card>
            <h3 className="font-semibold mb-2">Garanties de l'application</h3>
            <ul className="text-sm space-y-1.5">
              <li><b>Vente atomique</b> : facture, stock, règlement et comptabilité réussissent ensemble ou sont annulés ensemble.</li>
              <li><b>Numérotation sans trou</b> : une vente annulée ne laisse pas de trou dans les numéros de facture.</li>
              <li><b>Restauration sécurisée</b> : une copie de la base courante est conservée avant tout remplacement.</li>
              <li><b>Sauvegarde vérifiée</b> : chaque sauvegarde est accompagnée d'une empreinte SHA256.</li>
              <li><b>Aucune vente sans session</b> : chaque action est contrôlée côté serveur, pas seulement dans l'écran.</li>
            </ul>
          </Card>
        </div>
      )}
    </div>
  );
}
