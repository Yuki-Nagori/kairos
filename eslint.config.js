import eslint from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier";
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";

export default tseslint.config(
  {
    ignores: ["dist/**", "coverage/**", "src-tauri/**", "target/**", "node_modules/**"],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.ts"],
    rules: {
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
    },
  },
  // SFC：<script setup lang="ts"> 经 vue 解析器接管，脚本块内复用同一套 TS 规则。
  // no-undef 必须关闭——它读不懂 TS 类型，真值检查由 vue-tsc 全量负责（与 .ts 同理）。
  ...pluginVue.configs["flat/recommended"].map((config) => ({
    ...config,
    files: ["**/*.vue"],
    rules: {
      ...config.rules,
      "no-undef": "off",
      // 纯排版类模板规则交给 prettier 统一管辖，避免两套工具互相打架。
      "vue/max-attributes-per-line": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/html-self-closing": "off",
      "vue/attributes-order": "off",
    },
  })),
  {
    files: ["**/*.vue"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".vue"],
      },
    },
    rules: {
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
    },
  },
  {
    // 根组件名单词命名是约定俗成，不适用多词组件名规则。
    files: ["src-web/App.vue"],
    rules: {
      "vue/multi-word-component-names": "off",
    },
  },
  // 必须放在最后：让 prettier 的冲突规则关停覆盖上面所有配置（含 Vue 模板排版规则）。
  eslintConfigPrettier,
);
