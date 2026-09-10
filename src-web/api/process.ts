/** 工艺设置校验 IPC。 */
import { invokeCommand } from "../utils/ipc";
import type { ProcessSettings } from "../types";

/** 校验工艺设置，返回问题清单（空 = 通过）。 */
export function checkProcess(settings: ProcessSettings): Promise<string[]> {
  return invokeCommand("check_process", { settings });
}
