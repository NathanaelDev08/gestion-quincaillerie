import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Pencil, Plus, Search, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, PageHeader, IconBtn, Modal, confirmDelete, Field, DataView, Pagination } from "../components/ui";

const EMPTY = { nom: "", prenom: "", entreprise: "", email: "", telephone: "", adresse: "", ville: "", code_postal: "", pays: "France", siret: "", tva_intra: "", notes: "" };

export default function Clients() {
  const qc = useQueryClient();
  const [q, setQ] = useState("");
  const [results, setResults] = useState<any[] | null>(null);
  const [modal, setModal] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [form, setForm] = useState<any>({ ...EMPTY });

  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(20);
  const { data, isLoading, isError, refetch } = useQuery({ queryKey: ["clients", page, perPage], queryFn: () => api.clientsList(page, perPage) });
  const invalidate = () => { qc.invalidateQueries({ queryKey: ["clients"] }); setResults(null); };

  const save = useMutation({
    mutationFn: () => editId ? api.clientsUpdate(editId, { ...form }) : api.clientsCreate({ ...form, id: "" }),
    onSuccess: () => { invalidate(); setModal(false); },
  });
  const del = useMutation({
    mutationFn: (c: any) => api.clientsDelete(c.id),
    onSuccess: invalidate,
  });

  const openCreate = () => { setEditId(null); setForm({ ...EMPTY }); setModal(true); };
  const openEdit = (c: any) => { setEditId(c.id); setForm({ ...EMPTY, ...c }); setModal(true); };
  const rows = results ?? data?.data ?? [];
  const set = (k: string, v: string) => setForm({ ...form, [k]: v });

  return (
    <div>
      <PageHeader title="Clients" subtitle={`${data?.total || 0} clients`}
        actions={
          <>
            <div className="flex gap-1">
              <Input placeholder="Rechercher..." value={q} onChange={(e) => setQ(e.target.value)} className="w-48" />
              <Button title="Rechercher" onClick={async () => setResults(q ? await api.clientsSearch(q) : null)}><Search size={15} /></Button>
            </div>
            <Button onClick={openCreate}><Plus size={15} /> Nouveau</Button>
          </>
        } />
      <DataView
        data={rows}
        loading={isLoading && !results}
        error={isError && !results ? true : null}
        onRetry={() => refetch()}
        empty={results ? "Aucun résultat pour cette recherche." : "Aucun client enregistré."}
        emptyAction={!results ? <Button onClick={openCreate}><Plus size={15} /> Créer le premier client</Button> : undefined}
        title={(c) => `${c.nom} ${c.prenom ?? ""}`}
        subtitle={(c) => c.entreprise || c.ville || ""}
        fields={[
          { label: "Email", value: (c) => c.email || "-" },
          { label: "Téléphone", value: (c) => c.telephone || "-" },
          { label: "Ville", value: (c) => c.ville || "-" },
          { label: "Fidélité", value: (c) => `★ ${c.points || 0} pts` },
        ]}
        actions={(c) => (<>
          <IconBtn icon={Pencil} title="Modifier" tone="blue" onClick={() => openEdit(c)} />
          <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(c.nom)) del.mutate(c); }} />
        </>)}
      />
      {!results && (
        <Pagination page={page} perPage={perPage} total={data?.total || 0}
          onChange={(p, pp) => { setPage(p); setPerPage(pp); }} />
      )}

      {modal && (
        <Modal title={editId ? "Modifier le client" : "Nouveau client"} onClose={() => setModal(false)} wide>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <Field label="Nom *"><Input placeholder="Ex. Dupont" value={form.nom} onChange={(e) => set("nom", e.target.value)} /></Field>
            <Field label="Prénom"><Input placeholder="Ex. Marie" value={form.prenom} onChange={(e) => set("prenom", e.target.value)} /></Field>
            <Field label="Entreprise"><Input placeholder="Raison sociale" value={form.entreprise} onChange={(e) => set("entreprise", e.target.value)} /></Field>
            <Field label="Email"><Input type="email" placeholder="contact@exemple.com" value={form.email} onChange={(e) => set("email", e.target.value)} /></Field>
            <Field label="Téléphone"><Input placeholder="+225 ..." value={form.telephone} onChange={(e) => set("telephone", e.target.value)} /></Field>
            <Field label="Adresse"><Input placeholder="Rue, quartier" value={form.adresse} onChange={(e) => set("adresse", e.target.value)} /></Field>
            <Field label="Ville"><Input placeholder="Ex. Abidjan" value={form.ville} onChange={(e) => set("ville", e.target.value)} /></Field>
            <Field label="Code postal"><Input value={form.code_postal} onChange={(e) => set("code_postal", e.target.value)} /></Field>
            <Field label="SIRET / RCCM"><Input value={form.siret} onChange={(e) => set("siret", e.target.value)} /></Field>
            <Field label="TVA intracommunautaire"><Input value={form.tva_intra} onChange={(e) => set("tva_intra", e.target.value)} /></Field>
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
