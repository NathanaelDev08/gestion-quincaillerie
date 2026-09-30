import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { AlertTriangle, BellRing, Info, Package } from "lucide-react";
import { api } from "../services/api";
import { Modal } from "./ui";

const ICONS: Record<string, any> = {
  urgent: BellRing,
  alerte: AlertTriangle,
  info: Info,
};
const TONES: Record<string, string> = {
  urgent: "border-red-300 bg-red-50",
  alerte: "border-amber-300 bg-amber-50",
  info: "border-blue-200 bg-blue-50",
};

export default function NotificationsCenter({ open, onClose }: { open: boolean; onClose: () => void }) {
  const nav = useNavigate();
  const { data } = useQuery({ queryKey: ["notifications"], queryFn: api.notificationsList, enabled: open });

  if (!open) return null;
  return (
    <Modal title="Centre de notifications" onClose={onClose}>
      {(data || []).length === 0 && (
        <p className="text-sm text-green-700 text-center py-4">Tout est à jour, rien à signaler.</p>
      )}
      <div className="space-y-2">
        {(data || []).map((n, i) => {
          const Icon = ICONS[n.niveau] ?? Package;
          return (
            <button key={i} className={`w-full text-left border rounded-xl p-3 hover:shadow ${TONES[n.niveau] ?? "border-slate-200"}`}
              onClick={() => { nav(n.lien); onClose(); }}>
              <div className="flex items-center gap-2">
                <Icon size={16} />
                <span className="text-[11px] uppercase text-slate-500">{n.categorie}</span>
              </div>
              <div className="font-semibold text-sm mt-0.5">{n.titre}</div>
              <div className="text-xs text-slate-600">{n.detail}</div>
            </button>
          );
        })}
      </div>
    </Modal>
  );
}
