/** 底部状态栏逻辑：三段式——左侧三态状态、求解进度、右侧版本与 Shell 入口。 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import { useJobsStore } from "../../stores/jobs";
import { useVmStore } from "../../stores/vm";

/** 错误码 → 处置提示。前端按 code 分支的契约消费点（KairosError::as_code），
 *  禁止对 message 做文本匹配。 */
const CODE_HINTS: Record<string, string> = {
  validation: "请修正参数后重试",
  not_found: "请先完成前置步骤",
  io: "检查依赖与路径后可重试",
  solver: "查看作业日志或依赖面板",
  internal: "详情见诊断页",
};

export function useStatusBar() {
  const app = useAppStore();
  const jobsStore = useJobsStore();
  const vm = useVmStore();

  /** 左侧三态（优先级：错误 > 忙碌 > IPC 正常）。错误态附 code 徽章与处置提示。 */
  const status = computed(() => {
    if (app.error) {
      const { code } = app.error;
      return {
        text: app.error.message,
        code,
        hint: code !== null ? (CODE_HINTS[code] ?? null) : null,
        class: app.error.info ? "text-zinc-400" : "text-red-400",
      };
    }
    if (app.busy) {
      return { text: app.busy, code: null, hint: null, class: "text-amber-300" };
    }
    return { text: "● IPC 正常", code: null, hint: null, class: "text-emerald-400" };
  });

  /** 中部求解进度：运行中作业的物理时间（Rust 侧解析日志得到），空闲为空。 */
  const solveProgress = computed(() => {
    const job = jobsStore.jobs.at(-1);
    if (job?.status === "running" && job.lastTimeS !== null) {
      return `求解进度 · Time = ${job.lastTimeS.toFixed(2)} s`;
    }
    return "";
  });

  /** 右侧版本与平台（bootstrap 拉取 system info 之前为空）。 */
  const versionText = computed(() => (app.info ? `v${app.info.version} · ${app.info.os}` : ""));

  const shellVisible = computed(() => vm.vmPanelVisible);

  return { app, vm, status, solveProgress, versionText, shellVisible };
}
