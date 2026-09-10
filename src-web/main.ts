import "./app.css";
import { createApp } from "vue";
import App from "./App.vue";
import { setupMenuActions } from "./menu-actions";
import { setupGlobalShortcuts } from "./shortcuts";
import { initTheme } from "./theme";
import { bootstrap } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Root element #app not found");
}

// 先恢复主题再挂载，标题栏的初始主题图标才能与持久化偏好一致。
initTheme();

createApp(App).mount(root);

setupGlobalShortcuts();
setupMenuActions();

// bootstrap 内部已自行处理失败（setError），无需 await。
void bootstrap();
