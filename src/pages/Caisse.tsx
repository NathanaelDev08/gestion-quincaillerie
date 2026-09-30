import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Banknote, Check, Delete, History, Minus, Plus, Printer, ShoppingBag, Star, Trash2, X } from "lucide-react";
import { api } from "../services/api";
import { useAuth } from "../stores/useAuth";
import { Button, Input, Card, PageHeader, Select, DataView, Tabs, toast } from "../components/ui";
import { TicketCaisseDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, labelOf, MODES_REGLEMENT, todayISO } from "../utils/format";

type CartLine = { produit_id: string; designation: string; quantite: number; prix_unitaire_ht: number; taux_tva: number; stock: number };

const MODES_CAISSE = ["especes", "mobile", "cb", "virement"];
const BILLETS = [500, 1000, 2000, 5000, 10000];

export default function Caisse() {
  const qc = useQueryClient();
  const { user } = useAuth();
  const [tab, setTab] = useState<"vente" | "clotures">("vente");
  const [q, setQ] = useState("");
  const [cat, setCat] = useState("");
  const [cart, setCart] = useState<CartLine[]>([]);
  const [clientId, setClientId] = useState("");
  const [mode, setMode] = useState("especes");
  const [recuTxt, setRecuTxt] = useState("");
  const [promoCode, setPromoCode] = useState("");
  const [promoPct, setPromoPct] = useState(0);
  const [promoLabel, setPromoLabel] = useState("");
  const [ptsDebites, setPtsDebites] = useState(0);
  const [ticket, setTicket] = useState<any | null>(null);
  const [msg, setMsg] = useState("");
  const [cloture, setCloture] = useState<any | null>(null);

  const { data: produits } = useQuery({ queryKey: ["produits-all"], queryFn: () => api.produitsList(1, 500) });
  const { data: cats } = useQuery({ queryKey: ["categories"], queryFn: api.produitsCategories });
  const { data: clients } = useQuery({ queryKey: ["clients-all"], queryFn: () => api.clientsList(1, 200) });
  const { data: top } = useQuery({ queryKey: ["topP"], queryFn: api.topProduits });
  const { data: factures } = useQuery({ queryKey: ["factures-all"], queryFn: () => api.facturesList(1, 200) });
  const { data: pointsCli } = useQuery({
    queryKey: ["points", clientId],
    queryFn: () => api.fideliteSolde(clientId),
    enabled: !!clientId,
  });
  const clientPts = pointsCli || 0;
  const { data: clotures } = useQuery({ queryKey: ["clotures"], queryFn: api.cloturesList, enabled: tab === "clotures" });

  const catalogue = useMemo(() => (produits?.data || []).filter((p) =>
    (!q || (p.designation + " " + p.reference + " " + (p.code_barre || "")).toLowerCase().includes(q.toLowerCase())) &&
    (!cat || p.categorie === cat)
  ), [produits, q, cat]);

  const favoris = useMemo(() => {
    const names = new Set((top || []).slice(0, 6).map((t) => t.nom));
    return (produits?.data || []).filter((p) => names.has(p.designation)).slice(0, 6);
  }, [produits, top]);

  const ventesJour = useMemo(() => {
    const t = todayISO();
    const list = (factures?.data || []).filter((f: any) => f.date_emission === t);
    return { nb: list.length, total: list.reduce((s: number, f: any) => s + f.total_ttc, 0) };
  }, [factures]);

  const total = cart.reduce((s, l) => s + l.quantite * l.prix_unitaire_ht * (1 - promoPct / 100), 0);
  const recu = +recuTxt || 0;
  const rendu = recu - total;

  const appliquerPromo = async () => {
    setMsg("");
    try {
      const r = await api.promosValider(promoCode, total);
      const pct = r.type === "pourcent" ? r.valeur : (r.remise / Math.max(1, total)) * 100;
      setPromoPct(Math.min(100, pct));
      setPromoLabel(`${r.code} (−${fmtMoney(r.remise)})`);
      toast.success(`Code ${r.code} appliqué : −${fmtMoney(r.remise)}`);
    } catch (e: any) { setMsg(String(e)); }
  };

  const utiliserPoints = async () => {
    if (!clientId || clientPts <= 0) return;
    // 1 pt = 10 F, plafonné à 50 % du ticket ; converti en % de remise par ligne
    const valeur = Math.min(clientPts * 10, total * 0.5);
    const pts = Math.floor(valeur / 10);
    if (pts <= 0) return;
    try {
      await api.fideliteUtiliser(clientId, pts);
      const pct = Math.min(100, (pts * 10 / Math.max(1, total)) * 100);
      setPromoPct(pct);
      setPtsDebites(pts);
      setPromoLabel(`Fidélité (−${fmtMoney(pts * 10)}, ${pts} pts)`);
      qc.invalidateQueries({ queryKey: ["points", clientId] });
      toast.success(`${pts} points convertis : −${fmtMoney(pts * 10)}`);
    } catch (e: any) { setMsg(String(e)); }
  };

  const add = (p: any, qty = 1) => {
    setMsg("");
    const ex = cart.find((l) => l.produit_id === p.id);
    const cur = ex ? ex.quantite : 0;
    if (cur + qty > p.stock && !window.confirm(`Stock insuffisant pour ${p.designation} (reste ${p.stock}). Ajouter quand même ?`)) {
      return;
    }
    if (ex) setCart(cart.map((l) => l.produit_id === p.id ? { ...l, quantite: l.quantite + qty } : l));
    else setCart([...cart, { produit_id: p.id, designation: p.designation, quantite: qty, prix_unitaire_ht: p.prix_vente_ht, taux_tva: p.taux_tva, stock: p.stock }]);
  };

  /** Douchette / code-barres : Entrée ajoute le premier résultat */
  const onSearchKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && catalogue.length > 0) {
      add(catalogue[0]);
      setQ("");
    }
  };

  const chqte = (id: string, d: number) => {
    setCart(cart.map((l) => {
      if (l.produit_id !== id) return l;
      const nq = Math.min(l.stock, Math.max(0, l.quantite + d));
      return { ...l, quantite: nq };
    }).filter((l) => l.quantite > 0));
  };

  const refreshAll = () => {
    qc.invalidateQueries({ queryKey: ["produits-all"] });
    qc.invalidateQueries({ queryKey: ["factures-all"] });
    qc.invalidateQueries({ queryKey: ["factures"] });
    qc.invalidateQueries({ queryKey: ["stock"] });
  };

  const valider = async () => {
    setMsg("");
    const lignes = cart.map((l) => ({ ...l, remise: Math.round(promoPct * 100) / 100 }));
    const afterSuccess = () => {
      setCart([]); setRecuTxt(""); setPromoPct(0); setPromoLabel(""); setPromoCode(""); setPtsDebites(0);
    };
    try {
      const r = await api.venteComptoir({
        client_id: clientId || null,
        lignes,
        mode, montant_recu: recu || total,
      });
      const cli = (clients?.data || []).find((c) => c.id === clientId);
      const now = new Date();
      const dateTime = now.toLocaleDateString("fr-FR") + " " + now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
      setTicket({ ...r, lignes: cart, modeLabel: labelOf(MODES_REGLEMENT, mode), clientNom: cli ? cli.nom : "Client comptoir", caissier: user?.full_name || user?.username, dateTime });
      afterSuccess();
      toast.success(`Vente ${r.numero} encaissée — rendu ${fmtMoney(r.rendu)}`);
      refreshAll();
    } catch (e: any) {
      // Rollback : recrédite les points débités si la vente échoue
      if (ptsDebites > 0 && clientId) {
        try { await api.fideliteUtiliser(clientId, -ptsDebites); } catch { /* ignore */ }
        setPtsDebites(0);
        setPromoPct(0);
        setPromoLabel("");
      }
      setMsg(String(e));
    }
  };

  const faireCloture = async () => {
    try {
      setCloture(await api.clotureZ(todayISO()));
    } catch (e: any) { setMsg(String(e)); }
  };

  const numpad = (k: string) => {
    if (k === "C") setRecuTxt("");
    else if (k === "⌫") setRecuTxt((s) => s.slice(0, -1));
    else setRecuTxt((s) => (s + k).slice(0, 9));
  };

  return (
    <div className="select-none">
      <PageHeader title="Caisse tactile" subtitle={`Ventes du jour : ${ventesJour.nb} tickets • ${fmtMoney(ventesJour.total)}`} />
      {msg && <p className="text-sm text-red-600 bg-red-50 border border-red-200 rounded-lg px-3 py-2 mb-2">{msg}</p>}
      <Tabs<"vente" | "clotures">
        active={tab} onChange={setTab}
        tabs={[
          { key: "vente", label: "Encaissement", icon: ShoppingBag },
          { key: "clotures", label: "Clôtures Z", icon: History },
        ]}
      />
      {tab === "vente" && (
      <div className="grid grid-cols-1 xl:grid-cols-5 gap-3">
        {/* Catalogue tactile */}
        <Card className="xl:col-span-3">
          <div className="flex gap-2 mb-2">
            <Input placeholder="Rechercher ou scanner un code-barres… (Entrée = ajouter)" autoFocus
              value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={onSearchKey} className="text-base py-2.5" />
            <Select value={cat} onChange={(e) => setCat(e.target.value)} title="Catégorie" className="max-w-[140px]">
              <option value="">Toutes</option>
              {(cats || []).map((c) => <option key={c} value={c}>{c}</option>)}
            </Select>
          </div>
          {favoris.length > 0 && (
            <div className="mb-2">
              <div className="text-[11px] uppercase text-slate-400 mb-1 flex items-center gap-1"><Star size={11} /> Ventes fréquentes</div>
              <div className="flex gap-2 overflow-auto pb-1">
                {favoris.map((p: any) => (
                  <button key={p.id} onClick={() => add(p)} disabled={p.stock <= 0}
                    className="shrink-0 border border-amber-300 bg-amber-50 rounded-xl px-3 py-2 text-left touch-manipulation active:scale-95 disabled:opacity-40">
                    <div className="font-medium text-sm whitespace-nowrap">{p.designation}</div>
                    <div className="text-blue-800 font-bold text-sm">{fmtMoney(p.prix_vente_ht)}</div>
                  </button>
                ))}
              </div>
            </div>
          )}
          <div className="grid grid-cols-2 sm:grid-cols-3 xl:grid-cols-4 gap-2 max-h-[52vh] overflow-auto">
            {catalogue.map((p: any) => (
              <button key={p.id} onClick={() => add(p)} disabled={p.stock <= 0}
                className="border rounded-xl p-3 text-left touch-manipulation active:scale-95 hover:border-blue-400 hover:bg-blue-50 transition disabled:opacity-40 min-h-[88px] flex flex-col justify-between">
                <div className="font-medium truncate">{p.designation}</div>
                <div className="text-xs text-slate-500">{p.reference} • Stock : <b className={p.stock <= p.stock_alerte ? "text-red-600" : ""}>{p.stock}</b></div>
                <div className="font-bold text-blue-800 text-base">{fmtMoney(p.prix_vente_ht)}</div>
              </button>
            ))}
          </div>
        </Card>

        {/* Panier + encaissement */}
        <Card className="xl:col-span-2">
          <h3 className="font-semibold mb-1">Ticket en cours ({cart.reduce((s, l) => s + l.quantite, 0)} articles)</h3>
          <div className="space-y-1.5 max-h-[24vh] overflow-auto mb-2">
            {cart.length === 0 && <p className="text-sm text-slate-400">Touchez un article pour l'ajouter.</p>}
            {cart.map((l) => (
              <div key={l.produit_id} className="flex items-center gap-1.5 text-sm border rounded-xl px-2 py-1.5">
                <span className="flex-1 truncate font-medium">{l.designation}</span>
                <button className="p-2 hover:bg-slate-100 rounded-lg touch-manipulation" onClick={() => chqte(l.produit_id, -1)}><Minus size={15} /></button>
                <b className="w-7 text-center text-base">{l.quantite}</b>
                <button className="p-2 hover:bg-slate-100 rounded-lg touch-manipulation" onClick={() => chqte(l.produit_id, 1)}><Plus size={15} /></button>
                <span className="w-24 text-right font-semibold">{fmtMoney(l.quantite * l.prix_unitaire_ht)}</span>
                <button className="p-2 text-red-500 hover:bg-red-50 rounded-lg touch-manipulation" onClick={() => setCart(cart.filter((x) => x.produit_id !== l.produit_id))}><Trash2 size={14} /></button>
              </div>
            ))}
          </div>

          <div className="flex justify-between items-center text-xl font-bold border-t pt-2">
            <span>Total à payer</span><span className="text-blue-800">{fmtMoney(total)}</span>
          </div>

          <div className="grid grid-cols-2 gap-2 mt-2">
            <Select value={clientId} onChange={(e) => { setClientId(e.target.value); setPtsDebites(0); setPromoPct(0); setPromoLabel(""); }} title="Client (optionnel)" className="py-2.5">
              <option value="">Client comptoir</option>
              {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
            </Select>
            <Select value={mode} onChange={(e) => setMode(e.target.value)} title="Mode de paiement" className="py-2.5">
              {MODES_CAISSE.map((m) => <option key={m} value={m}>{labelOf(MODES_REGLEMENT, m)}</option>)}
            </Select>
          </div>
          {clientId && (
            <div className="flex items-center gap-2 mt-2 text-sm">
              <span className="text-amber-700 font-medium">★ {clientPts} pts fidélité</span>
              <Button className="bg-amber-600 !py-1 !px-2 !text-xs" disabled={clientPts <= 0 || cart.length === 0}
                onClick={utiliserPoints}>Convertir en remise</Button>
            </div>
          )}
          <div className="flex gap-2 mt-2">
            <Input placeholder="Code promo…" value={promoCode} onChange={(e) => setPromoCode(e.target.value.toUpperCase())} className="uppercase" />
            <Button className="bg-slate-600 shrink-0" onClick={appliquerPromo} disabled={!promoCode || cart.length === 0}>Appliquer</Button>
          </div>
          {promoLabel && (
            <p className="text-xs text-green-700 mt-1">Remise active : {promoLabel} <button className="underline" onClick={() => { setPromoPct(0); setPromoLabel(""); setPromoCode(""); }}>retirer</button></p>
          )}

          {/* Billets rapides */}
          <div className="grid grid-cols-5 gap-1.5 mt-2">
            {BILLETS.map((b) => (
              <button key={b} onClick={() => setRecuTxt(String((+recuTxt || 0) + b))}
                className="py-2 rounded-lg border text-sm font-semibold hover:bg-slate-100 active:scale-95 touch-manipulation">
                +{b >= 1000 ? `${b / 1000}k` : b}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-[1fr_110px] gap-2 mt-2">
            <div className="border rounded-xl px-3 py-2 flex items-center justify-between bg-slate-50">
              <span className="text-xs text-slate-500">Montant reçu</span>
              <b className="text-xl">{fmtMoney(recu)}</b>
            </div>
            <button onClick={() => setRecuTxt(String(total))}
              className="rounded-xl border border-blue-300 text-blue-700 text-sm font-semibold hover:bg-blue-50 active:scale-95 touch-manipulation">
              Montant exact
            </button>
          </div>

          {/* Pavé numérique tactile */}
          <div className="grid grid-cols-3 gap-1.5 mt-2">
            {["1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "0", "C"].map((k) => (
              <button key={k} onClick={() => numpad(k)}
                className="py-2.5 rounded-xl bg-slate-100 font-bold text-lg hover:bg-slate-200 active:scale-95 touch-manipulation">
                {k}
              </button>
            ))}
            <button onClick={() => numpad("⌫")} className="py-2.5 rounded-xl bg-slate-100 hover:bg-slate-200 active:scale-95 touch-manipulation flex justify-center">
              <Delete size={18} />
            </button>
          </div>

          <div className={`flex justify-between items-center mt-2 px-3 py-2 rounded-xl border text-lg font-bold ${rendu < 0 ? "text-red-600 border-red-300 bg-red-50" : "text-green-700 border-green-300 bg-green-50"}`}>
            <span>À rendre</span><span>{fmtMoney(Math.max(0, rendu))}</span>
          </div>

          <div className="grid grid-cols-2 gap-2 mt-2">
            <Button onClick={valider} disabled={cart.length === 0 || (recu || total) < total} className="py-3 text-base">
              <Check size={17} /> Encaisser
            </Button>
            <Button className="bg-slate-500 py-3" onClick={() => { setCart([]); setRecuTxt(""); }}><X size={16} /> Annuler</Button>
          </div>
          {cart.length > 0 && (recu || total) < total && (
            <p className="text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded-lg px-2 py-1 mt-2">
              Saisissez au moins {fmtMoney(total)} (pavé ou billets rapides) pour activer l'encaissement.
            </p>
          )}

          <div className="mt-3 pt-2 border-t">
            <Button className="bg-amber-600 w-full py-2.5" onClick={() => setTab("clotures")}>
              <Banknote size={16} /> Voir les clôtures
            </Button>
          </div>
        </Card>
      </div>
      )}
      {tab === "clotures" && (
        <div className="space-y-4">
          <Card>
            <div className="flex items-center justify-between mb-2">
              <h3 className="font-semibold">Clôture Z du jour ({todayISO()})</h3>
              <Button className="bg-amber-600" onClick={async () => {
                try { setCloture(await api.clotureZ(todayISO())); qc.invalidateQueries({ queryKey: ["clotures"] }); } catch (e: any) { setMsg(String(e)); }
              }}>
                <Banknote size={16} /> Calculer la clôture
              </Button>
            </div>
            {cloture && (
              <div className="text-sm mt-2 space-y-0.5 max-w-md">
                <div className="flex justify-between"><span>Tickets</span><b>{cloture.nb_ventes}</b></div>
                <div className="flex justify-between"><span>Espèces</span><b>{fmtMoney(cloture.especes)}</b></div>
                <div className="flex justify-between"><span>Mobile Money</span><b>{fmtMoney(cloture.mobile)}</b></div>
                <div className="flex justify-between"><span>Virements</span><b>{fmtMoney(cloture.virement)}</b></div>
                <div className="flex justify-between"><span>Chèques / CB</span><b>{fmtMoney(cloture.cheque + cloture.cb)}</b></div>
                <div className="flex justify-between font-bold border-t pt-1 text-base"><span>Total</span><span>{fmtMoney(cloture.total)}</span></div>
              </div>
            )}
          </Card>
          <Card>
            <h3 className="font-semibold mb-2">Historique des clôtures</h3>
            <DataView
              data={clotures || []}
              empty="Aucune clôture enregistrée."
              title={(z) => `Clôture du ${z.date}`}
              subtitle={(z) => `${z.nb_ventes} tickets`}
              fields={[
                { label: "Total", value: (z) => fmtMoney(z.total) },
                { label: "Espèces", value: (z) => fmtMoney(z.especes) },
                { label: "Mobile", value: (z) => fmtMoney(z.mobile) },
              ]}
            />
          </Card>
        </div>
      )}

      {ticket && (
        <PrintModal title={`Ticket ${ticket.numero}`} onClose={() => setTicket(null)}>
          <TicketCaisseDoc numero={ticket.numero} lignes={ticket.lignes} total={ticket.total_ttc}
            recu={ticket.montant_recu} rendu={ticket.rendu} mode={ticket.modeLabel} clientNom={ticket.clientNom}
            caissier={ticket.caissier} dateTime={ticket.dateTime} />
        </PrintModal>
      )}
    </div>
  );
}
