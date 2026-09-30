import { useState } from "react";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { ShieldCheck, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { Card, PageHeader, IconBtn, Badge, confirmDelete, DataView, Pagination, paginate, toast } from "../components/ui";

const ROLES = [
  { code: "admin", label: "Administrateur" },
  { code: "user", label: "Utilisateur" },
  { code: "comptable", label: "Comptable" },
  { code: "commercial", label: "Commercial" },
];

export default function Utilisateurs() {
  const qc = useQueryClient();
  const [page, setPage] = useState(1);
  const perPage = 10;
  const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.usersList });

  const role = useMutation({
    mutationFn: ({ id, r }: { id: string; r: string }) => api.usersChangerRole(id, r),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["users"] }),
  });
  const del = useMutation({
    mutationFn: (u: any) => api.usersDelete(u.id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["users"] }),
    onError: (e: any) => toast.error(String(e)),
  });

  return (
    <div>
      <PageHeader title="Utilisateurs & rôles" subtitle={`${users?.length || 0} comptes`} />
      <Card>
        <DataView
          data={paginate(users || [], page, perPage)}
          empty="Aucun utilisateur."
          title={(u) => u.username}
          subtitle={(u) => u.full_name}
          fields={[
            { label: "Email", value: (u) => u.email || "-" },
            {
              label: "Rôle", value: (u) => (
                <select className="text-xs border rounded px-1 py-0.5" value={u.role} title="Rôle"
                  onChange={(e) => role.mutate({ id: u.id, r: e.target.value })}>
                  {ROLES.map((r) => <option key={r.code} value={r.code}>{r.label}</option>)}
                </select>
              ),
            },
          ]}
          actions={(u) => (<>
            <Badge className="bg-slate-100 text-slate-600 mr-1"><ShieldCheck size={12} /> {u.role}</Badge>
            <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(u.username)) del.mutate(u); }} />
          </>)}
        />
        <Pagination page={page} perPage={perPage} total={(users || []).length}
          onChange={(p) => setPage(p)} />
        <p className="text-xs text-slate-500 mt-2">Les nouveaux comptes se créent depuis l'écran de connexion (« Créer un compte »). Le dernier compte ne peut pas être supprimé.</p>
      </Card>
    </div>
  );
}
