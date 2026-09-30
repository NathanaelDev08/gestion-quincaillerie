import { useEffect, useRef, useState } from "react";
import JsBarcode from "jsbarcode";
import { Printer, Tag } from "lucide-react";
import { Button, Input, Modal } from "./ui";
import { fmtMoney } from "../utils/format";

/** Planche d'étiquettes codes-barres imprimable (Code128) */
export function EtiquettesModal({ produits, onClose }: { produits: any[]; onClose: () => void }) {
  const [qtes, setQtes] = useState<Record<string, number>>({});
  const refs = useRef<Record<string, SVGSVGElement | null>>({});

  const selection = produits.filter((p) => (qtes[p.id] || 0) > 0);
  const totalEtiquettes = selection.reduce((s, p) => s + (qtes[p.id] || 0), 0);

  useEffect(() => {
    selection.forEach((p) => {
      const el = refs.current[p.id];
      const code = p.code_barre || p.reference;
      if (el && code) {
        try {
          JsBarcode(el, code, { format: code.length === 13 && /^\d+$/.test(code) ? "EAN13" : "CODE128", width: 1.6, height: 36, displayValue: true, fontSize: 11, margin: 2 });
        } catch { /* code invalide : ignoré */ }
      }
    });
  });

  return (
    <Modal title={`Étiquettes codes-barres (${totalEtiquettes})`} onClose={onClose} wide>
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 mb-3 max-h-[30vh] overflow-auto">
        {produits.map((p) => (
          <label key={p.id} className="flex items-center gap-2 border rounded-lg px-2 py-1.5 text-sm cursor-pointer hover:bg-slate-50">
            <input type="number" min={0} className="w-14 px-1 py-0.5 border rounded text-sm"
              value={qtes[p.id] ?? ""} placeholder="0"
              onChange={(e) => setQtes({ ...qtes, [p.id]: +e.target.value })} />
            <span className="truncate"><b>{p.designation}</b> <span className="text-slate-400 text-xs">{p.code_barre || p.reference}</span></span>
          </label>
        ))}
      </div>
      <div id="print-area">
        {selection.length === 0 && <p className="text-sm text-slate-400 text-center">Saisissez des quantités ci-dessus.</p>}
        <div className="grid grid-cols-3 gap-2">
          {selection.flatMap((p) =>
            Array.from({ length: qtes[p.id] || 0 }, (_, i) => (
              <div key={`${p.id}-${i}`} className="border border-slate-300 rounded p-1.5 text-center bg-white">
                <div className="text-[11px] font-semibold truncate">{p.designation}</div>
                <svg ref={(el) => { refs.current[p.id] = el; }} className="mx-auto" />
                <div className="font-bold text-sm">{fmtMoney(p.prix_vente_ht)}</div>
              </div>
            ))
          )}
        </div>
      </div>
      <div className="flex justify-end gap-2 mt-3">
        <Button className="bg-slate-500" onClick={onClose}>Fermer</Button>
        <Button onClick={() => window.print()} disabled={totalEtiquettes === 0}><Printer size={15} /> Imprimer les étiquettes</Button>
      </div>
    </Modal>
  );
}

/** Petit badge code-barres (texte seul si JsBarcode indisponible) */
export function CodeBarre({ value }: { value?: string }) {
  if (!value) return <span className="text-slate-300">—</span>;
  return (
    <span className="inline-flex items-center gap-1 font-mono text-xs">
      <Tag size={12} className="text-slate-400" /> {value}
    </span>
  );
}
