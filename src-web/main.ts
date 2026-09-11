/** 应用入口：安装 Pinia、挂载根组件，拉起菜单 / 快捷键与项目 bootstrap。 */
import "./app.css";
import { createPinia } from "pinia";
import { createApp } from "vue";
import App from "./App.vue";
import { setupMenuActions } from "./menu-actions";
import { setupGlobalShortcuts } from "./global-shortcuts";
import { activateWindowDecoration } from "./window-decoration";
import { initTheme } from "./composables/useTheme";
import { useProjectStore } from "./stores/project";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Root element #app not found");
}

// 先恢复主题再挂载，标题栏的初始主题图标才能与持久化偏好一致。
initTheme();

// Pinia 先于挂载安装：菜单/快捷键等非组件上下文也依赖它拿 store 实例。
const app = createApp(App);
app.use(createPinia());
app.mount(root);

setupGlobalShortcuts();
setupMenuActions();

// bootstrap 内部已自行处理失败（setError），无需 await。
void useProjectStore().bootstrap();

// 前端就绪后激活窗口装饰并显示窗口（失败回退原生标题栏；浏览器预览跳过）
activateWindowDecoration();
