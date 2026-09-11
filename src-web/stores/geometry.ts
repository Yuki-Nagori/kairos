/** 几何域状态：已导入的 STL 摘要列表与每个几何的体积网格报告。 */
import { defineStore } from "pinia";
import {
  generateDualDomainMesh as apiGenerateDualDomain,
  generateGmshMesh as apiGenerateGmshMesh,
  generateMidplaneMesh as apiGenerateMidplane,
  generateVolumeMesh,
  getRenderMesh,
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
  RenderMeshData,
  RepairReport,
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
    /** 每个几何最近一次修复的报告（key = geometryId）。 */
    repairReports: {} as Record<string, RepairReport>,
  }),
  actions: {
    /** 导入 STL：弹出文件对话框，解析检查后入列表。 */
    async importGeometry(): Promise<void> {
      const app = useAppStore();
      const path = await pickOpenGeometryPath();
      if (!path) {
        return;
      }
      await app.withBusy("正在导入几何…", async () => {
        const extension = path.split(".").pop()?.toLowerCase();
        const summary =
          extension === "step" || extension === "stp"
            ? await apiImportStep(path)
            : extension === "igs" || extension === "iges"
              ? await apiImportIges(path)
              : await importStl(path);
        this.geometries = [...this.geometries, summary];
      });
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
      await app.withBusy("正在生成 Gmsh 网格…", async () => {
        const report = await apiGenerateGmshMesh(geometryId, targetSize);
        this.meshReports = { ...this.meshReports, [geometryId]: report };
      });
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
      await app.withBusy("正在生成网格…", async () => {
        const report = await generateVolumeMesh(geometryId, targetSize, refinement);
        this.meshReports = { ...this.meshReports, [geometryId]: report };
      });
    },
    /** 为几何生成双域网格：表面厚度配对 + 当前方案杆系（流道/浇口）耦合。 */
    async generateDualDomain(geometryId: string): Promise<void> {
      const app = useAppStore();
      const runners = useProjectStore().activeStudy?.runnerElements ?? [];
      await app.withBusy("正在生成双域网格…", async () => {
        const report = await apiGenerateDualDomain(geometryId, runners);
        this.dualDomainReports = { ...this.dualDomainReports, [geometryId]: report };
      });
    },
    /** 为几何生成中面网格：顶点配对法，杆系梁耦合中面节点。 */
    async generateMidplane(geometryId: string): Promise<void> {
      const app = useAppStore();
      const runners = useProjectStore().activeStudy?.runnerElements ?? [];
      await app.withBusy("正在生成中面网格…", async () => {
        const report = await apiGenerateMidplane(geometryId, runners);
        this.midplaneReports = { ...this.midplaneReports, [geometryId]: report };
      });
    },
    /** 修复几何：焊接 / 去退化 / 填孔 / 一致化，刷新摘要并作废体积网格。 */
    async repairGeometryById(geometryId: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在修复几何…", async () => {
        const { summary, report } = await apiRepairGeometry(geometryId);
        this.geometries = this.geometries.map((geometry) =>
          geometry.geometryId === geometryId ? summary : geometry,
        );
        this.repairReports = { ...this.repairReports, [geometryId]: report };
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
      });
    },
    /** 导入内置样例立方体（首次使用引导）。 */
    async importSampleGeometry(size = 10): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在导入样例…", async () => {
        const summary = await importSampleBox(size);
        this.geometries = [...this.geometries, summary];
      });
    },
    /** 导出视口渲染网格（体积边界面优先，否则 STL 表面）；失败进全局错误。 */
    async fetchRenderMesh(geometryId: string): Promise<RenderMeshData | undefined> {
      const app = useAppStore();
      try {
        return await getRenderMesh(geometryId);
      } catch (error) {
        app.setError(error);
      }
    },
  },
});
