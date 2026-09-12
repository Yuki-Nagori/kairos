import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "happy-dom",
    setupFiles: ["tests/setup.ts"],
    include: ["tests/web/**/*.test.ts"],
    coverage: {
      provider: "v8",
      // 覆盖率口径：T38 后逻辑层是 stores + useXxx composables，与 utils 一样
      // 必须 100%；api 是薄 IPC 封装（逻辑在 Rust core，由 contract 测试锁定）、
      // render 是浏览器 WebGL 路径，均不计入门槛。
      include: [
        "src-web/utils/**/*.ts",
        "src-web/stores/**/*.ts",
        "src-web/composables/**/*.ts",
        "src-web/components/**/use*.ts",
        "src-web/views/**/use*.ts",
      ],
      // useViewportPanel 曾在此排除（happy-dom 无 WebGL2）；后端 mock 化测试
      // 落地后已收编（该排除期间 T49 重构曾丢失 renderMesh 赋值而无人察觉）。
      exclude: ["src-web/main.ts", "src-web/**/*.test.ts", "src-web/utils/bench/**"],
      thresholds: { lines: 100, functions: 100, branches: 100, statements: 100 },
    },
  },
});
