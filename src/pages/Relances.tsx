import { useState } from "react";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-shell";
import { BellRing, Megaphone, MessageCircle, Printer } from "lucide-react";
import { api } from "../services/api";
import { Button, Card, PageHeader, DataView, Pagination, paginate, toast } from "../components/ui";
import { fmtMoney, fmtDate } from "../utils/format";

export default function Relances() {
  const qc = useQueryClient();
  const [page, setPage] = useState(1);
  const perPage = 10;
  const { data: retard } = useQuery({ queryKey: ["retard"], queryFn: api.facturesRetard, refetchInterval: 30000 });

  const relancer = useMutation({
    mutationFn: (id: string) => api.facturesRelancer(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["retard"] }); toast.success("Relance enregistrée"); },
  });
  const toutRelancer = useMutation({
    mutationFn: () => api.toutRelancer(),
    onSuccess: (n) => { qc.invalidateQueries({ queryKey: ["retard"] }); toast.success(`${n} facture(s) relancée(s)`); },
    onError: (e: any) => toast.error(String(e)),
  });

  const whatsApp = async (f: any) => {
    const tel = (f.telephone || "").replace(/[^0-9]/g, "");
    const du = fmtMoney(f.total_ttc - f.montant_paye);
    const texte = `Bonjour ${f.client_nom || ""}, sauf erreur de notre part, la facture ${f.numero} (${du}, échue le ${fmtDate(f.date_echeance)}) reste impayée. Merci de régulariser.`;
    if (!tel) {
      try { await navigator.clipboard.writeText(texte); toast.info("N° manquant : message copié, collez-le dans WhatsApp"); }
      catch { toast.error("Numéro de téléphone manquant pour ce client"); }
      return;
    }
    try {
      await open(`https://wa.me/${tel}?text=${encodeURIComponent(texte)}`);
      relancer.mutate(f.id);
    } catch { toast.error("Impossible d'ouvrir WhatsApp"); }
  };

  const imprimer = () => window.print();

  return (
    <div>
      <PageHeader title="Relances clients" subtitle="Factures échues impayées"
        actions={<>
          <Button className="bg-amber-600" onClick={() => toutRelancer.mutate()} disabled={(retard || []).length === 0}><Megaphone size={15} /> Tout relancer</Button>
          <Button className="bg-slate-600" onClick={imprimer}><Printer size={15} /> Imprimer la liste</Button>
        </>} />
      <DataView
        data={paginate(retard || [], page, perPage)}
        empty="Aucune facture en retard. Bravo !"
        title={(f: any) => f.numero}
        subtitle={(f: any) => f.client_nom ?? ""}
        fields={[
          { label: "Échéance dépassée", value: (f: any) => fmtDate(f.date_echeance) },
          { label: "Reste dû", value: (f: any) => fmtMoney(f.total_ttc - f.montant_paye) },
          { label: "Dernière relance", value: (f: any) => (f.derniere_relance ? fmtDate(f.derniere_relance) : "Jamais relancé") },
        ]}
        actions={(f: any) => (
          <div className="flex gap-1">
            <Button onClick={() => relancer.mutate(f.id)}><BellRing size={15} /> Relancer</Button>
            <Button className="bg-green-600" title="Relancer via WhatsApp" onClick={() => whatsApp(f)}><MessageCircle size={15} /></Button>
          </div>
        )}
      />
      <Pagination page={page} perPage={perPage} total={(retard || []).length}
        onChange={(p) => setPage(p)} />
    </div>
  );
}
