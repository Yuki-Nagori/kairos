/** 求解结果 IPC：结果目录扫描与场数据加载。 */
import { invokeCommand } from "../utils/ipc";
import type { ResultCatalog, ScalarField } from "../types";

/** 扫描 case 目录的时间步与场文件清单。 */
export function listResultTimes(caseDir: string): Promise<ResultCatalog> {
  return invokeCommand("list_result_times", { caseDir });
}

/** 加载指定时间步的场（矢量场返回模量）。 */
export function loadResultField(
  caseDir: string,
  timeDir: string,
  field: string,
): Promise<ScalarField> {
  return invokeCommand("load_result_field", { caseDir, timeDir, field });
}

/** 对会话内最近加载的场执行派生（normalize / threshold），返回派生后的场。 */
export function deriveField(kind: string): Promise<ScalarField> {
  return invokeCommand("derive_field", { kind });
}
