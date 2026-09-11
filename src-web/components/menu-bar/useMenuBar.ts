/**
 * 标题栏菜单（Windows/Linux 自绘）的开合状态：一级菜单下标 + 外部点击/Esc 关闭。
 * 命令数据来自 useCommandRegistry；macOS 不渲染此菜单（系统菜单栏承载）。
 */
import { onMounted, onUnmounted, ref } from "vue";
import type { MenuCommand } from "./useCommandRegistry";
import { useCommandRegistry } from "./useCommandRegistry";

export function useMenuBar() {
  const { menuGroups } = useCommandRegistry();

  // 当前展开的一级菜单下标；null 表示全部收起。
  const openIndex = ref<number | null>(null);

  function toggleMenu(index: number): void {
    openIndex.value = openIndex.value === index ? null : index;
  }

  /** 菜单展开期间悬停其他一级项直接切换（标准菜单栏行为），未展开时不响应。 */
  function hoverMenu(index: number): void {
    if (openIndex.value !== null) {
      openIndex.value = index;
    }
  }

  function closeMenus(): void {
    openIndex.value = null;
  }

  /** 执行命令并收起菜单：动作可能弹出对话框/改变布局，菜单不应残留。 */
  function runCommand(command: MenuCommand): void {
    closeMenus();
    command.run();
  }

  function onPointerDown(event: PointerEvent): void {
    if (openIndex.value === null) {
      return;
    }
    const target = event.target;
    if (target instanceof Element && target.closest("[data-menu-root]") === null) {
      closeMenus();
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      closeMenus();
    }
  }

  onMounted(() => {
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
  });
  onUnmounted(() => {
    window.removeEventListener("pointerdown", onPointerDown);
    window.removeEventListener("keydown", onKeyDown);
  });

  return { menuGroups, openIndex, toggleMenu, hoverMenu, closeMenus, runCommand };
}
