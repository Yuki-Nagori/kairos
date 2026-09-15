import { decodeRenderMesh } from "../utils/render-mesh-binary";
/** 求解结果 IPC：结果目录扫描、场数据加载与派生算子。 */
import { invokeCommand } from "../utils/ipc";
import { decodeFieldBinary, decodeVectorFieldBinary } from "../utils/field-binary";
import type {
  DeriveRequest,
  Probe,
  ProbeTimeSeries,
  FieldSlot,
  RenderMeshData,
  ResultCatalog,
  ScalarField,
  TensorField,
  VectorField,
} from "../types";

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

/** 由 core 直接编码当前时间步的 CSV，前端不创建逐行中间数组。 */
export function exportResultFieldCsv(
  caseDir: string,
  timeDir: string,
  field: string,
): Promise<string> {
  return invokeCommand("export_result_field_csv", { caseDir, timeDir, field });
}

/** 加载指定时间步的矢量场三分量（变形显示 / 矢量派生用）。 */
export async function loadVectorField(
  caseDir: string,
  timeDir: string,
  field: string,
): Promise<VectorField> {
  const buffer = await invokeCommand<ArrayBuffer>("load_vector_field_binary", {
    caseDir,
    timeDir,
    field,
  });
  return decodeVectorFieldBinary(buffer);
}

/** 加载对称张量场（残余应力 / 取向张量）：分量 + 模量 + 主方向。 */
export function loadTensorField(
  caseDir: string,
  timeDir: string,
  field: string,
): Promise<TensorField> {
  return invokeCommand("load_tensor_field", { caseDir, timeDir, field });
}

/** 变形显示：按会话里已加载的矢量场（位移）偏移渲染网格，返回变形后的网格。 */
export async function deformRenderMesh(geometryId: string, scale: number): Promise<RenderMeshData> {
  return decodeRenderMesh(
    await invokeCommand<ArrayBuffer>("deform_render_mesh", { geometryId, scale }),
  );
}

/** 对会话主场执行单场派生（归一化 / 阈值掩码 / 线性映射），返回派生后的场。 */
export function deriveField(request: DeriveRequest): Promise<ScalarField> {
  return invokeCommand("derive_field", { request });
}

/** 两场差值：会话主场 − 对比场，返回派生后的场。 */
export function deriveDifference(): Promise<ScalarField> {
  return invokeCommand("derive_difference");
}

/** 只返回探针时间曲线，不修改后端主场与对比场。 */
export function sampleProbeSeries(
  caseDir: string,
  field: string,
  probes: Probe[],
): Promise<ProbeTimeSeries[]> {
  return invokeCommand("sample_probe_series", { caseDir, field, probes });
}
