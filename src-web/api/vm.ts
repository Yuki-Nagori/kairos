/** 虚拟机 IPC：状态探测、安装 / 启动 / 停止、应用内 Shell 与 bundle 部署。 */
import { invokeCommand } from "../utils/ipc";
import { Channel } from "@tauri-apps/api/core";
import type { VmStatus } from "../types";

/** 探测虚拟机运行时状态（provider 随平台固定：macOS → multipass，Windows → wsl）。 */
export function getVmStatus(): Promise<VmStatus> {
  return invokeCommand("vm_status");
}

/** 一键安装运行时（macOS：brew cask；Windows：UAC 提权 wsl --install）。 */
export function installVm(onLog: (line: string) => void): Promise<string> {
  const channel = new Channel<string>();
  channel.onmessage = onLog;
  return invokeCommand("vm_install", { progress: channel });
}

/** 确保受管实例就绪（缺则创建，停则启动；首次创建需下载镜像）。 */
export function startVm(onLog: (line: string) => void): Promise<string> {
  const channel = new Channel<string>();
  channel.onmessage = onLog;
  return invokeCommand("vm_start", { progress: channel });
}

/** 开启应用内 Shell：输出行经 Channel 持续回传（会话在 Rust 侧长驻）。 */
export function vmShellStart(onLog: (line: string) => void): Promise<void> {
  const channel = new Channel<string>();
  channel.onmessage = onLog;
  return invokeCommand("vm_shell_start", { log: channel });
}

/** 向 Shell 会话发送一行命令。 */
export function vmShellSend(line: string): Promise<void> {
  return invokeCommand("vm_shell_send", { line });
}

/** 结束 Shell 会话（杀子进程）。 */
export function vmShellStop(): Promise<void> {
  return invokeCommand("vm_shell_stop");
}

/** 停止受管虚拟机实例。 */
export function stopVm(): Promise<string> {
  return invokeCommand("vm_stop");
}

/** 部署求解环境：把受管 bundle 传输进虚拟机并解压（multipass 平台）。 */
export function deployVmBundle(onLog: (line: string) => void): Promise<string> {
  const channel = new Channel<string>();
  channel.onmessage = onLog;
  return invokeCommand("vm_deploy_bundle", { progress: channel });
}

/** 读取 VM 内已部署的求解环境版本标签（非 multipass / 未部署 → null）。 */
export function getDeployedReleaseTag(): Promise<string | null> {
  return invokeCommand("vm_deployed_release_tag");
}
