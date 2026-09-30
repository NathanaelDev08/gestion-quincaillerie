import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Pencil, Plus, Printer, RotateCcw, Search, Tags, Trash2, X } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Badge, IconBtn, confirmDelete, DataView, Select, Pagination, toast } from "../components/ui";
import { EtiquettesModal, CodeBarre } from "../components/Etiquettes";
import { fmtMoney } from "../utils/format";
import { getDefaultTva } from "../stores/prefs";
import { CATEGORIES_QUINCAILLERIE, UNITES_QUINCAILLERIE, CATALOGUE_INITIAL } from "../data/quincaillerie";

export default function Produits() {
  const qc = useQueryClient();
  const [form, setForm] = useState<any>({ reference: "", designation: "", prix_vente_ht: 0, prix_achat_ht: 0, taux_tva: getDefaultTva(), stock: 0, stock_alerte: 5, categorie: "Quincaillerie bâtiment", unite: "pièce" });
  const [editId, setEditId] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [cat, setCat] = useState("");
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(20);
  const [showEtiquettes, setShowEtiquettes] = useState(false);
  const { data, isLoading, isError, refetch } = useQuery({ queryKey: ["produits", page, perPage], queryFn: () => api.produitsList(page, perPage) });
  const { data: cats } = useQuery({ queryKey: ["categories"], queryFn: api.produitsCategories });
  const filtered = (data?.data ?? []).filter((p) =>
    (!q || (p.designation + " " + p.reference).toLowerCase().includes(q.toLowerCase())) &&
    (!cat || p.categorie === cat)
  );
  const reset = () => { setEditId(null); setForm({ reference: "", designation: "", prix_vente_ht: 0, prix_achat_ht: 0, taux_tva: getDefaultTva(), stock: 0, stock_alerte: 5, categorie: "Quincaillerie bâtiment", unite: "pièce" }); };
  const importerCatalogue = useMutation({
    mutationFn: async () => {
      let n = 0;
      for (const a of CATALOGUE_INITIAL) {
        try { await api.produitsCreate({ ...a, taux_tva: getDefaultTva(), id: "", actif: 1 }); n++; } catch { /* déjà existant */ }
      }
      return n;
    },
    onSuccess: (n) => { qc.invalidateQueries({ queryKey: ["produits"] }); toast.success(`${n} articles quincaillerie importés`); },
  });
  const create = useMutation({
    mutationFn: () => api.produitsCreate({ ...form, id: "", actif: 1 }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["produits"] }); reset(); },
  });
  const update = useMutation({
    mutationFn: () => api.produitsUpdate(editId!, { ...form }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["produits"] }); reset(); },
  });
  const reactiver = useMutation({
    mutationFn: (p: any) => api.produitsUpdate(p.id, { ...p, actif: 1 }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["produits"] }),
  });
  const del = useMutation({ mutationFn: (id: string) => api.produitsDelete(id), onSuccess: () => qc.invalidateQueries({ queryKey: ["produits"] }) });
  return (
    <div>
      <PageHeader title="Articles Quincaillerie" subtitle={`${filtered.length} articles`}
        actions={
          <div className="flex gap-1 flex-wrap">
            <Input placeholder="Rechercher (réf, désignation, code-barres)..." value={q} onChange={(e) => setQ(e.target.value)} className="w-56" />
            <Select value={cat} onChange={(e) => setCat(e.target.value)} title="Filtrer par catégorie">
              <option value="">Toutes catégories</option>
              {[...new Set([...CATEGORIES_QUINCAILLERIE, ...((cats as string[]) || [])])].map((c) => <option key={c} value={c}>{c}</option>)}
            </Select>
            <Button className="bg-emerald-600" onClick={() => importerCatalogue.mutate()} title="Importer 20 articles quincaillerie de démarrage"><Plus size={15} /> Catalogue</Button>
            <Button className="bg-slate-600" onClick={() => setShowEtiquettes(true)} title="Imprimer des étiquettes codes-barres"><Tags size={15} /> Étiquettes</Button>
          </div>
        } />
      <Card>
        <div className="grid grid-cols-2 lg:grid-cols-5 gap-2 mb-3">
          <Input placeholder="Référence *" value={form.reference} onChange={(e) => setForm({ ...form, reference: e.target.value })} />
          <Input placeholder="Désignation *" value={form.designation} onChange={(e) => setForm({ ...form, designation: e.target.value })} />
          <Input type="number" placeholder="PV HT (F CFA)" value={form.prix_vente_ht} onChange={(e) => setForm({ ...form, prix_vente_ht: +e.target.value })} />
          <Input type="number" placeholder="PA HT (F CFA)" value={form.prix_achat_ht} onChange={(e) => setForm({ ...form, prix_achat_ht: +e.target.value })} />
          <Input type="number" placeholder="TVA %" value={form.taux_tva} onChange={(e) => setForm({ ...form, taux_tva: +e.target.value })} />
          <Input type="number" placeholder="Stock initial" value={form.stock} onChange={(e) => setForm({ ...form, stock: +e.target.value })} />
          <Input type="number" placeholder="Seuil d'alerte" value={form.stock_alerte} onChange={(e) => setForm({ ...form, stock_alerte: +e.target.value })} />
          <Input placeholder="Catégorie" value={form.categorie} onChange={(e) => setForm({ ...form, categorie: e.target.value })} list="cats-quinca" />
          <Input placeholder="Unité (pièce, sac, kg…)" value={form.unite} onChange={(e) => setForm({ ...form, unite: e.target.value })} list="unites-quinca" />
          <Input placeholder="Code-barres (EAN13)" value={form.code_barre || ""} onChange={(e) => setForm({ ...form, code_barre: e.target.value })} />
        </div>
        <datalist id="cats-quinca">{CATEGORIES_QUINCAILLERIE.map((c) => <option key={c} value={c} />)}</datalist>
        <datalist id="unites-quinca">{UNITES_QUINCAILLERIE.map((u) => <option key={u} value={u} />)}</datalist>
        {editId ? (
          <div className="flex gap-2">
            <Button onClick={() => update.mutate()}><Check size={15} /> Enregistrer</Button>
            <Button className="bg-slate-500" onClick={reset}><X size={15} /> Annuler</Button>
          </div>
        ) : (
          <Button onClick={() => create.mutate()} disabled={!form.reference || !form.designation}><Plus size={15} /> Ajouter article</Button>
        )}
        <div className="mt-3">
          <DataView
            data={filtered}
            loading={isLoading}
            error={isError ? true : null}
            onRetry={() => refetch()}
            empty="Aucun article — cliquez sur Catalogue pour importer le stock de démarrage."
            title={(p) => p.designation}
            subtitle={(p) => `Réf. ${p.reference}${p.categorie ? ` • ${p.categorie}` : ""}${p.actif ? "" : " • INACTIF"}`}
            fields={[
              { label: "Prix de vente HT", value: (p) => fmtMoney(p.prix_vente_ht) },
              { label: "Prix d'achat HT", value: (p) => fmtMoney(p.prix_achat_ht) },
              {
                label: "Marge unitaire", value: (p) => {
                  const m = p.prix_vente_ht - p.prix_achat_ht;
                  const t = p.prix_vente_ht > 0 ? (m / p.prix_vente_ht) * 100 : 0;
                  return `${fmtMoney(m)} (${t.toFixed(1)} %)`;
                },
              },
              {
                label: "Stock", value: (p) => (
                  <Badge className={p.stock <= p.stock_alerte ? "bg-red-100 text-red-700" : "bg-green-100 text-green-700"}>{p.stock} {p.unite ?? ""}</Badge>
                ),
              },
              { label: "Code-barres", value: (p) => <CodeBarre value={p.code_barre} /> },
            ]}
            actions={(p) => (<>
              {p.actif
                ? (<>
                  <IconBtn icon={Pencil} title="Modifier" tone="blue" onClick={() => { setEditId(p.id); setForm({ ...p }); }} />
                  <IconBtn icon={Trash2} title="Supprimer / désactiver" tone="red" onClick={() => { if (confirmDelete(p.designation)) del.mutate(p.id); }} />
                </>)
                : (<IconBtn icon={RotateCcw} title="Réactiver le produit" tone="green" onClick={() => reactiver.mutate(p)} />)}
            </>)}
          />
          <Pagination page={page} perPage={perPage} total={data?.total || 0}
            onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
        </div>
      </Card>
      {showEtiquettes && (
        <EtiquettesModal produits={data?.data ?? []} onClose={() => setShowEtiquettes(false)} />
      )}
    </div>
  );
}
