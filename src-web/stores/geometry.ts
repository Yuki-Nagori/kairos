/** 几何域状态：已导入的 STL 摘要列表与每个几何的体积网格报告。 */
import { defineStore } from "pinia";
import {
  generateGmshMesh as apiGenerateGmshMesh,
  generateVolumeMesh,
  importSampleBox,
  importStl,
  removeGeometry,
  repairGeometry as apiRepairGeometry,
} from "../api/geometry";
import { pickStlPath } from "../api/dialog";
import type { GeometrySummary, MeshingReport } from "../types";
import { useAppStore } from "./app";

export const useGeometryStore = defineStore("geometry", {
  state: () => ({
    /** 已导入的几何（摘要列表，全量网格在 Rust 会话缓存）。 */
    geometries: [] as GeometrySummary[],
    /** 每个几何的体积网格报告（key = geometryId）。 */
    meshReports: {} as Record<string, MeshingReport>,
  }),
  actions: {
    /** 导入 STL：弹出文件对话框，解析检查后入列表。 */
    async importGeometry(): Promise<void> {
      const app = useAppStore();
      const path = await pickStlPath();
      if (!path) {
        return;
      }
      app.beginBusy("正在导入几何…");
      try {
        const summary = await importStl(path);
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
    /** 为几何生成 3D 体积网格（体素 + 5-四面体保形分解）。 */
    async generateMesh(geometryId: string, targetSize: number): Promise<void> {
      const app = useAppStore();
      if (!(targetSize > 0)) {
        app.setError("目标网格尺寸必须为正数。");
        return;
      }
      app.beginBusy("正在生成网格…");
      try {
        const report = await generateVolumeMesh(geometryId, targetSize);
        this.meshReports = { ...this.meshReports, [geometryId]: report };
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
        const meshReports = { ...this.meshReports };
        delete meshReports[geometryId];
        this.meshReports = meshReports;
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
