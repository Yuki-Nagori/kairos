/** 底部状态栏逻辑：版本/IPC、忙碌、错误三态（优先级：错误 > 忙碌 > 版本）。 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import { useVmStore } from "../../stores/vm";

export function useStatusBar() {
  const app = useAppStore();
  const vm = useVmStore();

  const status = computed(() => {
    if (app.error) {
      return {
        text: app.error.message,
        class: app.error.info ? "text-zinc-400" : "text-red-400",
      };
    }
    if (app.busy) {
      return { text: app.busy, class: "text-amber-300" };
    }
    if (app.info) {
      return {
        text: `v${app.info.version} · ${app.info.os} · IPC 正常`,
        class: "text-zinc-500",
      };
    }
    return { text: "", class: "" };
  });

  // Shell 环境入口：点击切换右下角虚拟机终端面板的显隐。
  const shellVisible = computed(() => vm.vmPanelVisible);

  return { app, vm, status, shellVisible };
}
