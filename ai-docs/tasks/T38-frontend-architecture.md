# T38 · 前端架构重构：views / composables / Pinia 分层

- 阶段：进行中
- 依赖：T37（Vue 3 迁移完成）
- 优先级：P1

## 目标

对齐标准 Vue 3 工程分层：页面级单元只写 UI，逻辑全部抽到 `useXxx.ts`；
状态层升级 Pinia；目录命名与通用约定完全对齐。

## 目标结构

```
src-web/
├── views/<module>/      # 页面级面板：XxxPanel.vue（只写 UI）+ useXxxPanel.ts（逻辑）
├── components/          # 可复用 UI（ui/ 基础件 + AppHeader/StatusBar 等）
├── composables/         # 全局可复用逻辑（useTheme 等）
├── stores/              # Pinia：按领域 defineStore（app/project/geometry/materials/jobs/results/dependencies/vm）
├── api/                 # 原 services/：Tauri IPC 封装（薄适配层）
├── utils/               # 原 lib/：chart/stats/ipc/environment/report + bench
└── types/               # 原 types.ts：index.ts 统一出口
```

## 分阶段实施（一个阶段一个 commit）

### Phase A · 目录改名对齐（已完成）

- services → api、lib → utils、types.ts → types/index.ts
- components/panels/ → views/<module>/（13 个面板各归模块目录）
- vitest 覆盖率 include、knip 入口、bench 脚本、AGENTS.md 分层契约同步
- 本阶段纯移动：.vue / .ts 内容不动，行为零变化

### Phase B · 状态层 Pinia 化

- state/ 八个分片重写为 stores/ 下 defineStore（state + getters + actions）
- setError 收敛到 app store；跨域动作经 store 间引用
- 消费端（views / components / menu-actions / shortcuts）与测试同步迁移

### Phase C · composables 抽取

- 13 个 views 各抽 useXxx.ts：状态消费、computed、事件处理、命令式集成
- .vue 保留模板 + 一行 setup 调用
- 共享组件薄逻辑就近抽 useXxx；全局可复用项进 composables/

## 非目标

- 引入 Vue Router（Tauri 单窗口工作台，阶段显隐已由 App 编排）
- 渲染层 render/ 不动（WebGL 与分层无关）

## 验收标准

- `bun run verify` 全绿（typecheck / eslint --max-warnings 0 / 覆盖率 / knip）
- 功能等价：所有面板、动作、快捷键与重构前一致

### Phase B · 状态层 Pinia 化（已完成）

- stores/ 八个 defineStore：app(info/stage/busy/error) / project(含 activeStudy
  getter) / geometry / materials / jobs / pipeline / results / dependencies
- busy 前后置收敛为 app store 的 beginBusy/endBusy；错误统一 app.setError
- 跨域动作经 store 间引用（bootstrap 扇出 materials 装载、submitPipeline
  读四个 store、assignMaterial 回写 project）
- menu-actions / shortcuts 在处理器内懒取 store（模块加载早于 Pinia 安装）
- 消费端 21 文件、测试 9 文件同步迁移；state/ 删除；96 测试全绿
