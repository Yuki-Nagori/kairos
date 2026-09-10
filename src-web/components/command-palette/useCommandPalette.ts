/**
 * 命令面板（⌘K）：搜索并执行命令注册表中的全部命令（阶段切换 + 菜单命令）。
 * 开合/过滤/高亮为模块级单例状态——菜单栏搜索框与悬浮层共享同一份，
 * 全局快捷键也只注册一次（应用生命周期，无需随组件卸载）。
 */
import { computed, ref } from "vue";
import { isCommandPaletteShortcut } from "../../utils/environment";
import type { PaletteItem } from "../menu-bar/useCommandRegistry";
import { useCommandRegistry } from "../menu-bar/useCommandRegistry";

const open = ref(false);
const query = ref("");
const activeIndex = ref(0);
let keysInstalled = false;

export function useCommandPalette() {
  const { allCommands } = useCommandRegistry();

  /** 关键词匹配「分组 + 标签」；空关键词返回全量。 */
  const filtered = computed<PaletteItem[]>(() => {
    const keyword = query.value.trim().toLowerCase();
    if (keyword === "") {
      return allCommands;
    }
    return allCommands.filter((command) =>
      `${command.group} ${command.label}`.toLowerCase().includes(keyword),
    );
  });

  function openPalette(): void {
    query.value = "";
    activeIndex.value = 0;
    open.value = true;
  }

  function closePalette(): void {
    open.value = false;
  }

  function togglePalette(): void {
    if (open.value) {
      closePalette();
    } else {
      openPalette();
    }
  }

  /** 移动高亮项并夹在有效范围内；无候选时不动。 */
  function moveActive(delta: number): void {
    const count = filtered.value.length;
    if (count === 0) {
      return;
    }
    const next = activeIndex.value + delta;
    activeIndex.value = Math.min(Math.max(next, 0), count - 1);
  }

  function closeAndRun(run: () => void): void {
    closePalette();
    run();
  }

  function runActive(): void {
    const command = filtered.value[activeIndex.value];
    if (command === undefined) {
      return;
    }
    closeAndRun(command.run);
  }

  function onGlobalKeydown(event: KeyboardEvent): void {
    if (isCommandPaletteShortcut(event)) {
      event.preventDefault();
      togglePalette();
      return;
    }
    if (!open.value) {
      return;
    }
    if (event.key === "Escape") {
      closePalette();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      moveActive(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      moveActive(-1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      runActive();
    }
  }

  if (!keysInstalled) {
    keysInstalled = true;
    window.addEventListener("keydown", onGlobalKeydown);
  }

  return {
    open,
    query,
    activeIndex,
    filtered,
    openPalette,
    closePalette,
    togglePalette,
    moveActive,
    runActive,
    runCommand: closeAndRun,
  };
}
