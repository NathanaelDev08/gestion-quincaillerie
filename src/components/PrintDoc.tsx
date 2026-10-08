import { useEffect, useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import { Printer } from "lucide-react";
import { api } from "../services/api";
import { fmtMoney, fmtDate } from "../utils/format";
import { calculerTotaux, type LigneCalcul } from "../utils/caisse";

/** Taux de TVA des lignes du ticket (normalement unique et uniforme). */
function fmtTaux(lignes: any[]): string {
  const t = new Set(lignes.map((l) => Number(l?.taux_tva) || 0));
  if (t.size !== 1) return "";
  return [...t][0].toString().replace(".0", "");
}
import { Button, Modal } from "./ui";

export function useCompany() {
  const [c, setC] = useState<any>({});
  useEffect(() => {
    api.settingsGet("company").then((v) => { if (v) setC(v); }).catch(() => {});
  }, []);
  return c;
}

function initials(name: string) {
  return (name || "G").split(/\s+/).map((w) => w[0]).join("").slice(0, 2).toUpperCase();
}

function CompanyHeader({ c }: { c: any }) {
  return (
    <div className="flex items-start gap-3">
      {c.logo ? (
        <img src={c.logo} alt="Logo" className="h-14 w-14 object-contain border rounded" />
      ) : (
        <div className="h-14 w-14 rounded-lg bg-blue-800 text-white flex items-center justify-center font-bold text-xl shrink-0">
          {initials(c.company_name)}
        </div>
      )}
      <div>
        <div className="font-bold text-lg text-blue-900 leading-tight">{c.company_name || "Ma Société"}</div>
        {c.forme_juridique && <div className="text-xs text-slate-500">{c.forme_juridique}{c.capital ? ` au capital de ${fmtMoney(+c.capital)}` : ""}</div>}
        <div className="text-xs text-slate-600">{c.company_address || ""}</div>
        <div className="text-xs text-slate-600">{[c.company_phone, c.company_email].filter(Boolean).join("  •  ")}</div>
        {c.company_siret && <div className="text-xs text-slate-600">RCCM / SIRET : {c.company_siret}</div>}
      </div>
    </div>
  );
}

function TitleBlock({ kicker, title, numero }: { kicker: string; title: string; numero: string }) {
  return (
    <div className="text-right shrink-0">
      <div className="text-[11px] uppercase tracking-widest text-slate-400">{kicker}</div>
      <div className="inline-block mt-1 px-4 py-2 rounded-lg bg-blue-800 text-white">
        <span className="font-bold text-xl">{title}</span>
        <span className="ml-2 text-sm opacity-90">{numero}</span>
      </div>
    </div>
  );
}

function TiersBlock({ label, nom, extra }: { label: string; nom?: string; extra?: string }) {
  return (
    <div className="border border-slate-300 rounded-lg p-2.5 bg-slate-50">
      <div className="text-[11px] uppercase tracking-wide text-slate-500">{label}</div>
      <div className="font-semibold">{nom || "-"}</div>
      {extra && <div className="text-xs text-slate-600">{extra}</div>}
    </div>
  );
}

function LinesTable({ lignes, withRecue }: { lignes: any[]; withRecue?: boolean }) {
  return (
    <table className="w-full text-sm border-collapse mt-1">
      <thead>
        <tr className="bg-blue-800 text-white">
          <th className="px-2 py-1.5 text-left font-semibold">Désignation</th>
          <th className="px-2 py-1.5 text-right font-semibold">{withRecue ? "Qté cmdée" : "Qté"}</th>
          {withRecue && <th className="px-2 py-1.5 text-right font-semibold">Qté reçue</th>}
          <th className="px-2 py-1.5 text-right font-semibold">PU HT</th>
          <th className="px-2 py-1.5 text-right font-semibold">TVA %</th>
          <th className="px-2 py-1.5 text-right font-semibold">Total HT</th>
        </tr>
      </thead>
      <tbody>
        {lignes.map((l, i) => (
          <tr key={i} className={i % 2 ? "bg-slate-50" : ""}>
            <td className="border border-slate-200 px-2 py-1">{l.designation}</td>
            <td className="border border-slate-200 px-2 py-1 text-right">{l.quantite}</td>
            {withRecue && <td className="border border-slate-200 px-2 py-1 text-right">{l.quantite_recue ?? "-"}</td>}
            <td className="border border-slate-200 px-2 py-1 text-right">{fmtMoney(l.prix_unitaire_ht)}</td>
            <td className="border border-slate-200 px-2 py-1 text-right">{l.taux_tva ?? 0}</td>
            <td className="border border-slate-200 px-2 py-1 text-right font-medium">{fmtMoney(l.total_ht ?? l.quantite * l.prix_unitaire_ht)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function TotalsBlock({ ht, tva, ttc, paye }: { ht: number; tva: number; ttc: number; paye?: number }) {
  return (
    <div className="flex justify-end mt-2">
      <table className="text-sm min-w-[240px]">
        <tbody>
          <tr><td className="px-3 py-0.5 text-slate-600">Total HT</td><td className="px-3 py-0.5 text-right font-medium">{fmtMoney(ht)}</td></tr>
          <tr><td className="px-3 py-0.5 text-slate-600">TVA</td><td className="px-3 py-0.5 text-right font-medium">{fmtMoney(tva)}</td></tr>
          <tr><td className="px-3 py-1 font-bold bg-blue-50 rounded">Total TTC</td><td className="px-3 py-1 text-right font-bold bg-blue-50 rounded">{fmtMoney(ttc)}</td></tr>
          {paye !== undefined && (<>
            <tr><td className="px-3 py-0.5 text-slate-600">Montant réglé</td><td className="px-3 py-0.5 text-right">{fmtMoney(paye)}</td></tr>
            <tr><td className="px-3 py-0.5 font-semibold">Reste dû</td><td className="px-3 py-0.5 text-right font-semibold">{fmtMoney(ttc - paye)}</td></tr>
          </>)}
        </tbody>
      </table>
    </div>
  );
}

const DEFAULT_CONDITIONS =
  "Paiement à réception de facture. Tout retard de paiement entraîne des pénalités calculées au taux d'intérêt légal en vigueur, ainsi qu'une indemnité forfaitaire de recouvrement de 25 000 F CFA. Pas d'escompte pour paiement anticipé.";

function DocFooter({ c, signatures }: { c: any; signatures?: string[] }) {
  return (
    <div className="mt-4">
      <div className="border-t-2 border-blue-800 pt-2">
        <div className="text-[11px] font-semibold text-slate-700 mb-0.5">Conditions de règlement</div>
        <p className="text-[10px] text-slate-500 leading-snug">{c.conditions_paiement || DEFAULT_CONDITIONS}</p>
        {c.iban && <p className="text-[11px] text-slate-600 mt-1"><b>Domiciliation bancaire :</b> {c.iban}</p>}
      </div>
      {signatures && (
        <div className="grid grid-cols-2 gap-10 mt-6 text-xs text-slate-500">
          {signatures.map((s) => (
            <div key={s} className="border-t border-slate-400 pt-1">{s}</div>
          ))}
        </div>
      )}
      <p className="text-[10px] text-slate-400 mt-3 text-center border-t border-slate-200 pt-1">
        {[c.company_name, c.forme_juridique && c.capital ? `${c.forme_juridique} au capital de ${fmtMoney(+c.capital)}` : c.forme_juridique, c.company_siret ? `RCCM : ${c.company_siret}` : "", c.company_address, [c.company_phone, c.company_email].filter(Boolean).join(" • ")]
          .filter(Boolean).join(" — ")}
        {"  •  "}Document généré le {fmtDate(new Date().toISOString().slice(0, 10))} — Montants en Franc CFA (XOF)
      </p>
    </div>
  );
}

function DocShell({ kicker, title, numero, children, notes, signatures }: {
  kicker: string; title: string; numero: string;
  children: React.ReactNode; notes?: string; signatures?: string[];
}) {
  const c = useCompany();
  return (
    <div className="bg-white text-slate-900 p-6 text-sm" style={{ fontFamily: "Arial, Helvetica, sans-serif" }}>
      <div className="flex justify-between items-start gap-4 pb-3 border-b border-slate-200">
        <CompanyHeader c={c} />
        <TitleBlock kicker={kicker} title={title} numero={numero} />
      </div>
      <div className="mt-3">{children}</div>
      {notes && <p className="text-xs text-slate-600 mt-3 whitespace-pre-line border-l-2 border-blue-200 pl-2">{notes}</p>}
      <DocFooter c={c} signatures={signatures} />
    </div>
  );
}

export function FactureDoc({ facture, lignes }: { facture: any; lignes: any[] }) {
  return (
    <DocShell kicker="Facture de vente" title="FACTURE" numero={facture.numero} notes={facture.notes} signatures={["Signature et cachet du vendeur"]}>
      <div className="grid grid-cols-2 gap-3 mb-1">
        <TiersBlock label="Facturé à" nom={facture.client_nom} />
        <div className="text-sm space-y-0.5 border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Émise le : </span><b>{fmtDate(facture.date_emission)}</b></div>
          <div><span className="text-slate-500">Échéance : </span><b>{fmtDate(facture.date_echeance)}</b></div>
          {facture.devis_id && <div className="text-xs text-slate-500">Issue d'un devis accepté</div>}
        </div>
      </div>
      <LinesTable lignes={lignes} />
      <TotalsBlock ht={facture.total_ht} tva={facture.total_tva} ttc={facture.total_ttc} paye={facture.montant_paye} />
    </DocShell>
  );
}

export function DevisDoc({ devis, lignes }: { devis: any; lignes: any[] }) {
  return (
    <DocShell kicker="Proposition commerciale" title="DEVIS" numero={devis.numero} notes={devis.notes || devis.conditions} signatures={["Signature du vendeur", "Bon pour accord — signature du client"]}>
      <div className="grid grid-cols-2 gap-3 mb-1">
        <TiersBlock label="Établi pour" nom={devis.client_nom} />
        <div className="text-sm space-y-0.5 border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Émis le : </span><b>{fmtDate(devis.date_emission)}</b></div>
          <div><span className="text-slate-500">Valable jusqu'au : </span><b>{fmtDate(devis.date_validite)}</b></div>
        </div>
      </div>
      <LinesTable lignes={lignes} />
      <TotalsBlock ht={devis.total_ht} tva={devis.total_tva} ttc={devis.total_ttc} />
    </DocShell>
  );
}

export function BonCommandeDoc({ commande, lignes, fournisseurNom }: { commande: any; lignes: any[]; fournisseurNom?: string }) {
  const ht = commande.total_ht;
  return (
    <DocShell kicker="Achat fournisseur" title="BON DE COMMANDE" numero={commande.numero} notes={commande.notes} signatures={["Signature et cachet de l'acheteur"]}>
      <div className="grid grid-cols-2 gap-3 mb-1">
        <TiersBlock label="Fournisseur" nom={fournisseurNom || commande.fournisseur_nom} />
        <div className="text-sm space-y-0.5 border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Commandé le : </span><b>{fmtDate(commande.date_commande)}</b></div>
          <div><span className="text-slate-500">Livraison prévue : </span><b>{fmtDate(commande.date_livraison_prevue)}</b></div>
        </div>
      </div>
      <LinesTable lignes={lignes} withRecue />
      <TotalsBlock ht={ht} tva={commande.total_ttc - ht} ttc={commande.total_ttc} />
    </DocShell>
  );
}

export function RecuDoc({ reglement, factureNumero, clientNom }: { reglement: any; factureNumero: string; clientNom?: string }) {
  return (
    <DocShell kicker="Reçu de paiement" title="REÇU" numero={reglement.reference || reglement.id.slice(0, 8).toUpperCase()} signatures={["Signature du caissier", "Signature du client"]}>
      <div className="grid grid-cols-2 gap-3 mb-3">
        <TiersBlock label="Reçu de" nom={clientNom} />
        <div className="text-sm space-y-0.5 border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Facture : </span><b>{factureNumero}</b></div>
          <div><span className="text-slate-500">Date : </span><b>{fmtDate(reglement.date_reglement)}</b></div>
          <div><span className="text-slate-500">Mode : </span><b>{reglement.mode}</b></div>
        </div>
      </div>
      <div className="border-2 border-blue-800 rounded-lg p-4 text-center bg-blue-50">
        <div className="text-xs uppercase tracking-widest text-slate-500">Montant reçu</div>
        <div className="font-bold text-3xl text-blue-900">{fmtMoney(reglement.montant)}</div>
      </div>
      {reglement.notes && <p className="text-xs mt-2">{reglement.notes}</p>}
    </DocShell>
  );
}

export function BLDoc({ bl, lignes }: { bl: any; lignes: any[] }) {
  const ht = lignes.reduce((s: number, l: any) => s + l.quantite * l.prix_unitaire_ht, 0);
  const tva = lignes.reduce((s: number, l: any) => s + l.quantite * l.prix_unitaire_ht * (l.taux_tva || 0) / 100, 0);
  return (
    <DocShell kicker="Livraison client" title="BON DE LIVRAISON" numero={bl.numero} notes={bl.notes} signatures={["Signature du livreur", "Bon pour réception — signature du client"]}>
      <div className="grid grid-cols-2 gap-3 mb-1">
        <TiersBlock label="Livré à" nom={bl.client_nom} />
        <div className="text-sm space-y-0.5 border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Date de livraison : </span><b>{fmtDate(bl.date_livraison)}</b></div>
          <div><span className="text-slate-500">Statut : </span><b>{bl.statut}</b></div>
        </div>
      </div>
      <LinesTable lignes={lignes} />
      <TotalsBlock ht={ht} tva={tva} ttc={ht + tva} />
    </DocShell>
  );
}

export function BulletinDoc({ bulletin }: { bulletin: any }) {
  return (
    <DocShell kicker="Paie" title="BULLETIN DE PAIE" numero={bulletin.numero} signatures={["Signature de l'employeur", "Signature de l'employé"]}>
      <div className="grid grid-cols-2 gap-3 mb-3">
        <TiersBlock label="Employé" nom={bulletin.employe_nom} extra={`Période : ${bulletin.periode}`} />
        <div className="text-sm border border-slate-300 rounded-lg p-2.5">
          <div><span className="text-slate-500">Statut : </span><b>{bulletin.statut}</b></div>
        </div>
      </div>
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr className="bg-blue-800 text-white">
            <th className="px-2 py-1.5 text-left">Rubrique</th>
            <th className="px-2 py-1.5 text-right">Base</th>
            <th className="px-2 py-1.5 text-right">Montant</th>
          </tr>
        </thead>
        <tbody>
          <tr><td className="border px-2 py-1">Salaire brut</td><td className="border px-2 py-1 text-right">-</td><td className="border px-2 py-1 text-right">{fmtMoney(bulletin.brut)}</td></tr>
          <tr><td className="border px-2 py-1">CNPS salariale (6,3 %)</td><td className="border px-2 py-1 text-right">{fmtMoney(bulletin.brut)}</td><td className="border px-2 py-1 text-right">− {fmtMoney(bulletin.cnps)}</td></tr>
          <tr><td className="border px-2 py-1">ITS (10 %)</td><td className="border px-2 py-1 text-right">{fmtMoney(bulletin.brut - bulletin.cnps)}</td><td className="border px-2 py-1 text-right">− {fmtMoney(bulletin.its)}</td></tr>
          <tr className="bg-blue-50"><td className="border px-2 py-1 font-bold">Net à payer</td><td className="border px-2 py-1"></td><td className="border px-2 py-1 text-right font-bold">{fmtMoney(bulletin.net)}</td></tr>
        </tbody>
      </table>
    </DocShell>
  );
}

/**
 * Totaux d'un ticket de caisse.
 *
 * Doivent venir de la MÊME logique que la caisse et le backend, remise
 * comprise. Sans cela, un ticket avec code promo affichait un « Total HT »
 * qui ne correspondait pas au « TOTAL TTC » : le document ne s'additionnait
 * pas, ce que le client comme le contrôleur remarquent immédiatement.
 */
export function totauxTicket(
  lignes: Pick<LigneCalcul, "quantite" | "prix_unitaire_ht" | "taux_tva" | "remise">[],
  remiseGlobalePct = 0,
) {
  const t = calculerTotaux(
    lignes.map((l) => ({ ...l, designation: "", produit_id: null })),
    remiseGlobalePct,
  );
  return { ht: t.ht, tva: t.tva, ttc: t.ttc };
}

export function TicketCaisseDoc({ numero, lignes, total, recu, rendu, mode, clientNom, caissier, dateTime, remiseGlobalePct = 0 }: {
  numero: string; lignes: any[]; total: number; recu: number; rendu: number; mode: string; clientNom?: string;
  caissier?: string; dateTime?: string; remiseGlobalePct?: number;
}) {
  const c: any = useCompany();
  const { ht, tva } = totauxTicket(lignes, remiseGlobalePct);
  const qrValue = `GC:${numero}|${Math.round(total)}|${new Date().toISOString().slice(0, 10)}`;
  return (
    <div className="bg-white text-slate-900 mx-auto" style={{ maxWidth: 300, fontFamily: "Arial, Helvetica, sans-serif" }}>
      {/* En-tête enseigne */}
      <div className="text-center pb-1">
        {c.logo && <img src={c.logo} alt="Logo" className="h-12 w-12 object-contain mx-auto mb-1" />}
        <div className="font-bold text-base leading-tight">{c.company_name || "Ma Société"}</div>
        <div className="text-[11px] text-slate-600">{c.company_address || ""}</div>
        <div className="text-[11px] text-slate-600">{[c.company_phone, c.company_email].filter(Boolean).join(" • ")}</div>
        {c.company_siret && <div className="text-[10px] text-slate-500">RCCM : {c.company_siret}</div>}
      </div>
      <div className="border-t-2 border-dashed border-slate-800 my-1" />
      {/* Références */}
      <div className="text-[11px] space-y-0.5">
        <div className="flex justify-between"><span className="text-slate-500">Ticket N°</span><b>{numero}</b></div>
        <div className="flex justify-between"><span className="text-slate-500">Date</span><span>{dateTime || fmtDate(new Date().toISOString().slice(0, 10))}</span></div>
        {caissier && <div className="flex justify-between"><span className="text-slate-500">Caissier</span><span>{caissier}</span></div>}
        {clientNom && <div className="flex justify-between"><span className="text-slate-500">Client</span><span>{clientNom}</span></div>}
      </div>
      <div className="border-t border-dashed border-slate-400 my-1" />
      {/* Articles */}
      <table className="w-full text-[11px]">
        <thead>
          <tr className="text-slate-500">
            <th className="text-left font-semibold">Article</th>
            <th className="text-right font-semibold">Qté</th>
            <th className="text-right font-semibold">PU</th>
            <th className="text-right font-semibold">Total</th>
          </tr>
        </thead>
        <tbody>
          {lignes.map((l, i) => (
            <tr key={i}>
              <td className="pr-1">{String(l.designation ?? "").slice(0, 22)}</td>
              <td className="text-right whitespace-nowrap">{l.quantite}</td>
              <td className="text-right whitespace-nowrap">{fmtMoney(l.prix_unitaire_ht)}</td>
              <td className="text-right whitespace-nowrap font-medium">
                {fmtMoney(l.quantite * l.prix_unitaire_ht * (1 - (l.remise ?? 0) / 100) * (1 - remiseGlobalePct / 100))}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="border-t border-dashed border-slate-400 my-1" />
      {/* Totaux */}
      <div className="text-[11px] space-y-0.5">
        <div className="flex justify-between"><span>Total HT</span><span>{fmtMoney(ht)}</span></div>
        <div className="flex justify-between"><span>TVA ({fmtTaux(lignes)} %)</span><span>{fmtMoney(tva)}</span></div>
        <div className="flex justify-between font-bold text-sm border-y border-slate-800 py-0.5"><span>TOTAL TTC</span><span>{fmtMoney(total)}</span></div>
        <div className="flex justify-between mt-1"><span>Reçu ({mode})</span><span>{fmtMoney(recu)}</span></div>
        <div className="flex justify-between font-bold"><span>RENDU</span><span>{fmtMoney(rendu)}</span></div>
      </div>
      <div className="border-t border-dashed border-slate-400 my-1" />
      {/* QR de vérification */}
      <div className="text-center">
        <QRCodeSVG value={qrValue} size={76} className="mx-auto" />
        <div className="text-[11px] tracking-[0.2em] font-mono mt-0.5">{numero}</div>
      </div>
      <p className="text-[10px] text-slate-500 text-center mt-1 leading-snug">
        Marchandise ni reprise ni échangée sans ticket.<br />Merci de votre visite et à bientôt !
      </p>
    </div>
  );
}

/** Modale d'aperçu + impression (zone isolée pour window.print) */
export function PrintModal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <Modal title={title} onClose={onClose} wide>
      <div id="print-area">{children}</div>
      <div className="flex justify-end gap-2 mt-3">
        <Button className="bg-slate-500" onClick={onClose}>Fermer</Button>
        <Button onClick={() => window.print()}><Printer size={15} /> Imprimer</Button>
      </div>
    </Modal>
  );
}
