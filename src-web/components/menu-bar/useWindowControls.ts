/**
 * 标题栏自绘窗口控制（Windows/Linux 无边框窗口）：最小化 / 最大化切换 / 关闭。
 * 可用性由外壳适配层给出——不可用时动作整体降级为空操作，调用方只管绑定。
 * 最大化状态经窗口 resize 事件跟踪，供最大化 / 还原图标切换。
 */
import { onScopeDispose, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useShell } from "../../composables/useShell";

export function useWindowControls() {
  const available = useShell().windowControls;
  // 仅在可用（Tauri 运行时）时取窗口句柄，浏览器预览不触达 Tauri API。
  const api = available ? getCurrentWindow() : null;
  const maximized = ref(false);
  let unlisten: (() => void) | null = null;

  function refreshMaximized(): void {
    void api?.isMaximized().then((value) => {
      maximized.value = value;
    });
  }

  if (api !== null) {
    refreshMaximized();
    void api
      .onResized(() => refreshMaximized())
      .then((dispose) => {
        unlisten = dispose;
      });
    // 事件监听随调用方作用域销毁自动退订；无作用域的纯调用（应用生命周期）静默跳过。
    onScopeDispose(() => {
      unlisten?.();
    }, true);
  }

  return {
    available,
    maximized,
    minimize: (): void => void api?.minimize(),
    toggleMaximize: (): void => void api?.toggleMaximize(),
    close: (): void => void api?.close(),
  };
}
