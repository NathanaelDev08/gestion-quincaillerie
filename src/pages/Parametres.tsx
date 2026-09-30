import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { Building2, DatabaseBackup, Download, Gauge, ListOrdered, RefreshCw, RotateCcw, Save, Settings2, ShieldCheck, Sparkles } from "lucide-react";
import { api } from "../services/api";
import { Button, Input, Card, PageHeader, Field, Select, Tabs, toast } from "../components/ui";
import { DEFAULT_PREFS, getPrefs, savePrefs } from "../stores/prefs";
import type { Prefs } from "../stores/prefs";

const COMPANY_KEY = "company";
const DEVISES = [
  { code: "XOF", label: "Franc CFA (XOF)" },
  { code: "EUR", label: "Euro (EUR)" },
  { code: "USD", label: "Dollar US (USD)" },
  { code: "GNF", label: "Franc guinéen (GNF)" },
  { code: "MAD", label: "Dirham (MAD)" },
];

export default function Parametres() {
  const [msg, setMsg] = useState("");
  const [tab, setTab] = useState<"societe" | "preferences" | "donnees" | "diagnostic">("societe");
  const [diag, setDiag] = useState<any | null>(null);
  const [company, setCompany] = useState<any>({ company_name: "", company_address: "", company_phone: "", company_email: "", company_siret: "", forme_juridique: "", capital: "", iban: "", conditions_paiement: "", logo: "" });
  const [prefs, setPrefs] = useState<Prefs>({ ...DEFAULT_PREFS, prefixes: { ...DEFAULT_PREFS.prefixes } });
  const { data: prochains } = useQuery({ queryKey: ["prochains"], queryFn: api.prochainsNumeros, enabled: tab === "preferences" });

  useEffect(() => {
    api.settingsGet(COMPANY_KEY).then((v) => { if (v) setCompany({ ...company, ...v }); }).catch(() => {});
    setPrefs(getPrefs());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onLogo = (file: File | undefined) => {
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        const size = 256;
        canvas.width = size; canvas.height = size;
        const ctx = canvas.getContext("2d")!;
        ctx.drawImage(img, 0, 0, size, size);
        setCompany({ ...company, logo: canvas.toDataURL("image/png") });
        setMsg("Logo chargé (pensez à Enregistrer)");
      };
      img.src = String(reader.result);
    };
    reader.readAsDataURL(file);
  };
  const saveCompany = async () => {
    try {
      await api.settingsSet(COMPANY_KEY, company);
      setMsg("Société enregistrée");
    } catch (e: any) { setMsg(String(e)); }
  };
  const savePreferences = async () => {
    try {
      await savePrefs(prefs);
      setMsg("Préférences enregistrées — appliquées aux nouveaux documents");
    } catch (e: any) { setMsg(String(e)); }
  };
  const doExport = async (entity: string) => {
    const csv = await api.exportCsv(entity);
    const blob = new Blob([csv], { type: "text/csv" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `${entity}.csv`;
    a.click();
    setMsg(`Export ${entity} OK`);
  };
  const doBackup = async () => {
    const p = await api.backup();
    setMsg("Backup : " + p);
  };
  const doRestore = async () => {
    const sel = await open({ multiple: false, filters: [{ name: "Sauvegarde", extensions: ["db"] }] });
    if (typeof sel !== "string" || !sel) return;
    if (!window.confirm("Restaurer cette sauvegarde ? L'application va redémarrer sur les données restaurées.")) return;
    await api.restoreDb(sel);
    setMsg("Base restaurée. Redémarrez l'application pour recharger les données.");
  };
  const doImport = async (entity: string, file: File | undefined) => {
    if (!file) return;
    const text = await file.text();
    const n = await api.importCsv(entity, text);
    setMsg(`Import ${entity} : ${n} lignes`);
  };
  const doSeed = async (force: boolean) => {
    if (force && !window.confirm("Réinitialiser TOUTES les données commerciales et charger la démo ? (comptes utilisateurs conservés)")) return;
    try {
      const r = await api.seedDemo(force);
      setMsg(r);
    } catch (e: any) { setMsg(String(e)); }
  };
  const doDiag = async () => {
    try {
      setDiag(await api.diagnostic());
      setMsg("Diagnostic terminé");
    } catch (e: any) { setMsg(String(e)); }
  };

  return (
    <div>
      <PageHeader title="Paramètres" subtitle="Société, préférences, données et sauvegardes" />
      <Tabs<"societe" | "preferences" | "donnees" | "diagnostic">
        active={tab} onChange={setTab}
        tabs={[
          { key: "societe", label: "Société", icon: Building2 },
          { key: "preferences", label: "Préférences", icon: Settings2 },
          { key: "donnees", label: "Données & sauvegardes", icon: DatabaseBackup },
          { key: "diagnostic", label: "Diagnostic & sécurité", icon: Gauge },
        ]}
      />
      {tab === "societe" && (
        <Card>
          <h3 className="font-semibold mb-2">Société (en-tête et pied des documents)</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <Field label="Nom de la société"><Input value={company.company_name} onChange={(e) => setCompany({ ...company, company_name: e.target.value })} /></Field>
            <Field label="Adresse complète"><Input value={company.company_address} onChange={(e) => setCompany({ ...company, company_address: e.target.value })} /></Field>
            <Field label="Téléphone"><Input placeholder="+225 ..." value={company.company_phone} onChange={(e) => setCompany({ ...company, company_phone: e.target.value })} /></Field>
            <Field label="Email professionnel"><Input type="email" value={company.company_email} onChange={(e) => setCompany({ ...company, company_email: e.target.value })} /></Field>
            <Field label="SIRET / RCCM"><Input value={company.company_siret} onChange={(e) => setCompany({ ...company, company_siret: e.target.value })} /></Field>
            <Field label="Forme juridique"><Input placeholder="Ex. SARL, SA, ETS" value={company.forme_juridique} onChange={(e) => setCompany({ ...company, forme_juridique: e.target.value })} /></Field>
            <Field label="Capital social (F CFA)"><Input type="number" value={company.capital} onChange={(e) => setCompany({ ...company, capital: e.target.value })} /></Field>
            <Field label="Compte bancaire (IBAN / RIB)"><Input value={company.iban} onChange={(e) => setCompany({ ...company, iban: e.target.value })} /></Field>
            <div className="col-span-2">
              <Field label="Logo de la société (PNG / JPG)">
                <div className="flex items-center gap-2">
                  {company.logo && <img src={company.logo} alt="Logo" className="h-12 w-12 object-contain border rounded" />}
                  <input type="file" accept="image/*" className="text-sm" onChange={(e) => onLogo(e.target.files?.[0])} />
                  {company.logo && <button className="text-xs text-red-600" onClick={() => setCompany({ ...company, logo: "" })}>Retirer</button>}
                </div>
              </Field>
            </div>
            <div className="col-span-2">
              <Field label="Conditions de règlement (pied de page)">
                <textarea className="px-3 py-2 rounded-lg border border-slate-300 w-full text-sm" rows={3}
                  value={company.conditions_paiement} onChange={(e) => setCompany({ ...company, conditions_paiement: e.target.value })}
                  placeholder="Laissez vide pour les mentions légales par défaut" />
              </Field>
            </div>
            <Button onClick={saveCompany}><Save size={15} /> Enregistrer</Button>
          </div>
        </Card>
      )}
      {tab === "preferences" && (
        <div className="space-y-4 max-w-2xl">
          <Card>
            <h3 className="font-semibold mb-2">TVA & devise par défaut</h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <Field label="TVA par défaut (%) — appliquée aux nouvelles lignes">
                <Input type="number" min={0} max={100} value={prefs.default_tva}
                  onChange={(e) => setPrefs({ ...prefs, default_tva: +e.target.value })} />
              </Field>
              <Field label="Devise d'affichage">
                <Select value={prefs.currency} onChange={(e) => setPrefs({ ...prefs, currency: e.target.value })}>
                  {DEVISES.map((d) => <option key={d.code} value={d.code}>{d.label}</option>)}
                </Select>
              </Field>
            </div>
          </Card>
          <Card>
            <h3 className="font-semibold mb-1">Préfixes de numérotation</h3>
            <p className="text-xs text-slate-500 mb-2">Appliqués aux nouveaux documents. Format : PRÉFIXE-AAAA-0001, sans trou par année et par préfixe.</p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
              {[
                ["devis", "Devis"], ["facture", "Factures"], ["livraison", "Bons de livraison"],
                ["commande", "Bons de commande"], ["avoir", "Avoirs"], ["bulletin", "Bulletins de paie"],
              ].map(([key, label]) => (
                <Field key={key} label={label}>
                  <Input value={prefs.prefixes[key] || ""} maxLength={6}
                    onChange={(e) => setPrefs({ ...prefs, prefixes: { ...prefs.prefixes, [key]: e.target.value.toUpperCase().replace(/[^A-Z]/g, "") } })} />
                </Field>
              ))}
            </div>
            <Button className="mt-3" onClick={savePreferences}><Save size={15} /> Enregistrer les préférences</Button>
          </Card>
          <Card>
            <h3 className="font-semibold mb-2">Prochains numéros (année en cours)</h3>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 text-sm">
              {(prochains || []).map((p: any) => (
                <div key={p.key} className="border rounded-lg px-2 py-1.5">
                  <div className="text-[11px] text-slate-500">{p.label}</div>
                  <div className="font-mono font-semibold">{p.prochain}</div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}
      {tab === "donnees" && (
        <Card>
          <h3 className="font-semibold mb-2">Exports CSV</h3>
          <div className="flex gap-2 mb-4 flex-wrap">
            <Button onClick={() => doExport("clients")}><Download size={15} /> Clients</Button>
            <Button onClick={() => doExport("produits")}><Download size={15} /> Produits</Button>
            <Button onClick={() => doExport("factures")}><Download size={15} /> Factures</Button>
          </div>
          <h3 className="font-semibold mb-2">Import CSV clients (nom,prenom,email,tel)</h3>
          <input type="file" accept=".csv" className="text-sm mb-4"
            onChange={(e) => doImport("clients", e.target.files?.[0])} />
          <h3 className="font-semibold mb-2">Sauvegarde</h3>
          <div className="flex gap-2 mb-4">
            <Button onClick={doBackup}><DatabaseBackup size={15} /> Backup BDD</Button>
            <Button className="bg-amber-600" onClick={doRestore}><RotateCcw size={15} /> Restaurer</Button>
          </div>
          <h3 className="font-semibold mb-2">Données de démonstration</h3>
          <p className="text-xs text-slate-500 mb-2">8 clients, 4 fournisseurs, 12 produits, 4 devis, 5 factures, règlements, dépenses, écritures — contexte ivoirien en F CFA.</p>
          <div className="flex gap-2">
            <Button onClick={() => doSeed(false)}><Sparkles size={15} /> Charger la démo</Button>
            <Button className="bg-amber-600" onClick={() => doSeed(true)}><RefreshCw size={15} /> Forcer / Réinitialiser</Button>
          </div>
          {msg && <p className="text-sm text-green-700 mt-3">{msg}</p>}
        </Card>
      )}
      {tab === "diagnostic" && (
        <div className="space-y-4 max-w-3xl">
          <Card>
            <div className="flex items-center justify-between mb-2">
              <h3 className="font-semibold">Montée en charge — test de performance</h3>
              <Button onClick={doDiag}><Gauge size={15} /> Lancer le diagnostic</Button>
            </div>
            {!diag && <p className="text-sm text-slate-500">Mesure le volume des tables et le temps des requêtes critiques (objectif &lt; 500 ms).</p>}
            {diag && (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <h4 className="text-xs font-semibold uppercase text-slate-500 mb-1">Requêtes critiques</h4>
                  {diag.mesures.map((m: any) => (
                    <div key={m.requete} className="flex justify-between text-sm py-1 border-b">
                      <span>{m.requete} <span className="text-slate-400">({m.lignes} lignes)</span></span>
                      <b className={m.ok ? "text-green-700" : "text-red-600"}>{m.ms} ms {m.ok ? "✓" : "✗"}</b>
                    </div>
                  ))}
                </div>
                <div>
                  <h4 className="text-xs font-semibold uppercase text-slate-500 mb-1">Volume & index</h4>
                  <div className="text-sm py-1 border-b flex justify-between"><span>Index de performance</span><b>{diag.index_perso}</b></div>
                  {Object.entries(diag.tables as Record<string, number>).map(([t, n]) => (
                    <div key={t} className="flex justify-between text-sm py-0.5 border-b">
                      <span className="font-mono text-xs">{t}</span><b>{n}</b>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </Card>
          <Card>
            <h3 className="font-semibold mb-2 flex items-center gap-1.5"><ShieldCheck size={15} /> Bonnes pratiques de sécurité actives</h3>
            <ul className="text-sm space-y-1">
              <li>✓ Mot de passe : 8 caractères minimum, hachage Argon2id</li>
              <li>✓ Secret JWT unique par installation (plus de secret codé en dur)</li>
              <li>✓ Anti-bruteforce : verrouillage 15 min après 5 échecs</li>
              <li>✓ Rôles vérifiés côté serveur (admin requis : utilisateurs, sauvegardes, clôtures, démo)</li>
              <li>✓ Sauvegardes avec empreinte SHA256 vérifiée à la restauration</li>
              <li>✓ Requêtes SQL paramétrées (aucune concaténation)</li>
              <li>✓ Suppressions protégées (pas de suppression en cascade silencieuse)</li>
            </ul>
          </Card>
        </div>
      )}
    </div>
  );
}
