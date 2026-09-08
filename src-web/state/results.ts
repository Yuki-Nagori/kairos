import { listResultTimes, loadResultField } from "../services/results";
import { toCsv } from "../lib/chart";
import { appStore, setError } from "./store";

/** 扫描 case 结果目录（时间步 + 场清单）。 */
export async function loadResultsCatalog(caseDir: string): Promise<void> {
  appStore.set({ busy: "正在扫描结果…", error: null });
  try {
    const catalog = await listResultTimes(caseDir);
    appStore.set({ resultCatalog: catalog });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

/** 加载指定时间步的场数据（供视口与图表）。 */
export async function loadField(caseDir: string, timeDir: string, field: string): Promise<void> {
  appStore.set({ busy: "正在加载场数据…", error: null });
  try {
    const loadedField = await loadResultField(caseDir, timeDir, field);
    appStore.set({ loadedField });
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ busy: null });
  }
}

let probeSeq = 0;

/** 添加探针（整数、非负、不重复、且在已加载场的范围内）。 */
export function addProbe(nodeIndex: number): void {
  const { loadedField, probes } = appStore.get();
  if (!Number.isInteger(nodeIndex) || nodeIndex < 0) {
    setError("探针节点序号必须为非负整数。");
    return;
  }
  if (probes.some((probe) => probe.nodeIndex === nodeIndex)) {
    setError(`节点 ${nodeIndex} 已有探针。`);
    return;
  }
  if (loadedField !== null && nodeIndex >= loadedField.values.length) {
    setError(`节点序号超出范围（当前场共 ${loadedField.values.length} 个值）。`);
    return;
  }
  appStore.set({ probes: [...probes, { id: ++probeSeq, nodeIndex }] });
}

export function removeProbe(id: number): void {
  appStore.set({ probes: appStore.get().probes.filter((probe) => probe.id !== id) });
}

/** 导出已加载场为 CSV（节点序号 + 值）。 */
export function exportFieldCsv(): void {
  const { loadedField } = appStore.get();
  if (loadedField === null || loadedField.values.length === 0) {
    setError("暂无可导出的场数据，请先加载场。");
    return;
  }
  const headers = ["node", `${loadedField.field}${loadedField.isMagnitude ? " (magnitude)" : ""}`];
  const rows = loadedField.values.map((value, index) => [index, value]);
  const csv = toCsv(headers, rows);
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${loadedField.field}-${loadedField.timeDir}.csv`;
  anchor.click();
  URL.revokeObjectURL(url);
}
