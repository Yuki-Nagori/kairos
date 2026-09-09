import {
  getVmStatus,
  installVm as apiInstallVm,
  startVm as apiStartVm,
  stopVm as apiStopVm,
  vmShellSend as apiVmShellSend,
  vmShellStart as apiVmShellStart,
  vmShellStop as apiVmShellStop,
} from "../services/vm";
import { appStore, setError } from "./store";

/** Shell 输出的环形上限：会话可以很长，不能让它撑爆内存。 */
const SHELL_LOG_LIMIT = 500;

/** 追加一行 Shell 输出（环形缓冲）。 */
function appendShellLog(line: string): void {
  const logs = [...appStore.get().vmShellLogs, line];
  if (logs.length > SHELL_LOG_LIMIT) {
    logs.splice(0, logs.length - SHELL_LOG_LIMIT);
  }
  appStore.set({ vmShellLogs: logs });
}

/** 探测虚拟机运行时状态。 */
export async function refreshVmStatus(): Promise<void> {
  try {
    const vmStatus = await getVmStatus();
    appStore.set({ vmStatus });
  } catch (error) {
    setError(error);
  }
}

/** 安装虚拟机运行时：日志实时滚动进 Shell 输出区，结束后重扫状态。 */
export async function installVmAction(): Promise<void> {
  if (appStore.get().vmBusy !== null) {
    return;
  }
  appStore.set({ vmBusy: "install", error: null });
  try {
    await apiInstallVm(appendShellLog);
    await refreshVmStatus();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ vmBusy: null });
  }
}

/** 创建 / 启动受管实例（首次需下载镜像，耗时分钟级），结束后重扫状态。 */
export async function startVmAction(): Promise<void> {
  if (appStore.get().vmBusy !== null) {
    return;
  }
  appStore.set({ vmBusy: "start", error: null });
  try {
    await apiStartVm(appendShellLog);
    await refreshVmStatus();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ vmBusy: null });
  }
}

/** 开启应用内 Shell：invoke 很快返回，日志行在会话期间持续到达。 */
export async function openVmShellAction(): Promise<void> {
  if (appStore.get().vmBusy !== null) {
    return;
  }
  appStore.set({ vmBusy: "shell", error: null });
  try {
    await apiVmShellStart(appendShellLog);
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ vmBusy: null });
  }
}

/** 发送一行命令；管道 Shell 没有回显，本函数把输入自己记进日志区。 */
export async function sendVmShellLine(line: string): Promise<void> {
  appendShellLog(`> ${line}`);
  try {
    await apiVmShellSend(line);
  } catch (error) {
    setError(error);
  }
}

/** 结束 Shell 会话。 */
export async function stopVmShellAction(): Promise<void> {
  try {
    await apiVmShellStop();
    appendShellLog("── Shell 会话已结束 ──");
  } catch (error) {
    setError(error);
  }
}

/** 停止受管虚拟机实例（应用退出时 Rust 侧也会联动做一次）。 */
export async function stopVmAction(): Promise<void> {
  if (appStore.get().vmBusy !== null) {
    return;
  }
  appStore.set({ vmBusy: "stop", error: null });
  try {
    await apiStopVm();
    await refreshVmStatus();
  } catch (error) {
    setError(error);
  } finally {
    appStore.set({ vmBusy: null });
  }
}
