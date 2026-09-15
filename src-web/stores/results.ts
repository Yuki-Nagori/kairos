/** 求解结果状态：结果目录清单、最近加载的场与探针列表。 */
import { defineStore } from "pinia";
import { analyzeGateLocation, previewFill } from "../api/geometry";
import {
  deriveDifference as deriveDifferenceApi,
  deriveField as deriveFieldApi,
  deformRenderMesh,
  listResultTimes,
  sampleProbeSeries,
  loadResultField,
  loadTensorField,
  loadVectorField,
  exportResultFieldCsv,
  summarizeResultField,
  summarizeVectorField,
} from "../api/results";
import type {
  DeriveRequest,
  FieldSlot,
  FillPreviewReport,
  GateLocationReport,
  Probe,
  RenderMeshData,
  TensorField,
  VectorField,
  ProbeTimeSeries,
  ResultCatalog,
  ScalarField,
  FieldStats,
} from "../types";
import { toCsv } from "../utils/chart";
import { downloadTextFile } from "../utils/download";
import { useAppStore } from "./app";
import { useProjectStore } from "./project";

let probeSeq = 0;

export const useResultsStore = defineStore("results", {
  state: () => ({
    /** 结果目录清单（扫描后填充）。 */
    resultCatalog: null as ResultCatalog | null,
    /** 最近加载的主场（视口/图表展示用）。 */
    loadedField: null as ScalarField | null,
    loadedFieldStats: null as FieldStats | null,
    /** 对比场（两场差值的减数）。 */
    compareField: null as ScalarField | null,
    /** 探针时间序列：每探针一份「时间步序 → 值」采样。 */
    probeTimeSeries: [] as ProbeTimeSeries[],
    /** 时间序列对应的原始场名（如 T）。 */
    probeSeriesField: null as string | null,
    /** 探针列表（节点序号）。 */
    probes: [] as Probe[],
    /** 最近一次浇口位置分析报告（null = 未运行过）。 */
    gateLocation: null as GateLocationReport | null,
    /** 最近一次填充预览报告（null = 未运行过）。 */
    fillPreview: null as FillPreviewReport | null,
    /** 最近一次加载的矢量场三分量（null = 未加载）。 */
    vectorField: null as VectorField | null,
    vectorFieldStats: null as FieldStats | null,
    /** 最近一次加载的对称张量场（null = 未加载）。 */
    tensorField: null as TensorField | null,
  }),
  actions: {
    /** 运行浇口位置分析（轻量启发式，不经求解器）：适合度场直接作为当前场
     *  载入视口云图，Top-N 建议供模具网络面板一键落浇口。 */
    async runGateLocation(geometryId: string, topN = 5): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在分析浇口位置…", async () => {
        const report = await analyzeGateLocation(geometryId, topN);
        this.gateLocation = report;
        this.loadedField = {
          field: "浇口适合度",
          timeDir: "—",
          timeS: 0,
          values: report.field,
          isMagnitude: false,
          complete: true,
        };
      });
    },
    /** 加载对称张量场（残余应力 / 取向张量）：模量作为当前场进云图，主轴供面板读数。 */
    async loadTensorComponents(caseDir: string, timeDir: string, field: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在加载张量场…", async () => {
        const tensor = await loadTensorField(caseDir, timeDir, field);
        this.tensorField = tensor;
        this.loadedField = {
          field: tensor.field,
          timeDir: tensor.timeDir,
          timeS: tensor.timeS,
          values: tensor.magnitudes,
          isMagnitude: true,
          complete: tensor.complete,
        };
      });
    },
    /** 变形显示：按已加载的矢量场（位移）偏移渲染网格；失败进全局错误并返回 null。 */
    async deformMesh(geometryId: string, scale: number): Promise<RenderMeshData | null> {
      const app = useAppStore();
      return (
        (await app.withBusy("正在计算变形网格…", () => deformRenderMesh(geometryId, scale))) ?? null
      );
    },
    /** 加载矢量场三分量（如位移 D）：供矢量展示与派生消费；失败进全局错误。 */
    async loadVectorComponents(caseDir: string, timeDir: string, field: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在加载矢量场…", async () => {
        this.vectorField = await loadVectorField(caseDir, timeDir, field);
        this.vectorFieldStats = await summarizeVectorField(caseDir, timeDir, field);
      });
    },
    /** 填充预览：以当前方案的浇口为源做覆盖估计，覆盖场作为当前场载入视口；
     *  未覆盖 / 落点异常等提示进预览报告（面板展示）。 */
    async runFillPreview(geometryId: string): Promise<void> {
      const app = useAppStore();
      const runners = useProjectStore().activeStudy?.runnerElements ?? [];
      await app.withBusy("正在估算充填覆盖…", async () => {
        const report = await previewFill(geometryId, runners);
        this.fillPreview = report;
        this.loadedField = {
          field: "充填覆盖",
          timeDir: "—",
          timeS: 0,
          values: report.field,
          isMagnitude: false,
          complete: true,
        };
      });
    },
    /** 扫描 case 结果目录（时间步 + 场清单）。 */
    async loadResultsCatalog(caseDir: string): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在扫描结果…", async () => {
        this.resultCatalog = await listResultTimes(caseDir);
      });
    },
    /** 用上次扫描的目录重扫（工具条入口）；尚无目录时引导先在结果面板扫描。 */
    async rescanCatalog(): Promise<void> {
      const catalog = this.resultCatalog;
      if (catalog === null) {
        useAppStore().setError("请先在结果面板填写 case 目录并扫描。");
        return;
      }
      await this.loadResultsCatalog(catalog.caseDir);
    },
    /** 加载指定时间步的场数据到指定槽位（供视口与图表）。 */
    async loadField(
      caseDir: string,
      timeDir: string,
      field: string,
      slot: FieldSlot = "primary",
    ): Promise<void> {
      const app = useAppStore();
      await app.withBusy("正在加载场数据…", async () => {
        const loaded = await loadResultField(caseDir, timeDir, field, slot);
        if (slot === "compare") {
          this.compareField = loaded;
        } else {
          this.loadedField = loaded;
          this.loadedFieldStats = await summarizeResultField(caseDir, timeDir, field);
        }
      });
    },
    /** 添加探针（整数、非负、不重复、且在已加载场的范围内）。 */
    addProbe(nodeIndex: number): void {
      const app = useAppStore();
      if (!Number.isInteger(nodeIndex) || nodeIndex < 0) {
        app.setError("探针节点序号必须为非负整数。");
        return;
      }
      if (this.probes.some((probe) => probe.nodeIndex === nodeIndex)) {
        app.setError(`节点 ${nodeIndex} 已有探针。`);
        return;
      }
      if (this.loadedField !== null && nodeIndex >= this.loadedField.values.length) {
        app.setError(`节点序号超出范围（当前场共 ${this.loadedField.values.length} 个值）。`);
        return;
      }
      this.probes = [...this.probes, { id: ++probeSeq, nodeIndex }];
    },
    removeProbe(id: number): void {
      this.probes = this.probes.filter((probe) => probe.id !== id);
      this.probeTimeSeries = this.probeTimeSeries.filter((series) => series.probeId !== id);
    },
    /** 加载探针时间序列：Rust 只读采样，保持主场与当前显示不变。 */
    async loadProbeTimeSeries(): Promise<void> {
      const app = useAppStore();
      const catalog = this.resultCatalog;
      const probes = this.probes;
      const source = this.loadedField;
      const rawField = source?.field.split(" · ")[0] ?? "";
      if (catalog === null || probes.length === 0 || source === null || rawField === "") {
        return;
      }
      await app.withBusy("正在加载探针时间曲线…", async () => {
        this.probeTimeSeries = await sampleProbeSeries(catalog.caseDir, rawField, probes);
        this.probeSeriesField = rawField;
      });
    },
    /** 对主场执行单场派生（normalize / threshold / linear），写回 loadedField。 */
    async deriveField(request: DeriveRequest): Promise<void> {
      const app = useAppStore();
      const current = this.loadedField;
      if (current === null || current.values.length === 0) {
        return;
      }
      await app.withBusy("正在派生场…", async () => {
        this.loadedField = await deriveFieldApi(request);
      });
    },
    /** 两场差值：主场 − 对比场，结果写回 loadedField。 */
    async deriveDifference(): Promise<void> {
      const app = useAppStore();
      if (this.loadedField === null || this.compareField === null) {
        return;
      }
      await app.withBusy("正在计算差值…", async () => {
        this.loadedField = await deriveDifferenceApi();
      });
    },
    /** 导出已加载场为 CSV（节点序号 + 值）。 */
    async exportFieldCsv(): Promise<void> {
      const app = useAppStore();
      const { loadedField } = this;
      if (loadedField === null || loadedField.values.length === 0) {
        app.setError("暂无可导出的场数据，请先加载场。");
        return;
      }
      let csv: string;
      const catalog = this.resultCatalog;
      const rawField = loadedField.field.split(" · ")[0]!;
      if (catalog !== null && loadedField.timeDir !== "—") {
        csv = await exportResultFieldCsv(catalog.caseDir, loadedField.timeDir, rawField);
      } else {
        // 非求解器生成的本地分析场（如浇口适合度）没有结果目录，保留轻量回退。
        const headers = [
          "node",
          `${loadedField.field}${loadedField.isMagnitude ? " (magnitude)" : ""}`,
        ];
        csv = toCsv(
          headers,
          loadedField.values.map((value, index) => [index, value]),
        );
      }
      downloadTextFile(`${loadedField.field}-${loadedField.timeDir}.csv`, csv);
    },
  },
});
