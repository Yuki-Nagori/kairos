import { invokeCommand } from "../lib/ipc";
import type { Material } from "../types";

export function listBuiltinMaterials(): Promise<Material[]> {
  return invokeCommand("list_builtin_materials");
}

export function listCustomMaterials(): Promise<Material[]> {
  return invokeCommand("list_custom_materials");
}

/** 从 JSON 文件导入（校验 + 按 id 合并），返回合并后的完整自定义材料库。 */
export function importCustomMaterials(path: string): Promise<Material[]> {
  return invokeCommand("import_custom_materials", { path });
}

export function upsertCustomMaterial(material: Material): Promise<Material[]> {
  return invokeCommand("upsert_custom_material", { material });
}

export function deleteCustomMaterial(id: string): Promise<Material[]> {
  return invokeCommand("delete_custom_material", { id });
}

export function exportMaterialsToFile(path: string, materials: Material[]): Promise<void> {
  return invokeCommand("export_materials_to_file", { path, materials });
}
