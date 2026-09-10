/**
 * 应用标题栏逻辑：主题图标响应式同步与切换。
 * 选项卡与品牌信息是纯模板，不在此管理。
 */
import { useTheme } from "../../composables/useTheme";

export function useAppHeader() {
  // cycleTheme 广播 THEME_CHANGED_EVENT，useTheme 内部监听保持 theme 同步。
  const { theme, cycleTheme } = useTheme();
  return { theme, cycleTheme };
}
