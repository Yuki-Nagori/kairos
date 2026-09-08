import {
  deleteCustomMaterial,
  exportMaterialsToFile,
  importCustomMaterials,
  listBuiltinMaterials,
  listCustomMaterials,
  upsertCustomMaterial,
} from "../services/materials";
import { pickExportJsonPath, pickOpenJsonPath } from "../services/dialog";
import type { Material } from "../types";
import { appStore, setError } from "./store";

export async function loadMaterials(): Promise<void> {
  const [builtin, custom] = await Promise.all([listBuiltinMaterials(), listCustomMaterials()]);
  appStore.set({ materials: { builtin, custom } });
}

/** 从 JSON 文件导入自定义材料。 */
async function importMaterialsFromPath(path: string): Promise<void> {
  appStore.set({ busy: "正在导入材料…", error: null });
  try {
    const custom = await importCustomMaterials(path);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 弹出对话框导入材料。 */
export async function importMaterials(): Promise<void> {
  const path = await pickOpenJsonPath();
  if (path) {
    await importMaterialsFromPath(path);
  }
}

/** 新增或更新一个自定义材料。 */
async function upsertMaterial(material: Material): Promise<void> {
  appStore.set({ busy: "正在保存材料…", error: null });
  try {
    const custom = await upsertCustomMaterial(material);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

export async function deleteMaterial(id: string): Promise<void> {
  appStore.set({ busy: "正在删除材料…", error: null });
  try {
    const custom = await deleteCustomMaterial(id);
    appStore.set({ materials: { ...appStore.get().materials, custom } });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 导出全部自定义材料到指定路径。 */
async function exportCustomMaterials(path: string): Promise<void> {
  const { materials } = appStore.get();
  if (materials.custom.length === 0) {
    setError("没有可导出的自定义材料。");
    return;
  }
  appStore.set({ busy: "正在导出材料…", error: null });
  try {
    await exportMaterialsToFile(path, materials.custom);
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 弹出对话框导出自定义材料。 */
export async function exportMaterials(): Promise<void> {
  const path = await pickExportJsonPath("kairos-custom-materials");
  if (path) {
    await exportCustomMaterials(path);
  }
}

/** 复制任一材料为自定义材料（新 id + 「副本」后缀）。 */
export async function copyMaterialToCustom(id: string): Promise<void> {
  const { materials } = appStore.get();
  const source = [...materials.builtin, ...materials.custom].find((m) => m.id === id);
  if (!source) {
    setError("未找到要复制的材料。");
    return;
  }
  const copy: Material = {
    ...source,
    id: `custom-${Date.now()}`,
    name: `${source.name}-副本`,
    dataNote: source.dataNote.startsWith("自定义")
      ? source.dataNote
      : `自定义副本。${source.dataNote}`,
  };
  await upsertMaterial(copy);
}

/** 把材料登记到活跃研究。 */
export function assignMaterial(materialId: string): void {
  const { project, activeStudyId, materials } = appStore.get();
  if (!project || !activeStudyId) {
    setError("请先创建或选择一个研究。");
    return;
  }
  const exists = [...materials.builtin, ...materials.custom].some((m) => m.id === materialId);
  if (!exists) {
    setError("材料不存在。");
    return;
  }
  const studies = project.studies.map((study) =>
    study.id === activeStudyId ? { ...study, materialId } : study,
  );
  appStore.set({ project: { ...project, studies, updatedMs: Date.now() } });
}
