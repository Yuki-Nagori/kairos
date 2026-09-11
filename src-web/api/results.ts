/** 求解结果 IPC：结果目录扫描、场数据加载与派生算子。 */
import { invokeCommand } from "../utils/ipc";
import { decodeFieldBinary } from "../utils/field-binary";
import type { DeriveRequest, FieldSlot, ResultCatalog, ScalarField } from "../types";

/** 扫描 case 目录的时间步与场文件清单。 */
export function listResultTimes(caseDir: string): Promise<ResultCatalog> {
  return invokeCommand("list_result_times", { caseDir });
}

/** 加载指定时间步的场到会话槽位（矢量场返回模量；默认写入主场）。
 * 走二进制通道（tauri ipc Response 原始字节），前端解析为场对象。 */
export async function loadResultField(
  caseDir: string,
  timeDir: string,
  field: string,
  slot: FieldSlot = "primary",
): Promise<ScalarField> {
  const buffer = await invokeCommand<ArrayBuffer>("load_result_field_binary", {
    caseDir,
    timeDir,
    field,
    slot,
  });
  return decodeFieldBinary(buffer);
}

/** 对会话主场执行单场派生（归一化 / 阈值掩码 / 线性映射），返回派生后的场。 */
export function deriveField(request: DeriveRequest): Promise<ScalarField> {
  return invokeCommand("derive_field", { request });
}

/** 两场差值：会话主场 − 对比场，返回派生后的场。 */
export function deriveDifference(): Promise<ScalarField> {
  return invokeCommand("derive_difference");
}
