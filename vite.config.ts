import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig, type PluginOption } from "vite";
import { visualizer } from "rollup-plugin-visualizer";

// Tauri 要求固定开发端口，生产构建产物需落在 dist/。
export default defineConfig({
  plugins: [
    tailwindcss(),
    vue(),
    // 体积分析按需开启（ANALYZE=1 bun run build），报告写入 dist/stats.html。
    process.env.ANALYZE &&
      visualizer({ filename: "dist/stats.html", gzipSize: true, brotliSize: true }),
  ] as PluginOption[],
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
    // 发布产物不随包分发 sourcemap：安装包体积优先；排障用 dev 模式复现。
    sourcemap: false,
    rollupOptions: {
      output: {
        // 将稳定的大型运行时拆出主入口，避免报告公式和图表依赖拖大首屏 chunk。
        manualChunks(id) {
          if (id.includes("node_modules/katex")) {
            return "katex";
          }
          if (id.includes("node_modules/uplot")) {
            return "uplot";
          }
          if (id.includes("node_modules/vue") || id.includes("node_modules/@vue/")) {
            return "vue-vendor";
          }
          return undefined;
        },
      },
    },
  },
});
