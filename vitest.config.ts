import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "happy-dom",
    include: ["tests/web/**/*.test.ts"],
    coverage: {
      provider: "v8",
      // 覆盖率口径：纯逻辑与 UI 组件；services 是薄 IPC 封装（逻辑在 Rust core，
      // 由 contract 测试锁定）、render 是浏览器 WebGL 路径，均不计入门槛。
      include: ["src-web/lib/**/*.ts"],
      exclude: [
        "src-web/main.ts",
        "src-web/vite-env.d.ts",
        "src-web/**/*.test.ts",
        "src-web/lib/bench/**",
      ],
      thresholds: { lines: 100, functions: 100, branches: 100, statements: 100 },
    },
  },
});
