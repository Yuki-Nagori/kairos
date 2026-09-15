/**
 * 材料库查询：内置与自定义合一后的按 id 查找。
 *
 * 「材料在库里按 id 找」这件事散在 store / pipeline / 多个面板里，
 * 展开方式（builtin 在前、custom 在后，同 id 时以自定义为准）必须一致——
 * 收在这里，库结构调整时不会漏改某一处。
 */
import type { Material, MaterialLibrary } from "../types";

/** 按 id 查找材料（内置 + 自定义）；未命中返回 null。 */
export function findMaterial(library: MaterialLibrary, id: string | null): Material | null {
  if (id === null) {
    return null;
  }
  return [...library.builtin, ...library.custom].find((material) => material.id === id) ?? null;
}
