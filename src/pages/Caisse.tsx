import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Banknote, Check, ChevronRight, Delete, History, Minus, Plus, Printer, ShoppingBag, Star, Tag, Trash2, User, Wallet, Smartphone, CreditCard, ArrowLeftRight, WifiOff, X } from "lucide-react";
import { api } from "../services/api";
import { useAuth } from "../stores/useAuth";
import { Button, Input, Card, PageHeader, Select, DataView, Tabs, toast } from "../components/ui";
import { TicketCaisseDoc, PrintModal } from "../components/PrintDoc";
import { fmtMoney, labelOf, MODES_REGLEMENT, todayISO } from "../utils/format";
import {
  calculerTotaux, prixTTC, renduMonnaie, peutEncaisser,
  rechercherArticle, ajouterAuPanier, ajusterQuantite, saisiePavé, stockSuffisant,
  type LigneCalcul,
} from "../utils/caisse";

/** Ligne du panier : les champs de calcul + le stock disponible (pour
 *  contrôler les quantités). `remise` à 0 : les remises sont gérées au niveau
 *  du ticket (promo), pas ligne par ligne. */
type CartLine = LigneCalcul & { produit_id: string; stock: number };

const MODES_CAISSE = [
  { key: "especes", label: "Espèces", icon: Banknote },
  { key: "mobile", label: "Mobile", icon: Smartphone },
  { key: "cb", label: "Carte", icon: CreditCard },
  { key: "virement", label: "Virement", icon: ArrowLeftRight },
];
const BILLETS = [500, 1000, 2000, 5000, 10000];

