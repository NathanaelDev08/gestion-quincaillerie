import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, Banknote, BellRing, FileText, Percent, ShoppingBag, ShoppingCart, TrendingUp, Wallet, Warehouse } from "lucide-react";
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { api } from "../services/api";
import { Card, PageHeader, DataView } from "../components/ui";
import { fmtCompact, fmtMoney, fmtMois } from "../utils/format";

function Kpi({ icon: Icon, label, value, sub, tone }: {
  icon: any; label: string; value: string; sub?: string;
  tone?: "blue" | "green" | "amber" | "red" | "slate";
}) {
  const tones: Record<string, string> = {
    blue: "bg-blue-50 text-blue-700",
    green: "bg-green-50 text-green-700",
    amber: "bg-amber-50 text-amber-700",
    red: "bg-red-50 text-red-600",
    slate: "bg-slate-100 text-slate-600",
  };
  return (
    <Card className="!p-3">
      <div className="flex items-center gap-2 mb-1">
        <span className={`p-1.5 rounded-lg ${tones[tone || "slate"]}`}><Icon size={15} /></span>
        <span className="text-xs text-slate-500">{label}</span>
      </div>
      <div className="text-lg font-bold leading-tight">{value}</div>
      {sub && <div className="text-[11px] text-slate-400 mt-0.5">{sub}</div>}
    </Card>
  );
}

