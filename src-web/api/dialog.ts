/** 文件对话框：工程 / JSON / STL 的打开与保存路径选择，取消一律返回 null。 */
import { open, save } from "@tauri-apps/plugin-dialog";

const PROJECT_FILTER = { name: "Kairos 工程", extensions: ["kairos"] };
const JSON_FILTER = { name: "JSON", extensions: ["json"] };
const MATERIALS_FILTER = { name: "材料文件", extensions: ["json", "csv"] };

/** 选择要打开的工程文件，取消返回 null。 */
export async function pickOpenProjectPath(): Promise<string | null> {
  const selection = await open({ multiple: false, filters: [PROJECT_FILTER] });
  return typeof selection === "string" ? selection : null;
}

/** 选择保存工程的路径，取消返回 null。 */
export async function pickSaveProjectPath(defaultName: string): Promise<string | null> {
  const path = await save({
    defaultPath: `${defaultName || "未命名项目"}.kairos`,
    filters: [PROJECT_FILTER],
  });
  return path ?? null;
}

/** 选择材料导入文件（JSON / CSV），取消返回 null。 */
export async function pickOpenMaterialsPath(): Promise<string | null> {
  const selection = await open({ multiple: false, filters: [MATERIALS_FILTER] });
  return typeof selection === "string" ? selection : null;
}

/** 选择导出 JSON 的路径，取消返回 null。 */
export async function pickExportJsonPath(defaultName: string): Promise<string | null> {
  const path = await save({
    defaultPath: `${defaultName}.json`,
    filters: [JSON_FILTER],
  });
  return path ?? null;
}

const GEOMETRY_FILTER = { name: "几何模型", extensions: ["stl", "step", "stp", "igs", "iges"] };

/** 选择要导入的几何模型文件（STL / STEP / IGES），取消返回 null。 */
export async function pickOpenGeometryPath(): Promise<string | null> {
  const selection = await open({ multiple: false, filters: [GEOMETRY_FILTER] });
  return typeof selection === "string" ? selection : null;
}
