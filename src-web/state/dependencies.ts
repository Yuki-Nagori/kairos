import {
  listRuntimeDependencies,
  openDependencyPage as apiOpenDependencyPage,
} from "../services/dependencies";
import { downloadComponentFile } from "../services/downloads";
import { appStore, setError } from "./store";

/** 刷新运行时依赖就绪状态（探测 blockMesh / openInjMoldSim / gmsh）。 */
export async function refreshDependencies(): Promise<void> {
  try {
    const dependencies = await listRuntimeDependencies();
    appStore.set({ dependencies });
  } catch (error) {
    setError(error);
  }
}

/** 打开组件官方页（下载 / 编译指引）。 */
export async function openDependencyPageAction(pageUrl: string): Promise<void> {
  try {
    await apiOpenDependencyPage(pageUrl);
  } catch (error) {
    setError(error);
  }
}

/** 应用内下载：把官方单文件直链取回受管目录；进度经 downloadProgress 反馈到面板。 */
export async function downloadComponent(componentId: string, url: string): Promise<void> {
  appStore.set({ busy: "正在下载…", error: null });
  try {
    const saved = await downloadComponentFile(url, (percent) => {
      appStore.set({
        downloadProgress: { ...appStore.get().downloadProgress, [componentId]: percent },
      });
    });
    appStore.set({
      savedDownloads: { ...appStore.get().savedDownloads, [componentId]: saved },
    });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}
