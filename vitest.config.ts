import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "happy-dom",
    include: ["src-web/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src-web/**/*.ts"],
      exclude: ["src-web/main.ts", "src-web/vite-env.d.ts", "src-web/**/*.test.ts"],
    },
  },
});
