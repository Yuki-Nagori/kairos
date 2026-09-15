/** 几何域状态：已导入的 STL 摘要列表与每个几何的体积网格报告。 */
import { defineStore } from "pinia";
import {
  estimateVolumeMesh as apiEstimateVolumeMesh,
  generateDualDomainMesh as apiGenerateDualDomain,
  generateGmshMesh as apiGenerateGmshMesh,
  generateMidplaneMesh as apiGenerateMidplane,
  generateVolumeMesh,
  getRenderMesh,
  importGeometryFile,
  importSampleBox,
  removeGeometry,
  repairGeometry as apiRepairGeometry,
} from "../api/geometry";
import { pickOpenGeometryPath } from "../api/dialog";
import {
  archiveWorkspaceGeometry,
  loadWorkspaceGeometry,
  restoreStudyMesh,
  saveStudyMesh,
} from "../api/project";
import type {
  DualDomainReport,
  GeometrySummary,
  ImportOutcome,
  MeshEstimate,
  MeshingReport,
  MeshRefinement,
  MidplaneReport,
  RenderMeshData,
  RepairReport,
} from "../types";
import { useAppStore } from "./app";

/** 导入日志环形上限：连续导入多个大件时不无限增长。 */
const IMPORT_LOG_LIMIT = 200;
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
    /** 每个几何当前的网格规模估算（key = geometryId；生成前预览）。 */
    meshEstimates: {} as Record<string, MeshEstimate>,
    /** 几何导入日志（环形缓冲，最近一次导入在最前；日志区展示）。 */
    importLogs: [] as string[],
  }),
  actions: {
    /** 导入 STL：弹出文件对话框，解析检查后入列表；工作区工程顺带归档源文件。 */
    async importGeometry(): Promise<void> {
      const app = useAppStore();
      const path = await pickOpenGeometryPath();
      if (!path) {
        return;
      }
      await app.withBusy("正在导入几何…", async () => {
        // 格式分派在 Rust 侧（Path::extension），前端不解析路径
        const outcome = await importGeometryFile(path);
        this.recordImport(outcome);
        await this.archiveIntoWorkspace(outcome.summary.geometryId, path);
      });
    },
    /** 登记导入结果：几何入列表 + 导入日志进环形缓冲（不阻塞导入主流程）。 */
    recordImport(outcome: ImportOutcome): void {
      this.geometries = [...this.geometries, outcome.summary];
      this.importLogs = [...outcome.log, ...this.importLogs].slice(0, IMPORT_LOG_LIMIT);
    },
    /** 几何归档进工作区（无工作区时为静默 no-op）：失败只记录，不影响导入本身。 */
    async archiveIntoWorkspace(geometryId: string, sourcePath: string): Promise<void> {
      const project = useProjectStore();
      const projectPath = project.projectPath;
      if (projectPath === null || project.workspaceRoot === null) {
        return;
      }
      try {
        const reference = await archiveWorkspaceGeometry(projectPath, geometryId, sourcePath);
        project.upsertGeometryRef(reference);
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 方案网格落盘（无工作区时为 no-op）：打开工程即可直接载入视口。 */
    async persistStudyMesh(
      geometryId: string,
      targetSize: number,
      refinement?: MeshRefinement,
    ): Promise<void> {
      const project = useProjectStore();
      const projectPath = project.projectPath;
      const studyId = project.activeStudyId;
      if (projectPath === null || project.workspaceRoot === null || studyId === null) {
        return;
      }
      try {
        await saveStudyMesh(projectPath, studyId, geometryId, targetSize, refinement);
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 从列表与会话缓存移除几何。 */
    async removeGeometryById(geometryId: string): Promise<void> {
      const app = useAppStore();
      try {
        await removeGeometry(geometryId);
        this.geometries = this.geometries.filter((g) => g.geometryId !== geometryId);
        const estimates = { ...this.meshEstimates };
        delete estimates[geometryId];
        this.meshEstimates = estimates;
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
        await this.persistStudyMesh(geometryId, targetSize);
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
        await this.persistStudyMesh(geometryId, targetSize, refinement);
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
    /** 网格规模估算（生成前预览）：尺寸非法或几何已移除时清掉旧值，不打断输入。 */
    async estimateMesh(
      geometryId: string,
      targetSize: number,
      refinement: MeshRefinement | undefined,
      engine: string,
    ): Promise<void> {
      const clear = (): void => {
        const estimates = { ...this.meshEstimates };
        delete estimates[geometryId];
        this.meshEstimates = estimates;
      };
      if (!(targetSize > 0)) {
        clear();
        return;
      }
      try {
        const estimate = await apiEstimateVolumeMesh(geometryId, targetSize, refinement, engine);
        this.meshEstimates = { ...this.meshEstimates, [geometryId]: estimate };
      } catch {
        clear();
      }
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
        const outcome = await importSampleBox(size);
        this.recordImport(outcome);
      });
    },
    /** 工作区恢复：按工程里的相对路径读回几何，并把当前方案的体积网格读回会话。
     *  打开工作区工程后调用；散装工程（无工作区）直接返回。 */
    async restoreWorkspaceContent(): Promise<void> {
      const app = useAppStore();
      const project = useProjectStore();
      const path = project.projectPath;
      if (path === null || project.project === null || project.workspaceRoot === null) {
        return;
      }
      await app.withBusy("正在恢复工程数据…", async () => {
        for (const reference of project.project!.geometries) {
          const summary = await loadWorkspaceGeometry(path, reference.id, reference.relativePath);
          this.geometries = [
            ...this.geometries.filter((entry) => entry.geometryId !== summary.geometryId),
            summary,
          ];
        }
        for (const study of project.project!.studies.filter(
          (entry) => entry.id === project.activeStudyId,
        )) {
          const restored = await restoreStudyMesh(path, study.id);
          if (restored !== null) {
            this.meshReports = { ...this.meshReports, [restored.geometryId]: restored.report };
          }
        }
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
