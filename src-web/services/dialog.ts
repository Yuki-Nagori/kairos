import { open, save } from "@tauri-apps/plugin-dialog";

const PROJECT_FILTER = { name: "Kairos 工程", extensions: ["kairos"] };

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
