/**
 * 虚拟机终端抽屉：浮于工作区右下角（不占布局列），显隐由
 * 状态栏右侧 Shell 按钮与原生菜单驱动（store.vmPanelVisible）。
 */
import { createVmPanel } from "./panels/vm-panel";
import { appStore } from "../state";

export function createVmDock(): HTMLElement {
  const dock = document.createElement("div");
  dock.className = "absolute bottom-2 right-2 z-40 w-[26rem] shadow-2xl shadow-black/50";
  dock.append(createVmPanel());

  const sync = (): void => {
    dock.classList.toggle("hidden", !appStore.get().vmPanelVisible);
  };
  sync();
  appStore.subscribe(sync);
  return dock;
}
