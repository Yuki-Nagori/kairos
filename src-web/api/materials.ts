/** 材料库 IPC：内置 / 自定义材料查询与自定义材料的增删改、导入导出。 */
import { invokeCommand } from "../utils/ipc";
import type { Material } from "../types";

/** 内置示例材料清单。 */
export function listBuiltinMaterials(): Promise<Material[]> {
  return invokeCommand("list_builtin_materials");
}

/** 用户自定义材料清单。 */
export function listCustomMaterials(): Promise<Material[]> {
  return invokeCommand("list_custom_materials");
}

/** 从 JSON 文件导入（校验 + 按 id 合并），返回合并后的完整自定义材料库。 */
export function importCustomMaterials(path: string): Promise<Material[]> {
  return invokeCommand("import_custom_materials", { path });
}

/** 新增或更新一个自定义材料，返回更新后的完整清单。 */
export function upsertCustomMaterial(material: Material): Promise<Material[]> {
  return invokeCommand("upsert_custom_material", { material });
}

/** 删除自定义材料，返回删除后的完整清单。 */
export function deleteCustomMaterial(id: string): Promise<Material[]> {
  return invokeCommand("delete_custom_material", { id });
}

/** 导出自定义材料到 JSON 文件。 */
export function exportMaterialsToFile(path: string, materials: Material[]): Promise<void> {
  return invokeCommand("export_materials_to_file", { path, materials });
}
