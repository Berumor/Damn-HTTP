import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The frontend lives in ui/ and is built into dist/, which Tauri embeds
// (see crates/app/tauri.conf.json).
export default defineConfig({
  root: "ui",
  plugins: [react()],
  // Tauri prints its own output; do not clear it.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2022",
  },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    css: true,
  },
});
