import { useState } from "react";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { KeyRound, ShieldCheck, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { useAuth } from "../stores/useAuth";
import { Button, Card, PageHeader, IconBtn, Badge, confirmDelete, DataView, Pagination, paginate, Modal, Field, Input, toast } from "../components/ui";

const ROLES = [
  { code: "admin", label: "Administrateur" },
  { code: "user", label: "Utilisateur" },
  { code: "comptable", label: "Comptable" },
  { code: "commercial", label: "Commercial" },
];

export default function Utilisateurs() {
  const qc = useQueryClient();
  const { user } = useAuth();
  const [page, setPage] = useState(1);
  const perPage = 10;
  const [showMdp, setShowMdp] = useState(false);
  const [mdp, setMdp] = useState({ ancien: "", nouveau: "", confirme: "" });
  const [nouveauSecret, setNouveauSecret] = useState("");
  const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.usersList });

  const changerMdp = useMutation({
    mutationFn: () => api.usersChangerMotDePasse(mdp.ancien, mdp.nouveau),
    onSuccess: () => {
      setShowMdp(false);
      setMdp({ ancien: "", nouveau: "", confirme: "" });
      toast.success("Mot de passe modifié");
    },
    onError: (e: any) => toast.error(String(e)),
  });
  const reinitialiser = useMutation({
    mutationFn: (id: string) => api.usersReinitialiserMotDePasse(id),
    onSuccess: (secret) => {
      setNouveauSecret(secret);
      qc.invalidateQueries({ queryKey: ["users"] });
    },
    onError: (e: any) => toast.error(String(e)),
  });

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
            <IconBtn icon={KeyRound} title="Réinitialiser le mot de passe" tone="amber"
              onClick={() => { if (window.confirm(`Réinitialiser le mot de passe de ${u.username} ?`)) reinitialiser.mutate(u.id); }} />
            {u.id !== user?.id && (
              <IconBtn icon={Trash2} title="Supprimer" tone="red" onClick={() => { if (confirmDelete(u.username)) del.mutate(u); }} />
            )}
          </>)}
        />
        <Pagination page={page} perPage={perPage} total={(users || []).length}
          onChange={(p) => setPage(p)} />
        <div className="flex items-center justify-between mt-3 pt-3 border-t flex-wrap gap-2">
          <p className="text-xs text-slate-500">Les nouveaux comptes se créent depuis l'écran de connexion (« Créer un compte »). Le dernier compte ne peut pas être supprimé.</p>
          <Button className="bg-blue-700" onClick={() => setShowMdp(true)}>
            <KeyRound size={15} /> Changer mon mot de passe
          </Button>
        </div>
      </Card>

      {showMdp && (
        <Modal title="Changer mon mot de passe" onClose={() => setShowMdp(false)}>
          <div className="space-y-3">
            <Field label="Mot de passe actuel">
              <Input type="password" value={mdp.ancien} onChange={(e) => setMdp({ ...mdp, ancien: e.target.value })} autoComplete="current-password" />
            </Field>
            <Field label="Nouveau mot de passe">
              <Input type="password" value={mdp.nouveau} onChange={(e) => setMdp({ ...mdp, nouveau: e.target.value })} autoComplete="new-password" />
              <p className="text-[11px] text-slate-400 mt-1">8 caractères minimum, avec une majuscule et un chiffre.</p>
            </Field>
            <Field label="Confirmer le nouveau mot de passe">
              <Input type="password" value={mdp.confirme} onChange={(e) => setMdp({ ...mdp, confirme: e.target.value })} autoComplete="new-password" />
            </Field>
            {mdp.confirme && mdp.nouveau !== mdp.confirme && (
              <p className="text-xs text-red-600">Les deux mots de passe ne correspondent pas.</p>
            )}
            <div className="flex gap-2 justify-end">
              <Button className="bg-slate-500" onClick={() => setShowMdp(false)}>Annuler</Button>
              <Button onClick={() => changerMdp.mutate()} disabled={!mdp.ancien || !mdp.nouveau || mdp.nouveau !== mdp.confirme}>
                Modifier
              </Button>
            </div>
          </div>
        </Modal>
      )}

      {nouveauSecret && (
        <Modal title="Nouveau mot de passe à communiquer" onClose={() => setNouveauSecret("")}>
          <p className="text-sm text-slate-600 mb-2">
            Communique ce mot de passe à l'utilisateur. Il s'affiche <b>une seule fois</b> et n'est stocké nulle part en clair.
          </p>
          <div className="px-3 py-2 rounded-lg bg-slate-100 border font-mono text-sm tracking-wide select-all mb-3">
            {nouveauSecret}
          </div>
          <div className="flex gap-2 justify-end">
            <Button className="bg-slate-500" onClick={() => setNouveauSecret("")}>Fermer</Button>
          </div>
        </Modal>
      )}
    </div>
  );
}
