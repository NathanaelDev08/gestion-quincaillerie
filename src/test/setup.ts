import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

// jsdom n'implémente pas matchMedia, utilisé par recharts et la CSP.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// jsdom ne fournit pas localStorage de façon fiable entre les tests.
class MemoireLocal implements Storage {
  private donnees = new Map<string, string>();
  get length() { return this.donnees.size; }
  clear() { this.donnees.clear(); }
  getItem(k: string) { return this.donnees.has(k) ? this.donnees.get(k)! : null; }
  key(i: number) { return Array.from(this.donnees.keys())[i] ?? null; }
  removeItem(k: string) { this.donnees.delete(k); }
  setItem(k: string, v: string) { this.donnees.set(k, String(v)); }
}

Object.defineProperty(window, "localStorage", { value: new MemoireLocal(), writable: true });

// L'environnement Tauri est simulé : sans cela les appels API rejetteraient
// tous les tests avec « Tauri indisponible ».
Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: vi.fn() }, writable: true });

afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.restoreAllMocks();
});
