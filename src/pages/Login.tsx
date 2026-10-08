import { useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { AlertTriangle, ArrowRight, BarChart3, Eye, EyeOff, Loader2, Lock, Receipt, Store, User, Wallet, UserPlus, ShieldCheck } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../services/api";
import { useAuth } from "../stores/useAuth";
import { Button, Input, Card, Field } from "../components/ui";

function BrandPanel() {
  const points = [
    { icon: Receipt, text: "Devis, factures et avoirs en Franc CFA" },
    { icon: Wallet, text: "Caisse tactile, stock et comptabilité" },
    { icon: BarChart3, text: "Pilotage : marges, tops et impayés" },
  ];
  return (
    <div className="hidden md:flex flex-col justify-between bg-gradient-to-br from-blue-900 via-blue-800 to-blue-950 text-white p-8 w-[420px] shrink-0">
      <div className="flex items-center gap-2 font-bold text-lg">
        <span className="bg-white/15 rounded-lg p-2"><Store size={20} /></span>
        Gestion Quincaillerie
      </div>
      <div className="space-y-4">
        <h2 className="text-2xl font-bold leading-snug">Pilotez votre activité<br />en toute simplicité.</h2>
        <ul className="space-y-2.5">
          {points.map((p, i) => (
            <li key={i} className="flex items-center gap-2.5 text-sm text-blue-100">
              <span className="bg-white/15 rounded-lg p-1.5"><p.icon size={15} /></span>
              {p.text}
            </li>
          ))}
        </ul>
      </div>
      <p className="text-xs text-blue-200">Devis • Factures • Stock • Caisse • Paie • Comptabilité — 100 % hors-ligne</p>
    </div>
  );
}

function CapsWarning({ on }: { on: boolean }) {
  if (!on) return null;
  return (
    <p className="flex items-center gap-1 text-xs text-amber-600">
      <AlertTriangle size={13} /> Verr Maj activée — attention aux majuscules.
    </p>
  );
}

export default function Login() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [show, setShow] = useState(false);
  const [caps, setCaps] = useState(false);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const { setAuth } = useAuth();
  const nav = useNavigate();
  // Détecte une installation neuve : aucun compte n'existe encore.
  const { data: nbUsers } = useQuery({
    queryKey: ["nb-utilisateurs"],
    queryFn: api.nbUtilisateurs,
    retry: false,
  });
  const installationNeuve = nbUsers === 0;

  const submit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (busy) return;
    try {
      setErr("");
      setBusy(true);
      const r = await api.login(username, password);
      setAuth(r.user, r.token, r.refresh);
      nav("/");
    } catch (e: any) {
      // Une session périmée n'a rien à signaler ici : l'utilisateur est
      // justement en train de se connecter. Tout autre cas est affiché.
      const msg = String(e);
      setErr(
        /session invalide|session expirée|expired/i.test(msg)
          ? "Identifiants incorrects. Vérifie ton nom d'utilisateur et ton mot de passe."
          : msg,
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-slate-100 p-4">
      <div className="flex w-full max-w-3xl rounded-2xl overflow-hidden shadow-xl bg-white min-h-[480px]">
        <BrandPanel />
        <div className="flex-1 p-6 sm:p-8 flex flex-col justify-center">
          {installationNeuve ? (
            <>
              <div className="flex items-center gap-2 mb-1">
                <span className="bg-blue-50 text-blue-700 p-1.5 rounded-lg"><UserPlus size={17} /></span>
                <h1 className="text-xl font-bold">Première installation</h1>
              </div>
              <p className="text-sm text-slate-500 mb-5">
                Aucun compte n'existe encore. Créez le compte administrateur de votre quincaillerie.
              </p>
              <Button className="w-full py-2.5 justify-center" onClick={() => nav("/register")}>
                <ShieldCheck size={16} /> Créer le compte administrateur
              </Button>
              <p className="text-xs text-slate-400 mt-3">
                Choisissez votre propre mot de passe : il n'est connu que de vous.
              </p>
            </>
          ) : (
            <>
              <h1 className="text-xl font-bold mb-1">Bon retour !</h1>
              <p className="text-sm text-slate-500 mb-5">Connectez-vous à votre espace de gestion</p>
            </>
          )}
          {!installationNeuve && (
          <form onSubmit={submit} className="space-y-3">
            <Field label="Nom d'utilisateur">
              <div className="relative">
                <User size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                <Input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Ex. admin" className="pl-9" autoComplete="username" />
              </div>
            </Field>
            <div>
              <Field label="Mot de passe">
                <div className="relative">
                  <Lock size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                  <Input type={show ? "text" : "password"} value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    onKeyUp={(e) => setCaps((e.nativeEvent as KeyboardEvent).getModifierState?.("CapsLock") ?? false)}
                    placeholder="Votre mot de passe" className="pl-9 pr-9" autoComplete="current-password" />
                  <button type="button" onClick={() => setShow((v) => !v)} title={show ? "Masquer" : "Afficher"}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600">
                    {show ? <EyeOff size={15} /> : <Eye size={15} />}
                  </button>
                </div>
              </Field>
              <CapsWarning on={caps} />
            </div>
            {err && (
              <p className="flex items-start gap-1.5 text-xs text-red-700 bg-red-50 border border-red-200 rounded-lg px-3 py-2">
                <AlertTriangle size={14} className="shrink-0 mt-px" /> {err}
              </p>
            )}
            <Button className="w-full py-2.5 justify-center" disabled={busy || !username || !password}>
              {busy ? <Loader2 size={16} className="animate-spin" /> : <ArrowRight size={16} />}
              {busy ? "Connexion…" : "Se connecter"}
            </Button>
            <Link to="/register" className="text-xs text-blue-700 block text-center hover:underline">Créer un autre compte</Link>
          </form>
          )}
        </div>
      </div>
    </div>
  );
}

export function Register() {
  const [f, setF] = useState({ username: "", password: "", fullName: "", email: "" });
  const [show, setShow] = useState(false);
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);
  const nav = useNavigate();
  const { data: nbUsers } = useQuery({
    queryKey: ["nb-utilisateurs"],
    queryFn: api.nbUtilisateurs,
    retry: false,
  });
  const submit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (busy) return;
    setBusy(true);
    try {
      await api.register(f.username, f.password, f.fullName, f.email);
      setMsg("Compte créé, connectez-vous");
      setTimeout(() => nav("/login"), 1000);
    } catch (e: any) { setMsg(String(e)); }
    finally { setBusy(false); }
  };
  return (
    <div className="min-h-screen flex items-center justify-center bg-slate-100 p-4">
      <div className="flex w-full max-w-3xl rounded-2xl overflow-hidden shadow-xl bg-white min-h-[480px]">
        <BrandPanel />
        <div className="flex-1 p-6 sm:p-8 flex flex-col justify-center">
          <h1 className="text-xl font-bold mb-1">Créer un compte</h1>
          <p className="text-sm text-slate-500 mb-5">
            {nbUsers === 0
              ? "Première installation : ce compte sera l'administrateur de la quincaillerie."
              : "Les nouveaux comptes sont créés avec le rôle Utilisateur."}
          </p>
          <form onSubmit={submit} className="space-y-3">
            <Field label="Nom d'utilisateur"><Input placeholder="Ex. admin" value={f.username} onChange={(e) => setF({ ...f, username: e.target.value })} autoComplete="username" /></Field>
            <Field label="Nom complet"><Input placeholder="Ex. Marie Dupont" value={f.fullName} onChange={(e) => setF({ ...f, fullName: e.target.value })} /></Field>
            <Field label="Adresse email"><Input type="email" placeholder="contact@exemple.com" value={f.email} onChange={(e) => setF({ ...f, email: e.target.value })} /></Field>
            <Field label="Mot de passe (8 caractères min.)">
              <div className="relative">
                <Input type={show ? "text" : "password"} placeholder="Mot de passe" value={f.password} onChange={(e) => setF({ ...f, password: e.target.value })} className="pr-9" autoComplete="new-password" />
                <button type="button" onClick={() => setShow((v) => !v)} title={show ? "Masquer" : "Afficher"}
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600">
                  {show ? <EyeOff size={15} /> : <Eye size={15} />}
                </button>
              </div>
            </Field>
            <p className="text-[11px] text-slate-400">
              8 caractères minimum, avec au moins une majuscule et un chiffre.
            </p>
            {msg && <p className="text-xs bg-red-50 border border-red-200 text-red-700 rounded-lg px-3 py-2">{msg}</p>}
            <Button className="w-full py-2.5 justify-center" disabled={busy}>
              {busy ? <Loader2 size={16} className="animate-spin" /> : null} Créer mon compte
            </Button>
            <Link to="/login" className="text-xs text-blue-700 block text-center hover:underline">J'ai déjà un compte</Link>
          </form>
        </div>
      </div>
    </div>
  );
}
