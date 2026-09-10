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
