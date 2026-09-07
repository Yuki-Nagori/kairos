/** 设计 tokens：统一色彩、字号、间距与面板样式（T16 设计系统）。 */

export const tokens = {
  /** 面板容器 */
  panel: "rounded-2xl border border-zinc-800 bg-zinc-900 shadow-xl",
  panelHeader:
    "border-b border-zinc-800 px-5 py-3 text-sm font-semibold text-zinc-100 cursor-pointer select-none",
  panelHeaderCollapsed: "text-zinc-500",
  panelBody: "space-y-3 px-5 py-4",
  /** 主色（品牌强调） */
  accentText: "text-emerald-400",
  accentBorder: "border-emerald-500",
  accentBg: "bg-emerald-500/10",
  /** 状态色 */
  statusRunning: "text-amber-300",
  statusOk: "text-emerald-400",
  statusError: "text-red-400",
  statusMuted: "text-zinc-500",
  /** 表单控件 */
  input:
    "rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-emerald-500 focus:outline-none",
  /** 布局 */
  layoutGap: "gap-3",
} as const;

/** 折叠状态持久化键前缀。 */
export const PANEL_STATE_PREFIX = "kairos-panel:";