export default function Caisse() {
  const qc = useQueryClient();
  const { user } = useAuth();
  const { data: sante, refetch: recheckSante } = useQuery({
    queryKey: ["sante"],
    queryFn: api.santeEtat,
    refetchInterval: 20000,
  });
  const [tab, setTab] = useState<"vente" | "clotures">("vente");
  const [etape, setEtape] = useState<1 | 2>(1);
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

  // Le catalogue est filtré CÔTÉ SERVEUR. Charger 5 000 articles d'un coup
  // pour n'en afficher que 120 ralentissait la caisse sur un gros catalogue.
  // `q` et `cat` sont appliqués par le backend ; le filtre local ne sert plus
  // que de sécurité visuelle.
  const { data: produits, isFetching: rechercheEnCours } = useQuery({
    queryKey: ["caisse-catalogue", q, cat],
    queryFn: () => api.produitsCaisse(q, cat, 120),
    // `staleTime` évite de re-requêter à chaque frappe : le serveur filtre
    // déjà, le cache reste chaud pendant une vente.
    staleTime: 30_000,
  });
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

  const catalogue = useMemo(() => produits || [], [produits]);

  const favoris = useMemo(() => {
    const names = new Set((top || []).slice(0, 6).map((t: any) => t.nom));
    return (catalogue || []).filter((p: any) => names.has(p.designation)).slice(0, 6);
  }, [catalogue, top]);

  const ventesJour = useMemo(() => {
    const t = todayISO();
    const list = (factures?.data || []).filter((f: any) => f.date_emission === t);
    return { nb: list.length, total: list.reduce((s: number, f: any) => s + f.total_ttc, 0) };
  }, [factures]);

  const { ht: totalHT, tva: totalTVA, ttc: total, nbArticles } = calculerTotaux(cart, promoPct);
  const recu = +recuTxt || 0;
  const rendu = renduMonnaie(total, recu);
  const encaissable = peutEncaisser(cart, total, recu || total);
  const qtyInCart = (id: string) => cart.find((l) => l.produit_id === id)?.quantite ?? 0;

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
    if (!stockSuffisant(p.stock, cur, qty)
      && !window.confirm(`Stock insuffisant pour ${p.designation} (reste ${p.stock}). Ajouter quand même ?`)) {
      return;
    }
    setCart(ajouterAuPanier(cart, {
      produit_id: p.id,
      designation: p.designation,
      quantite: qty,
      prix_unitaire_ht: p.prix_vente_ht,
      taux_tva: p.taux_tva,
      remise: 0,
      stock: p.stock,
    }, qty));
  };

  const onSearchKey = (e: React.KeyboardEvent) => {
    // F1-F12 sont capturés avant la saisie : sinon la touche « /» d'un
    // pavé numérique ouvrirait la recherche au lieu de saisir un chiffre.
    const raccourcis: Record<string, () => void> = {
      F2: () => { if (encaissable) valider(); },
      F4: () => setMode("especes"),
      F5: () => setMode("mobile"),
      F6: () => setMode("cb"),
      F7: () => setMode("virement"),
      Escape: () => { if (etape === 2) setEtape(1); else if (cart.length) resetTicket(); },
    };
    if (raccourcis[e.key]) {
      e.preventDefault();
      raccourcis[e.key]();
      return;
    }
    if (e.key === "Enter") {
      // Recherche ciblée : un scan doit tomber sur le bon article même si le
      // catalogue est filtré par catégorie ou par recherche partielle.
      const cible = rechercherArticle(q, catalogue) ?? catalogue[0];
      if (cible) {
        add(cible);
        setQ("");
      }
    }
  };

  const chqte = (id: string, d: number) => setCart(ajusterQuantite(cart, id, d));

  const resetTicket = () => {
    setCart([]); setRecuTxt(""); setPromoPct(0); setPromoLabel(""); setPromoCode(""); setPtsDebites(0); setEtape(1);
  };

  const valider = async () => {
    setMsg("");
    const lignes = cart.map((l) => ({ ...l, remise: Math.round(promoPct * 100) / 100 }));
    try {
      const r = await api.venteComptoir({
        client_id: clientId || null,
        lignes,
        mode, montant_recu: recu || total,
      });
      const cli = (clients?.data || []).find((c) => c.id === clientId);
      const now = new Date();
      const dateTime = now.toLocaleDateString("fr-FR") + " " + now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
      setTicket({ ...r, lignes: cart, modeLabel: labelOf(MODES_REGLEMENT, mode), clientNom: cli ? cli.nom : "Client comptoir", caissier: user?.full_name || user?.username, dateTime, horsLigne: false, remiseGlobalePct: promoPct });
      resetTicket();
      toast.success(`Vente ${r.numero} encaissée — rendu ${fmtMoney(r.rendu)}`);
      qc.invalidateQueries({ queryKey: ["produits-all"] });
      qc.invalidateQueries({ queryKey: ["factures-all"] });
      qc.invalidateQueries({ queryKey: ["stock"] });
      recheckSante();
    } catch (e: any) {
      if (ptsDebites > 0 && clientId) {
        try { await api.fideliteUtiliser(clientId, -ptsDebites); } catch { /* ignore */ }
        setPtsDebites(0);
        setPromoPct(0);
        setPromoLabel("");
      }
      // La caisse ne doit pas s'arrêter de vendre si la base ne répond plus :
      // la vente est conservée localement et rejouée au retour à la normale.
      const text = String(e);
      const baseIndisponible = /base de données|database|locked|disk I\/O|no such/i.test(text);
      if (baseIndisponible) {
        const numero = `ATT-${new Date().toTimeString().slice(0, 5).replace(":", "")}-${Math.floor(Math.random() * 900 + 100)}`;
        try {
          await api.horsLigneEnregistrer({
            id: numero,
            horodatage: new Date().toISOString(),
            numero,
            total_ttc: total,
            mode,
            client_id: clientId || null,
            lignes,
            montant_recu: recu || total,
          });
          const cli = (clients?.data || []).find((c) => c.id === clientId);
          const now = new Date();
          setTicket({
            numero, total_ttc: total, montant_recu: recu || total, rendu: (recu || total) - total,
            lignes: cart, modeLabel: labelOf(MODES_REGLEMENT, mode),
            clientNom: cli ? cli.nom : "Client comptoir",
            caissier: user?.full_name || user?.username,
            dateTime: now.toLocaleDateString("fr-FR") + " " + now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
            remiseGlobalePct: promoPct,
            horsLigne: true,
          });
          resetTicket();
          toast.info("Base indisponible : vente enregistrée hors-ligne, à rejouer ensuite");
          recheckSante();
          return;
        } catch (e2: any) {
          setMsg(`Vente non enregistrée (base indisponible ET journal local inaccessible) : ${text}`);
          return;
        }
      }
      setMsg(text);
    }
  };

  const numpad = (k: string) => setRecuTxt((s) => saisiePavé(s, k));

  return (
    <div className="select-none" onKeyDown={onSearchKey}>
      <PageHeader title="Caisse" subtitle={`${ventesJour.nb} tickets • ${fmtMoney(ventesJour.total)} aujourd'hui`} />
      {/* Raccourcis : une caisse rapide se pilote au clavier. */}
      <p className="text-[11px] text-slate-400 mb-2">
        <b>Entrée</b> ajouter l'article • <b>F2</b> encaisser • <b>F4</b> espèces •
        {" "}<b>F5</b> mobile • <b>F6</b> carte • <b>Échap</b> annuler le ticket
      </p>
      {msg && <p className="text-sm text-red-600 bg-red-50 border border-red-200 rounded-lg px-3 py-2 mb-2">{msg}</p>}

      {sante && sante.etat !== "Ok" && (
        <div className={`flex items-center gap-2 px-3 py-2 rounded-lg border text-sm mb-2 ${
          sante.etat === "HorsLigne"
            ? "bg-red-50 border-red-200 text-red-800"
            : "bg-amber-50 border-amber-200 text-amber-800"
        }`}>
          <WifiOff size={16} />
          <span className="flex-1">{sante.libelle}{sante.en_attente > 0 ? ` — ${sante.en_attente} vente(s) en attente de synchronisation` : ""}</span>
          <button className="text-xs underline shrink-0" onClick={() => recheckSante()}>Vérifier</button>
        </div>
      )}

      {/* Étapes */}
      {tab === "vente" && (
        <div className="flex items-center gap-2 mb-3 text-sm">
          <button onClick={() => setEtape(1)} className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full font-medium ${etape === 1 ? "bg-blue-600 text-white" : "bg-white border text-slate-600"}`}>
            <ShoppingBag size={14} /> 1. Articles {nbArticles > 0 && <span className="bg-white/25 rounded-full px-1.5 text-xs">{nbArticles}</span>}
          </button>
          <ChevronRight size={15} className="text-slate-300" />
          <button onClick={() => cart.length > 0 && setEtape(2)} disabled={cart.length === 0} className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full font-medium ${etape === 2 ? "bg-blue-600 text-white" : "bg-white border text-slate-600"} disabled:opacity-40`}>
            <Wallet size={14} /> 2. Paiement • {fmtMoney(total)}
          </button>
          <div className="ml-auto hidden sm:block">
            <Tabs<"vente" | "clotures"> active={tab} onChange={setTab} tabs={[{ key: "vente", label: "Encaissement", icon: ShoppingBag }, { key: "clotures", label: "Clôtures", icon: History }]} />
          </div>
        </div>
      )}

      {tab === "vente" && etape === 1 && (
        <div className="grid grid-cols-1 xl:grid-cols-[1fr_360px] gap-3">
          <Card>
            <div className="flex gap-2 mb-2">
              <Input placeholder="Scanner ou taper : ciment, vis, code-barres… puis Entrée" autoFocus
                value={q} onChange={(e) => setQ(e.target.value)} className="text-base py-3" />
            </div>
            <div className="flex gap-1.5 overflow-auto pb-2 mb-1">
              <button onClick={() => setCat("")} className={`shrink-0 px-3 py-1.5 rounded-full text-sm font-medium border ${cat === "" ? "bg-slate-900 text-white border-slate-900" : "bg-white text-slate-600"}`}>Tout</button>
              {(cats || []).map((c) => (
                <button key={c} onClick={() => setCat(cat === c ? "" : c)} title={c}
                  className={`shrink-0 px-3 py-1.5 rounded-full text-sm font-medium border max-w-[160px] truncate ${cat === c ? "bg-blue-600 text-white border-blue-600" : "bg-white text-slate-600"}`}>{c}</button>
              ))}
            </div>
            {favoris.length > 0 && !q && (
              <div className="mb-2">
                <div className="text-[11px] uppercase text-slate-400 mb-1 flex items-center gap-1"><Star size={11} /> Fréquents</div>
                <div className="flex gap-2 overflow-auto pb-1">
                  {favoris.map((p: any) => (
                    <button key={p.id} onClick={() => add(p)} disabled={p.stock <= 0}
                      className="shrink-0 border border-amber-300 bg-amber-50 rounded-xl px-3 py-2 text-left active:scale-95 disabled:opacity-40">
                      <div className="font-medium text-sm whitespace-nowrap">{p.designation}</div>
                      <div className="text-blue-800 font-bold text-sm">{fmtMoney(prixTTC(p.prix_vente_ht, p.taux_tva))} TTC</div>
                    </button>
                  ))}
                </div>
              </div>
            )}
            <div className="grid grid-cols-2 sm:grid-cols-3 2xl:grid-cols-4 gap-2 max-h-[58vh] overflow-auto">
              {catalogue.map((p: any) => {
                const inCart = qtyInCart(p.id);
                return (
                  <button key={p.id} onClick={() => add(p)} disabled={p.stock <= 0}
                    className={`relative border rounded-xl p-3 text-left active:scale-[0.98] hover:border-blue-400 hover:bg-blue-50/50 transition disabled:opacity-40 min-h-[92px] flex flex-col justify-between ${inCart > 0 ? "border-blue-500 bg-blue-50/60 ring-1 ring-blue-500" : ""}`}>
                    {inCart > 0 && <span className="absolute -top-2 -right-2 bg-blue-600 text-white text-xs font-bold rounded-full min-w-[24px] h-6 flex items-center justify-center px-1">×{inCart}</span>}
                    <div className="font-medium truncate">{p.designation}</div>
                    <div className="text-xs text-slate-500">{p.reference} • <b className={p.stock <= p.stock_alerte ? "text-red-600" : ""}>{p.stock}</b> dispo</div>
                    <div className="font-bold text-blue-800 text-base">{fmtMoney(prixTTC(p.prix_vente_ht, p.taux_tva))} <span className="text-[11px] font-normal text-slate-400">TTC</span></div>
                  </button>
                );
              })}
              {catalogue.length === 0 && <p className="col-span-full text-sm text-slate-400 py-8 text-center">Aucun article — modifie la recherche ou la catégorie.</p>}
            </div>
          </Card>

          {/* Résumé ticket */}
          <Card className="h-fit xl:sticky xl:top-2">
            <h3 className="font-semibold mb-1 flex items-center gap-1.5"><ShoppingBag size={16} /> Ticket ({nbArticles})</h3>
            <div className="space-y-1.5 max-h-[38vh] overflow-auto mb-2">
              {cart.length === 0 && <p className="text-sm text-slate-400">Touche un article à gauche pour commencer.</p>}
              {cart.map((l) => (
                <div key={l.produit_id} className="flex items-center gap-1 text-sm border rounded-xl px-2 py-1.5">
                  <span className="flex-1 truncate font-medium">{l.designation}</span>
                  <button className="p-2 hover:bg-slate-100 rounded-lg" onClick={() => chqte(l.produit_id, -1)}><Minus size={15} /></button>
                  <b className="w-7 text-center text-base">{l.quantite}</b>
                  <button className="p-2 hover:bg-slate-100 rounded-lg" onClick={() => chqte(l.produit_id, 1)}><Plus size={15} /></button>
                  <span className="w-20 text-right font-semibold text-xs">{fmtMoney(prixTTC(l.quantite * l.prix_unitaire_ht * (1 - promoPct / 100), l.taux_tva))}</span>
                </div>
              ))}
            </div>
            <div className="flex justify-between items-center text-xl font-bold border-t pt-2">
              <span>Total TTC</span><span className="text-blue-800">{fmtMoney(total)}</span>
            </div>
            {promoLabel && <p className="text-xs text-green-700 mt-1">{promoLabel}</p>}
            <Button onClick={() => setEtape(2)} disabled={cart.length === 0} className="w-full py-3 text-base mt-2">
              Continuer vers paiement <ChevronRight size={17} />
            </Button>
            {cart.length > 0 && <button onClick={resetTicket} className="w-full text-xs text-slate-400 hover:text-red-600 mt-1.5 underline">Vider le ticket</button>}
          </Card>
        </div>
      )}

      {tab === "vente" && etape === 2 && (
        <div className="grid grid-cols-1 xl:grid-cols-[1fr_380px] gap-3 max-w-[1100px]">
          <Card>
            <button onClick={() => setEtape(1)} className="flex items-center gap-1 text-sm text-slate-500 hover:text-slate-800 mb-2"><ArrowLeft size={14} /> Retour articles</button>
            <h3 className="font-semibold mb-2">Mode de paiement</h3>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mb-3">
              {MODES_CAISSE.map((m) => (
                <button key={m.key} onClick={() => setMode(m.key)}
                  className={`flex flex-col items-center gap-1 py-3 rounded-xl border-2 font-medium text-sm active:scale-95 ${mode === m.key ? "border-blue-600 bg-blue-50 text-blue-800" : "border-slate-200 text-slate-500"}`}>
                  <m.icon size={20} /> {m.label}
                </button>
              ))}
            </div>
            <h3 className="font-semibold mb-2">💰 Montant donné par le client <span className="font-normal text-slate-400 text-xs">— pour calculer la monnaie</span></h3>
            <div className="flex gap-2 mb-2">
              <Input type="number" min={0} autoFocus placeholder={`Ex : ${Math.round(total)}`} value={recuTxt}
                onChange={(e) => setRecuTxt(e.target.value.replace(/[^0-9]/g, "").slice(0, 9))}
                className="!text-2xl font-bold py-3 tabular-nums" title="Tape le montant reçu du client" />
              <div className="flex flex-col gap-1.5 shrink-0">
                <button onClick={() => setRecuTxt(String(Math.round(total)))} className="px-3 py-1.5 rounded-xl bg-blue-600 text-white text-sm font-semibold active:scale-95 whitespace-nowrap">Montant exact</button>
                <button onClick={() => setRecuTxt("")} className="px-3 py-1.5 rounded-xl border text-sm text-slate-500 hover:bg-slate-50">Effacer</button>
              </div>
            </div>
            <div className="grid grid-cols-5 gap-1.5 mb-2">
              {BILLETS.map((b) => (
                <button key={b} onClick={() => setRecuTxt(String((+recuTxt || 0) + b))} title={`Ajouter ${b} F`}
                  className="py-2.5 rounded-lg border text-sm font-semibold hover:bg-slate-100 active:scale-95">+{b >= 1000 ? `${b / 1000}k` : b}</button>
              ))}
            </div>
            <div className="grid grid-cols-3 gap-1.5 max-w-[300px]">
              {["1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "0", "C"].map((k) => (
                <button key={k} onClick={() => numpad(k)} className="py-2.5 rounded-xl bg-slate-100 font-bold text-lg hover:bg-slate-200 active:scale-95">{k}</button>
              ))}
            </div>
            <details className="mt-3 border rounded-xl">
              <summary className="px-3 py-2 text-sm font-medium cursor-pointer flex items-center gap-1.5"><Tag size={14} /> Client, remise, fidélité (optionnel)</summary>
              <div className="p-3 pt-1 space-y-2">
                <Select value={clientId} onChange={(e) => { setClientId(e.target.value); setPtsDebites(0); setPromoPct(0); setPromoLabel(""); }} title="Client">
                  <option value="">Client comptoir</option>
                  {(clients?.data || []).map((c) => <option key={c.id} value={c.id}>{c.nom}</option>)}
                </Select>
                {clientId && clientPts > 0 && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-amber-700 font-medium">★ {clientPts} pts</span>
                    <Button className="bg-amber-600 !py-1 !px-2 !text-xs" onClick={utiliserPoints}>Convertir en remise</Button>
                  </div>
                )}
                <div className="flex gap-2">
                  <Input placeholder="Code promo…" value={promoCode} onChange={(e) => setPromoCode(e.target.value.toUpperCase())} className="uppercase" />
                  <Button className="bg-slate-600 shrink-0" onClick={appliquerPromo} disabled={!promoCode}>OK</Button>
                </div>
                {promoLabel && <p className="text-xs text-green-700">{promoLabel} <button className="underline" onClick={() => { setPromoPct(0); setPromoLabel(""); setPromoCode(""); }}>retirer</button></p>}
              </div>
            </details>
          </Card>

          <Card className="h-fit xl:sticky xl:top-2">
            <h3 className="font-semibold mb-2 flex items-center gap-1.5"><User size={15} /> Récapitulatif</h3>
            <div className="text-sm space-y-1 max-h-[24vh] overflow-auto mb-2">
              {cart.map((l) => (
                <div key={l.produit_id} className="flex justify-between gap-2">
                  <span className="truncate">{l.quantite} × {l.designation}</span>
                  <b className="shrink-0">{fmtMoney(prixTTC(l.quantite * l.prix_unitaire_ht * (1 - promoPct / 100), l.taux_tva))}</b>
                </div>
              ))}
            </div>
            <div className="border-t pt-2 space-y-1 text-sm">
              <div className="flex justify-between"><span className="text-slate-500">Total HT</span><span>{fmtMoney(totalHT)}</span></div>
              <div className="flex justify-between"><span className="text-slate-500">TVA</span><span>{fmtMoney(totalTVA)}</span></div>
              <div className="flex justify-between"><span className="text-slate-500 font-medium">Total à payer (TTC)</span><b className="text-xl text-blue-800">{fmtMoney(total)}</b></div>
              <div className="flex justify-between"><span className="text-slate-500">Reçu</span><b>{fmtMoney(recu)}</b></div>
              <div className={`flex justify-between items-center px-3 py-2 rounded-xl border text-lg font-bold ${rendu < 0 ? "text-red-600 border-red-300 bg-red-50" : "text-green-700 border-green-300 bg-green-50"}`}>
                <span>À rendre</span><span>{fmtMoney(Math.max(0, rendu))}</span>
              </div>
            </div>
            <Button onClick={valider} disabled={!encaissable} className="w-full py-3.5 text-lg mt-3 !bg-green-600">
              <Check size={19} /> Encaisser {fmtMoney(total)}
            </Button>
            {!encaissable && <p className="text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded-lg px-2 py-1.5 mt-2">Saisis au moins {fmtMoney(total)} via Exact, billets ou pavé. Raccourci : F2.</p>}
            <div className="grid grid-cols-2 gap-2 mt-2">
              <Button className="bg-slate-500" onClick={() => setEtape(1)}><ArrowLeft size={15} /> Articles</Button>
              <Button className="bg-white !text-slate-500 border" onClick={resetTicket}><Trash2 size={14} /> Annuler</Button>
            </div>
          </Card>
        </div>
      )}

      {tab === "clotures" && (
        <div className="space-y-4 max-w-[800px]">
          <div className="sm:hidden"><Tabs<"vente" | "clotures"> active={tab} onChange={setTab} tabs={[{ key: "vente", label: "Encaissement", icon: ShoppingBag }, { key: "clotures", label: "Clôtures", icon: History }]} /></div>
          <Card>
            <div className="flex items-center justify-between mb-2 flex-wrap gap-2">
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
            <DataView data={clotures || []} empty="Aucune clôture enregistrée."
              title={(z) => `Clôture du ${z.date}`} subtitle={(z) => `${z.nb_ventes} tickets`}
              fields={[{ label: "Total", value: (z) => fmtMoney(z.total) }, { label: "Espèces", value: (z) => fmtMoney(z.especes) }, { label: "Mobile", value: (z) => fmtMoney(z.mobile) }]} />
          </Card>
        </div>
      )}

      {ticket && (
        <PrintModal title={`Ticket ${ticket.numero}`} onClose={() => setTicket(null)}>
          {ticket.horsLigne && (
            <p className="text-xs bg-amber-50 border border-amber-300 text-amber-800 rounded-lg px-2 py-1.5 mb-2">
              ⚠ Vente enregistrée <b>hors-ligne</b> : le stock et la facture seront mis à jour au retour de la base.
              Conserve ce ticket.
            </p>
          )}
          <TicketCaisseDoc numero={ticket.numero} lignes={ticket.lignes} total={ticket.total_ttc}
            remiseGlobalePct={ticket.remiseGlobalePct ?? 0}
            recu={ticket.montant_recu} rendu={ticket.rendu} mode={ticket.modeLabel} clientNom={ticket.clientNom}
            caissier={ticket.caissier} dateTime={ticket.dateTime} />
          <div className="flex gap-2 mt-2">
            <Button className="flex-1" onClick={() => { setTicket(null); setEtape(1); }}><Plus size={15} /> Nouvelle vente</Button>
            <Button className="bg-slate-500" onClick={() => window.print()}><Printer size={15} /> Imprimer</Button>
          </div>
        </PrintModal>
      )}
    </div>
  );
}
