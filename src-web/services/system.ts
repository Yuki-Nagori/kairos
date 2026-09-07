import { invokeCommand } from "../lib/ipc";
import type { SystemInfo } from "../types";

/** 拉取系统信息，同时充当 IPC 链路的连通性探测。 */
export function getSystemInfo(): Promise<SystemInfo> {
  return invokeCommand("system_info");
}
