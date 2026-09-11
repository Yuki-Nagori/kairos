/**
 * 窗口装饰激活（tauri-plugin-decoration）：前端就绪后调用 activate_and_show，
 * 插件按平台运行时管理装饰——Windows/Linux 切无边框并注入内嵌控制按钮
 * （Windows 11 含原生 Snap Layout 热区），macOS 保留红绿灯叠加在内容上。
 * 激活失败（含 Linux X11 等不支持场景）时插件回退并显示原生标题栏；
 * 浏览器预览无 Tauri 运行时，静默跳过。
 */
import { invoke } from "@tauri-apps/api/core";
import { currentPlatform, isTauriRuntime } from "./utils/environment";

export function activateWindowDecoration(): void {
  // macOS 不激活：插件激活路径的 set_decorations(true) 会抹掉配置生成的
  // Overlay 标题栏（退回原生带边框窗口），macOS 直接用配置生效的红绿灯叠加。
  if (!isTauriRuntime() || currentPlatform() === "macos") {
    return;
  }
  void invoke<"custom" | "native">("activate_and_show").catch(() => undefined);
}
