import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Dev proxy: `npm run dev` against a running daemon (default 127.0.0.1:7676).
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/health": "http://127.0.0.1:7676",
      "/version": "http://127.0.0.1:7676",
    },
  },
});
