import {
  generateVolumeMesh,
  importSampleBox,
  importStl,
  removeGeometry,
} from "../services/geometry";
import { pickStlPath } from "../services/dialog";
import { appStore, setError } from "./store";

/** 导入 STL：弹出文件对话框，解析检查后入列表。 */
export async function importGeometry(): Promise<void> {
  const path = await pickStlPath();
  if (!path) {
    return;
  }
  appStore.set({ busy: "正在导入几何…", error: null });
  try {
    const summary = await importStl(path);
    appStore.set({ geometries: [...appStore.get().geometries, summary] });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 从列表与会话缓存移除几何。 */
export async function removeGeometryById(geometryId: string): Promise<void> {
  try {
    await removeGeometry(geometryId);
    appStore.set({
      geometries: appStore.get().geometries.filter((g) => g.geometryId !== geometryId),
    });
  } catch (error) {
    setError(error);
  }
}

/** 为几何生成 3D 体积网格（体素 + 5-四面体保形分解）。 */
export async function generateMesh(geometryId: string, targetSize: number): Promise<void> {
  if (!(targetSize > 0)) {
    setError("目标网格尺寸必须为正数。");
    return;
  }
  appStore.set({ busy: "正在生成网格…", error: null });
  try {
    const report = await generateVolumeMesh(geometryId, targetSize);
    appStore.set({
      meshReports: { ...appStore.get().meshReports, [geometryId]: report },
    });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 导入内置样例立方体（首次使用引导）。 */
export async function importSampleGeometry(size = 10): Promise<void> {
  appStore.set({ busy: "正在导入样例…", error: null });
  try {
    const summary = await importSampleBox(size);
    appStore.set({ geometries: [...appStore.get().geometries, summary] });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}
