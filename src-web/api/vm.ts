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

/** 读取已部署的求解环境版本标签（VM 内 / 本机原生；未知 → null）。 */
export function getDeployedReleaseTag(): Promise<string | null> {
  return invokeCommand("vm_deployed_release_tag");
}

/** 原生（Linux）求解环境状态。 */
export interface NativeEnvStatus {
  /** 环境根目录（bundle 未解压时为 null）。 */
  envRoot: string | null;
  /** bashrc 是否就位（就绪 = 可直接提交作业）。 */
  envReady: boolean;
  /** OpenMPI 运行时（mpirun）是否可用。 */
  mpiReady: boolean;
  /** 处置提示（空 = 无问题）。 */
  hints: string[];
}

/** 探测原生（Linux）求解环境（bundle 是否解压就位、OpenMPI 是否可用）。 */
export function nativeEnvStatus(): Promise<NativeEnvStatus> {
  return invokeCommand("native_env_status");
}

/** 原生平台「部署」：bundle 已在本机解压即就位，这里做结构校验 + 版本标记落盘。 */
export function nativeDeployBundle(): Promise<string> {
  return invokeCommand("native_deploy_bundle");
}
