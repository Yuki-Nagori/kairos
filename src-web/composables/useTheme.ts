/** 主题的响应式适配层：把 `utils/theme` 的全局主题名接成 ref 并转发切换动作。 */
import { onUnmounted, ref } from "vue";
import { cycleTheme, getActiveTheme, THEME_CHANGED_EVENT } from "../utils/theme";

/** 组合式入口：组件内拿到响应式当前主题名；切换广播经事件监听同步，卸载时清理监听。 */
export function useTheme() {
  const theme = ref(getActiveTheme());
  const sync = (): void => {
    theme.value = getActiveTheme();
  };
  window.addEventListener(THEME_CHANGED_EVENT, sync);
  onUnmounted(() => window.removeEventListener(THEME_CHANGED_EVENT, sync));
  return { theme, cycleTheme };
}
