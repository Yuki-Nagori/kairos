/** 几何域状态：已导入的 STL 摘要列表与每个几何的体积网格报告。 */
import { defineStore } from "pinia";
import {
  generateDualDomainMesh as apiGenerateDualDomain,
  generateGmshMesh as apiGenerateGmshMesh,
  generateMidplaneMesh as apiGenerateMidplane,
  generateVolumeMesh,
  importIges as apiImportIges,
  importSampleBox,
  importStl,
  importStep as apiImportStep,
  removeGeometry,
  repairGeometry as apiRepairGeometry,
} from "../api/geometry";
import { pickOpenGeometryPath } from "../api/dialog";
import type {
  DualDomainReport,
  GeometrySummary,
  MeshingReport,
  MeshRefinement,
  MidplaneReport,
} from "../types";
import { useAppStore } from "./app";
import { useProjectStore } from "./project";

export const useGeometryStore = defineStore("geometry", {
  state: () => ({
    /** 已导入的几何（摘要列表，全量网格在 Rust 会话缓存）。 */
    geometries: [] as GeometrySummary[],
    /** 每个几何的体积网格报告（key = geometryId）。 */
    meshReports: {} as Record<string, MeshingReport>,
    /** 每个几何的双域网格报告（key = geometryId）。 */
    dualDomainReports: {} as Record<string, DualDomainReport>,
    /** 每个几何的中面网格报告（key = geometryId）。 */
    midplaneReports: {} as Record<string, MidplaneReport>,
  }),
  actions: {
    /** 导入 STL：弹出文件对话框，解析检查后入列表。 */
    async importGeometry(): Promise<void> {
      const app = useAppStore();
      const path = await pickOpenGeometryPath();
      if (!path) {
        return;
      }
      app.beginBusy("正在导入几何…");
      try {
        const extension = path.split(".").pop()?.toLowerCase();
        const summary =
          extension === "step" || extension === "stp"
            ? await apiImportStep(path)
            : extension === "igs" || extension === "iges"
              ? await apiImportIges(path)
              : await importStl(path);
        this.geometries = [...this.geometries, summary];
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 从列表与会话缓存移除几何。 */
    async removeGeometryById(geometryId: string): Promise<void> {
      const app = useAppStore();
      try {
        await removeGeometry(geometryId);
        this.geometries = this.geometries.filter((g) => g.geometryId !== geometryId);
      } catch (error) {
        app.setError(error);
      }
    },
    /** 为几何生成 Gmsh 引擎 3D 体积网格（薄壁/曲面件；需依赖面板已下载 Gmsh）。 */
    async generateGmshMesh(geometryId: string, targetSize: number): Promise<void> {
      const app = useAppStore();
      if (!(targetSize > 0)) {
        app.setError("目标网格尺寸必须为正数。");
        return;
      }
      app.beginBusy("正在生成 Gmsh 网格…");
      try {
        const report = await apiGenerateGmshMesh(geometryId, targetSize);
        this.meshReports = { ...this.meshReports, [geometryId]: report };
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 为几何生成 3D 体积网格（体素 + 5-四面体保形分解），可选分级加密。 */
    async generateMesh(
      geometryId: string,
      targetSize: number,
      refinement?: MeshRefinement,
    ): Promise<void> {
      const app = useAppStore();
      if (!(targetSize > 0)) {
        app.setError("目标网格尺寸必须为正数。");
        return;
      }
      app.beginBusy("正在生成网格…");
      try {
        const report = await generateVolumeMesh(geometryId, targetSize, refinement);
        this.meshReports = { ...this.meshReports, [geometryId]: report };
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 为几何生成双域网格：表面厚度配对 + 当前方案杆系（流道/浇口）耦合。 */
    async generateDualDomain(geometryId: string): Promise<void> {
      const app = useAppStore();
      const runners = useProjectStore().activeStudy?.runnerElements ?? [];
      app.beginBusy("正在生成双域网格…");
      try {
        const report = await apiGenerateDualDomain(geometryId, runners);
        this.dualDomainReports = { ...this.dualDomainReports, [geometryId]: report };
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 为几何生成中面网格：顶点配对法，杆系梁耦合中面节点。 */
    async generateMidplane(geometryId: string): Promise<void> {
      const app = useAppStore();
      const runners = useProjectStore().activeStudy?.runnerElements ?? [];
      app.beginBusy("正在生成中面网格…");
      try {
        const report = await apiGenerateMidplane(geometryId, runners);
        this.midplaneReports = { ...this.midplaneReports, [geometryId]: report };
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 修复几何：焊接 / 去退化 / 填孔 / 一致化，刷新摘要并作废体积网格。 */
    async repairGeometryById(geometryId: string): Promise<void> {
      const app = useAppStore();
      app.beginBusy("正在修复几何…");
      try {
        const summary = await apiRepairGeometry(geometryId);
        this.geometries = this.geometries.map((geometry) =>
          geometry.geometryId === geometryId ? summary : geometry,
        );
        // 修复改变了表面网格：体积与双域网格一并作废。
        const meshReports = { ...this.meshReports };
        delete meshReports[geometryId];
        this.meshReports = meshReports;
        const dualDomainReports = { ...this.dualDomainReports };
        delete dualDomainReports[geometryId];
        this.dualDomainReports = dualDomainReports;
        const midplaneReports = { ...this.midplaneReports };
        delete midplaneReports[geometryId];
        this.midplaneReports = midplaneReports;
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 导入内置样例立方体（首次使用引导）。 */
    async importSampleGeometry(size = 10): Promise<void> {
      const app = useAppStore();
      app.beginBusy("正在导入样例…");
      try {
        const summary = await importSampleBox(size);
        this.geometries = [...this.geometries, summary];
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
  },
});
