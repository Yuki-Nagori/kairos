/**
 * 关于对话框：模块级单例开合态——原生菜单（macOS 应用菜单走系统面板不经此）、
 * 自绘标题栏菜单与命令面板都指向同一份状态。
 */
import { ref } from "vue";

const aboutOpen = ref(false);

export function useAboutDialog() {
  function showAbout(): void {
    aboutOpen.value = true;
  }

  function hideAbout(): void {
    aboutOpen.value = false;
  }

  return { aboutOpen, showAbout, hideAbout };
}
