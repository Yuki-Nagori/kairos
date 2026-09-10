/** 求解结果状态：结果目录清单、最近加载的场与探针列表。 */
import { defineStore } from "pinia";
import { listResultTimes, loadResultField } from "../api/results";
import type { Probe, ResultCatalog, ScalarField } from "../types";
import { toCsv } from "../utils/chart";
import { useAppStore } from "./app";

let probeSeq = 0;

export const useResultsStore = defineStore("results", {
  state: () => ({
    /** 结果目录清单（扫描后填充）。 */
    resultCatalog: null as ResultCatalog | null,
    /** 最近加载的场（视口/图表展示用）。 */
    loadedField: null as ScalarField | null,
    /** 探针列表（节点序号）。 */
    probes: [] as Probe[],
  }),
  actions: {
    /** 扫描 case 结果目录（时间步 + 场清单）。 */
    async loadResultsCatalog(caseDir: string): Promise<void> {
      const app = useAppStore();
      app.beginBusy("正在扫描结果…");
      try {
        this.resultCatalog = await listResultTimes(caseDir);
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
    },
    /** 加载指定时间步的场数据（供视口与图表）。 */
    async loadField(caseDir: string, timeDir: string, field: string): Promise<void> {
      const app = useAppStore();
      app.beginBusy("正在加载场数据…");
      try {
        this.loadedField = await loadResultField(caseDir, timeDir, field);
      } catch (error) {
        app.setError(error);
      } finally {
        app.endBusy();
      }
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
    },
    /** 导出已加载场为 CSV（节点序号 + 值）。 */
    exportFieldCsv(): void {
      const app = useAppStore();
      const { loadedField } = this;
      if (loadedField === null || loadedField.values.length === 0) {
        app.setError("暂无可导出的场数据，请先加载场。");
        return;
      }
      const headers = [
        "node",
        `${loadedField.field}${loadedField.isMagnitude ? " (magnitude)" : ""}`,
      ];
      const rows = loadedField.values.map((value, index) => [index, value]);
      const csv = toCsv(headers, rows);
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${loadedField.field}-${loadedField.timeDir}.csv`;
      anchor.click();
      URL.revokeObjectURL(url);
    },
  },
});
