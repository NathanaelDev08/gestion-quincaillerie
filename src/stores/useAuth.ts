import { create } from "zustand";
import type { UserPublic } from "../types";

interface AuthState {
  user: UserPublic | null;
  token: string | null;
  /** Jeton long, révocable : permet de rester connecté sans rouvrir le mot
   *  de passe, et permet une déconnexion réelle côté serveur. */
  refresh: string | null;
  setAuth: (user: UserPublic, token: string, refresh?: string) => void;
  logout: () => void;
}

export const useAuth = create<AuthState>((set) => ({
  user: JSON.parse(localStorage.getItem("user") || "null"),
  token: localStorage.getItem("token"),
  refresh: localStorage.getItem("refresh"),
  setAuth: (user, token, refresh) => {
    localStorage.setItem("user", JSON.stringify(user));
    localStorage.setItem("token", token);
    if (refresh) localStorage.setItem("refresh", refresh);
    set({ user, token, refresh: refresh ?? localStorage.getItem("refresh") });
  },
  logout: () => {
    localStorage.removeItem("user");
    localStorage.removeItem("token");
    localStorage.removeItem("refresh");
    set({ user: null, token: null, refresh: null });
  },
}));
