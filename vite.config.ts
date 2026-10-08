import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/reference/vite-plugin/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        // Découpage manuel : sans cela tout est envoyé dans un seul bloc de
        // ~890 Ko, trop lourd pour un poste de boutique avec disque lent.
        // Recharts et la caisse ne sont utiles qu'une fois la page ouverte.
        manualChunks: {
          react: ["react", "react-dom", "react-router-dom"],
          query: ["@tanstack/react-query", "zustand"],
          charts: ["recharts"],
          utils: ["date-fns", "jsbarcode", "qrcode.react"],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/utils/**/*.ts", "src/services/**/*.ts"],
    },
  },
});
