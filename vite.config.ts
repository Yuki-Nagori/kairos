import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// Tauri 要求固定开发端口，生产构建产物需落在 dist/。
export default defineConfig({
  plugins: [tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/src-crates/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "safari13",
    minify: "esbuild",
    sourcemap: true,
  },
});
