/** 工艺设置校验 IPC。 */
import { invokeCommand } from "../utils/ipc";
import type { ProcessSettings } from "../types";

/** 填充工况上下文：件体积（mm³）与浇口流通面积（m²），缺省则跳过量级检查。 */
export interface FillLoadContext {
  volumeMm3?: number;
  inletAreaM2?: number;
}

/** 校验工艺设置，返回问题清单（空 = 通过）。 */
export function checkProcess(
  settings: ProcessSettings,
  context: FillLoadContext = {},
): Promise<string[]> {
  return invokeCommand("check_process", {
    settings,
    volumeMm3: context.volumeMm3 ?? null,
    inletAreaM2: context.inletAreaM2 ?? null,
  });
}
