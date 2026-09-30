import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Pencil, Plus, Search, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, PageHeader, IconBtn, Modal, confirmDelete, Field, DataView, Pagination } from "../components/ui";

const EMPTY = { nom: "", entreprise: "", email: "", telephone: "", adresse: "", ville: "", code_postal: "", pays: "France", siret: "", tva_intra: "", notes: "" };

export default function Fournisseurs() {
  const qc = useQueryClient();
  const [modal, setModal] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [form, setForm] = useState<any>({ ...EMPTY });
  const [q, setQ] = useState("");
  const [results, setResults] = useState<any[] | null>(null);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(20);

  const { data, isLoading, isError, refetch } = useQuery({ queryKey: ["fournisseurs", page, perPage], queryFn: () => api.fournisseursList(page, perPage) });
  const rows = results ?? data?.data ?? [];
  const invalidate = () => qc.invalidateQueries({ queryKey: ["fournisseurs"] });

  const save = useMutation({
    mutationFn: () => editId ? api.fournisseursUpdate(editId, { ...form }) : api.fournisseursCreate({ ...form, id: "" }),
    onSuccess: () => { invalidate(); setModal(false); },
  });
  const del = useMutation({
    mutationFn: (f: any) => api.fournisseursDelete(f.id),
    onSuccess: invalidate,
  });

  const openCreate = () => { setEditId(null); setForm({ ...EMPTY }); setModal(true); };
  const openEdit = (f: any) => { setEditId(f.id); setForm({ ...EMPTY, ...f }); setModal(true); };
  const set = (k: string, v: string) => setForm({ ...form, [k]: v });

  return (
    <div>
      <PageHeader title="Fournisseurs" subtitle={`${data?.total || 0} fournisseurs`}
        actions={
          <>
            <div className="flex gap-1">
              <Input placeholder="Rechercher..." value={q} onChange={(e) => { setQ(e.target.value); setResults(null); }} className="w-48" />
              <Button title="Rechercher" onClick={async () => setResults(q ? await api.fournisseursSearch(q) : null)}><Search size={15} /></Button>
            </div>
            <Button onClick={openCreate}><Plus size={15} /> Nouveau</Button>
          </>
        } />
      <DataView
        data={results ?? rows}
        loading={isLoading && !results}
        error={isError && !results ? true : null}
        onRetry={() => refetch()}
        empty="Aucun fournisseur enregistré."
        emptyAction={!results ? <Button onClick={openCreate}><Plus size={15} /> Créer le premier fournisseur</Button> : undefined}
        title={(f) => f.nom}
        subtitle={(f) => f.entreprise || f.ville || ""}
        fields={[
          { label: "Email", value: (f) => f.email || "-" },
          { label: "Téléphone", value: (f) => f.telephone || "-" },
          { label: "Ville", value: (f) => f.ville || "-" },
          { label: "SIRET / RCCM", value: (f) => f.siret || "-" },
        ]}
        actions={(f) => (<>
          <IconBtn icon={Pencil} title="Modifier" tone="blue" onClick={() => openEdit(f)} />
          <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(f.nom)) del.mutate(f); }} />
        </>)}
      />
      {!results && (
        <Pagination page={page} perPage={perPage} total={data?.total || 0}
          onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
      )}

      {modal && (
        <Modal title={editId ? "Modifier le fournisseur" : "Nouveau fournisseur"} onClose={() => setModal(false)} wide>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <Field label="Nom *"><Input placeholder="Nom du fournisseur" value={form.nom} onChange={(e) => set("nom", e.target.value)} /></Field>
            <Field label="Entreprise"><Input placeholder="Raison sociale" value={form.entreprise} onChange={(e) => set("entreprise", e.target.value)} /></Field>
            <Field label="Email"><Input type="email" placeholder="contact@exemple.com" value={form.email} onChange={(e) => set("email", e.target.value)} /></Field>
            <Field label="Téléphone"><Input placeholder="+225 ..." value={form.telephone} onChange={(e) => set("telephone", e.target.value)} /></Field>
            <Field label="Adresse"><Input placeholder="Rue, quartier" value={form.adresse} onChange={(e) => set("adresse", e.target.value)} /></Field>
            <Field label="Ville"><Input placeholder="Ex. Abidjan" value={form.ville} onChange={(e) => set("ville", e.target.value)} /></Field>
            <Field label="Code postal"><Input value={form.code_postal} onChange={(e) => set("code_postal", e.target.value)} /></Field>
            <Field label="SIRET / RCCM"><Input value={form.siret} onChange={(e) => set("siret", e.target.value)} /></Field>
          </div>
          <div className="flex justify-end gap-2 mt-3">
            <Button className="bg-slate-500" onClick={() => setModal(false)}>Annuler</Button>
            <Button onClick={() => save.mutate()} disabled={!form.nom}>Enregistrer</Button>
          </div>
        </Modal>
      )}
    </div>
  );
}
