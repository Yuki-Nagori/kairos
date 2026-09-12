/** 材料库状态：内置参考牌号 + 用户自定义材料的导入/导出/复制与研究登记。 */
import { defineStore } from "pinia";
import {
  deleteCustomMaterial,
  exportMaterialsToFile,
  importCustomMaterials,
  listBuiltinMaterials,
  listCustomMaterials,
  upsertCustomMaterial,
} from "../api/materials";
import { pickExportJsonPath, pickOpenMaterialsPath } from "../api/dialog";
/** 自定义材料 id 的会话内序列号：同一毫秒连续复制也不撞 id（与 project store 同策略）。 */
let customMaterialSeq = 0;
import type { Material, MaterialLibrary } from "../types";
import { useAppStore } from "./app";
import { useProjectStore } from "./project";

export const useMaterialsStore = defineStore("materials", {
  state: () => ({
    materials: { builtin: [], custom: [] } as MaterialLibrary,
  }),
  actions: {
    /** 拉取内置与自定义材料（bootstrap 触发；失败上抛由调用方统一处理）。 */
    async loadMaterials(): Promise<void> {
      const [builtin, custom] = await Promise.all([listBuiltinMaterials(), listCustomMaterials()]);
      this.materials = { builtin, custom };
    },
    /** 从 JSON 文件导入自定义材料（importMaterials 的公共尾部）。 */
    async importMaterialsFromPath(path: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在导入材料…", async () => {
        const custom = await importCustomMaterials(path);
        this.materials = { ...this.materials, custom };
      });
    },
    /** 弹出对话框导入材料。 */
    async importMaterials(): Promise<void> {
      const path = await pickOpenMaterialsPath();
      if (path) {
        await this.importMaterialsFromPath(path);
      }
    },
    /** 新增或更新一个自定义材料。 */
    async upsertMaterial(material: Material): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在保存材料…", async () => {
        const custom = await upsertCustomMaterial(material);
        this.materials = { ...this.materials, custom };
      });
    },
    async deleteMaterial(id: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在删除材料…", async () => {
        const custom = await deleteCustomMaterial(id);
        this.materials = { ...this.materials, custom };
      });
    },
    /** 导出全部自定义材料到指定路径（exportMaterials 的公共尾部）。 */
    async exportCustomMaterials(path: string): Promise<void> {
      const app = useAppStore();
      if (this.materials.custom.length === 0) {
        app.setError("没有可导出的自定义材料。");
        return;
      }
      await app.withBusy("正在导出材料…", async () => {
        await exportMaterialsToFile(path, this.materials.custom);
      });
    },
    /** 弹出对话框导出自定义材料。 */
    async exportMaterials(): Promise<void> {
      const path = await pickExportJsonPath("kairos-custom-materials");
      if (path) {
        await this.exportCustomMaterials(path);
      }
    },
    /** 复制任一材料为自定义材料（新 id + 「副本」后缀）。 */
    async copyMaterialToCustom(id: string): Promise<void> {
      const app = useAppStore();
      const source = [...this.materials.builtin, ...this.materials.custom].find((m) => m.id === id);
      if (!source) {
        app.setError("未找到要复制的材料。");
        return;
      }
      const copy: Material = {
        ...source,
        id: `custom-${Date.now()}-${++customMaterialSeq}`,
        name: `${source.name}-副本`,
        dataNote: source.dataNote.startsWith("自定义")
          ? source.dataNote
          : `自定义副本。${source.dataNote}`,
      };
      await this.upsertMaterial(copy);
    },
    /** 把材料登记到活跃研究（活跃研究归属 project store）。 */
    assignMaterial(materialId: string): void {
      const app = useAppStore();
      const projectStore = useProjectStore();
      const project = projectStore.project;
      const activeStudyId = projectStore.activeStudyId;
      if (!project || !activeStudyId) {
        app.setError("请先创建或选择一个研究。");
        return;
      }
      const exists = [...this.materials.builtin, ...this.materials.custom].some(
        (m) => m.id === materialId,
      );
      if (!exists) {
        app.setError("材料不存在。");
        return;
      }
      const studies = project.studies.map((study) =>
        study.id === activeStudyId ? { ...study, materialId } : study,
      );
      projectStore.project = { ...project, studies, updatedMs: Date.now() };
    },
  },
});
