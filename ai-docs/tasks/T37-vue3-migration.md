# T37 · 前端迁移 Vue 3 + Composition API

- 阶段：进行中
- 依赖：T34/T35/T36（现有功能已闭环）
- 优先级：P1

## 目标

前端从原生 TS + DOM 操作迁移到 Vue 3（Composition API + `<script setup>`），
为后续大量新面板/新交互提供更好的可维护性与响应式开发体验。

## 为什么现在迁移

- architecture-status.md 里后续模块很多，原生 DOM 手写会越来越难维护；
- 当前前端 ~3000 行、13 个面板，规模适中——现在迁移成本最低；
- services 层与 kairos-core 零改动——IPC 接口框架无关。

## 技术选型

- **Vue 3.5+** + `<script setup>` Composition API
- **状态**：Vue 原生 `reactive`/`ref`/`computed`（不引 Pinia——分片够用）
- **样式**：Tailwind CSS v4（不变）
- **构建**：Vite + `@vitejs/plugin-vue`（Vite 本身不变）
- **测试**：Vitest + `@vue/test-utils`
- **类型**：全量 TypeScript strict

## 分阶段计划

### Phase 1 · 基础设施（本提交）

- 安装 vue3 / @vitejs/plugin-vue / vue-tsc
- vite.config.ts 加 Vue 插件
- tsconfig.json 加 Vue SFC 支持
- 建立 src-web/components/ Vue 组件目录结构

### Phase 2 · 核心组件（已完成）

- Card.vue（通用卡片：标题/图标/状态行/刷新/折叠）
- Button.vue / TextInput.vue / Dropdown.vue（基础控件）
- ShellIcon.vue（SVG 图标，currentColor）
- StageTabs.vue（分析阶段选项卡）
- LatexBlock.vue（KaTeX 公式渲染）
- LogTabs.vue（日志标签组）
- VmDock.vue（虚拟机终端抽屉）
- typecheck 切换 vue-tsc；vitest 挂 Vue 插件；knip 跟踪 .vue

### Phase 3 · 状态迁移（已完成）

- appStore 状态本体换成 Vue `reactive`：set 仍同步浅合并 + 手动全量通知，
  既有 get/set/subscribe 语义不变——原生组件零改动
- 新增 `useAppState()` 组合式入口；Phase 2 组件改用 computed 直接跟踪
- services 层零改动（IPC 封装框架无关）
- kairos-core 零改动

### Phase 4 · 面板迁移（13 个）

按依赖顺序：ProjectTree → Pipeline → Materials → Geometry → Mold →
Process → Dependencies → Report → Jobs → VmPanel → Results → Viewport → XyChart

### Phase 5 · 清理

- 删除旧 .ts 组件文件
- 删除自定义 store
- 测试全部迁移 @vue/test-utils

## 非目标

- 引入 Pinia / Vue Router（当前单页面 + 分片 store 够用）
- SSR / Nuxt
- 改变 services / core 层

## 验收标准

- `bun run verify` 全绿（含 Vue SFC 类型检查）
- 功能等价：所有面板/动作/快捷键与迁移前一致
- 测试全绿（@vue/test-utils 迁移）