export default function Dashboard() {
  const { data: stats, dataUpdatedAt } = useQuery({ queryKey: ["stats"], queryFn: api.dashboardStats, refetchInterval: 30000 });
  const { data: ca } = useQuery({ queryKey: ["ca"], queryFn: api.caMensuel, refetchInterval: 30000 });
  const { data: dep } = useQuery({ queryKey: ["dep-mens"], queryFn: api.depensesMensuelles, refetchInterval: 30000 });
  const { data: marges } = useQuery({ queryKey: ["marges"], queryFn: api.marges, refetchInterval: 30000 });
  const { data: topP } = useQuery({ queryKey: ["topP"], queryFn: api.topProduits, refetchInterval: 30000 });
  const { data: topC } = useQuery({ queryKey: ["topC"], queryFn: api.topClients, refetchInterval: 30000 });
  const { data: retard } = useQuery({ queryKey: ["retard"], queryFn: api.facturesRetard, refetchInterval: 30000 });
  const { data: dernieres } = useQuery({ queryKey: ["factures", 1, 5], queryFn: () => api.facturesList(1, 5) });
  const { data: commandes } = useQuery({ queryKey: ["commandes"], queryFn: api.commandesList, refetchInterval: 30000 });
  const { data: dettes } = useQuery({ queryKey: ["dettes"], queryFn: api.ffList, refetchInterval: 30000 });
  const actualise = new Date(dataUpdatedAt || Date.now()).toLocaleTimeString("fr-FR");

  const graphe = (ca || []).map((c) => {
    const d = (dep || []).find((x) => x.mois === c.mois);
    return { mois: fmtMois(c.mois), CA: Math.round(c.ca), Dépenses: Math.round(d?.ca || 0) };
  });

  const nbRetard = retard?.length || 0;
  const totalDettes = (dettes || []).reduce((s: number, d: any) => s + Math.max(0, (d.total_ttc || 0) - (d.montant_paye || 0)), 0);
  const commandesEnCours = (commandes || []).filter((c: any) => ["brouillon", "validee", "partielle"].includes(c.statut)).length;

  return (
    <div>
      <PageHeader title="Tableau de bord" subtitle={`Votre quincaillerie en Franc CFA (XOF) • Actualisé à ${actualise}`} />

      {/* Raccourcis vente / achat */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2.5 mb-3">
        <Link to="/caisse" className="flex items-center gap-2 px-3 py-2.5 rounded-xl bg-green-600 text-white font-medium hover:bg-green-700">
          <ShoppingBag size={16} /> Nouvelle vente (caisse)
        </Link>
        <Link to="/achats" className="flex items-center gap-2 px-3 py-2.5 rounded-xl bg-blue-700 text-white font-medium hover:bg-blue-800">
          <ShoppingCart size={16} /> Nouvel achat (commande)
        </Link>
        <Link to="/factures" className="flex items-center gap-2 px-3 py-2.5 rounded-xl bg-white border font-medium text-slate-700 hover:bg-slate-50">
          <FileText size={16} /> Factures clients
        </Link>
        <Link to="/achats" className="flex items-center gap-2 px-3 py-2.5 rounded-xl bg-white border font-medium text-slate-700 hover:bg-slate-50">
          <Wallet size={16} /> Dettes : {fmtCompact(totalDettes)}
        </Link>
      </div>

      {nbRetard > 0 && (
        <Link to="/relances" className="flex items-center gap-2 mb-3 px-3 py-2 rounded-xl bg-amber-50 border border-amber-200 text-sm text-amber-800 hover:bg-amber-100">
          <BellRing size={16} />
          <span><b>{nbRetard} facture{nbRetard > 1 ? "s" : ""} en retard</b> — pensez à relancer vos clients.</span>
        </Link>
      )}

      <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-2.5 mb-3">
        <Kpi icon={Banknote} tone="blue" label="CA total" value={fmtCompact(stats?.ca_total || 0)} sub="Toutes factures non annulées" />
        <Kpi icon={TrendingUp} tone="green" label="CA ce mois" value={fmtCompact(stats?.ca_mois || 0)} sub={fmtMoney(stats?.ca_mois || 0)} />
        <Kpi icon={Percent} tone="green" label="Marge du mois" value={fmtCompact(marges?.marge_mois || 0)} sub={`Taux : ${(marges?.taux_marge_mois || 0).toFixed(1)} %`} />
        <Kpi icon={AlertTriangle} tone="red" label="Impayés" value={fmtCompact(stats?.factures_impayees || 0)} sub={`${nbRetard} facture(s) en retard`} />
        <Kpi icon={Wallet} tone="amber" label="Clients / Produits" value={`${stats?.nb_clients || 0} / ${stats?.nb_produits || 0}`} sub={`${stats?.nb_devis_en_cours || 0} devis en cours`} />
        <Kpi icon={Warehouse} tone="slate" label="Stock valorisé" value={fmtCompact(stats?.stock_valeur || 0)} sub={`${stats?.alertes_stock || 0} alerte(s) seuil`} />
      </div>

      <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-2.5 mb-3">
        <Kpi icon={ShoppingCart} tone="blue" label="Commandes en cours" value={String(commandesEnCours)} sub={`${(commandes || []).length} bons au total`} />
        <Kpi icon={Wallet} tone="amber" label="Dettes fournisseurs" value={fmtCompact(totalDettes)} sub={fmtMoney(totalDettes)} />
        <Kpi icon={ShoppingBag} tone="green" label="Devis en cours" value={String(stats?.nb_devis_en_cours || 0)} sub="À transformer en facture" />
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
        <Card>
          <h3 className="font-semibold mb-2">Chiffre d'affaires vs dépenses (12 mois)</h3>
          <div style={{ height: 200 }}>
            <ResponsiveContainer>
              <BarChart data={graphe}>
                <XAxis dataKey="mois" fontSize={10} interval={1} />
                <YAxis fontSize={10} tickFormatter={(v: number) => v >= 1000000 ? `${(v / 1000000).toFixed(1)} M` : `${Math.round(v / 1000)} k`} />
                <Tooltip formatter={(v: any) => fmtMoney(+v)} />
                <Bar dataKey="CA" fill="#1e40af" radius={[3, 3, 0, 0]} />
                <Bar dataKey="Dépenses" fill="#f59e0b" radius={[3, 3, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </Card>
        <div className="space-y-4">
          <Card>
            <h3 className="font-semibold mb-2">Points d'attention</h3>
            <div className="text-sm space-y-1.5">
              <div className="flex justify-between"><span>Factures en retard</span><Link to="/relances" className="font-bold text-red-600">{nbRetard} → relancer</Link></div>
              <div className="flex justify-between"><span>Alertes stock</span><Link to="/stock" className="font-bold text-amber-600">{stats?.alertes_stock || 0} article(s)</Link></div>
              <div className="flex justify-between"><span>Devis en cours</span><Link to="/devis" className="font-bold text-blue-700">{stats?.nb_devis_en_cours || 0} devis</Link></div>
              <div className="flex justify-between"><span>Marge totale cumulée</span><b>{fmtCompact(marges?.marge_totale || 0)}</b></div>
            </div>
          </Card>
          <Card>
            <h3 className="font-semibold mb-2">Dernières factures</h3>
            <DataView
              data={dernieres?.data || []} empty="Aucune facture."
              title={(f) => f.numero}
              subtitle={(f) => f.client_nom ?? ""}
              fields={[{ label: "Total TTC", value: (f) => fmtMoney(f.total_ttc) }]}
            />
          </Card>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-4 mt-4">
        <Card>
          <h3 className="font-semibold mb-2 flex items-center gap-1.5"><FileText size={15} /> Top produits</h3>
          <DataView
            data={(topP || []).slice(0, 5)} empty="Pas encore de ventes."
            title={(t) => t.nom}
            subtitle={(t) => t.quantite ? `Quantité vendue : ${t.quantite}` : "Chiffre d'affaires"}
            fields={[{ label: "Total vendu", value: (t) => fmtMoney(t.total) }]}
          />
        </Card>
        <Card>
          <h3 className="font-semibold mb-2">Top clients</h3>
          <DataView
            data={(topC || []).slice(0, 5)} empty="Pas encore de ventes."
            title={(t) => t.nom}
            subtitle={() => "Chiffre d'affaires"}
            fields={[{ label: "Total acheté", value: (t) => fmtMoney(t.total) }]}
          />
        </Card>
      </div>
    </div>
  );
}
