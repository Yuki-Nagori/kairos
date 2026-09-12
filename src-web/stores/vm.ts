/** 虚拟机域状态：Multipass / WSL2 探测结果、应用内 Shell 会话与终端抽屉显隐。 */
import { defineStore } from "pinia";
import * as api from "../api/vm";
import type { VmAction, VmStatus } from "../types";
import { useAppStore } from "./app";

/** Shell 输出的环形上限：会话可以很长，不能让它撑爆内存。 */
const SHELL_LOG_LIMIT = 500;

export const useVmStore = defineStore("vm", {
  state: () => ({
    vmStatus: null as VmStatus | null,
    /** VM 内已部署的求解环境版本标签（null = 非 multipass / 未部署 / 未知）。 */
    deployedReleaseTag: null as string | null,
    /** 应用内 Shell 输出（环形缓冲）。 */
    vmShellLogs: [] as string[],
    /** 进行中的动作（同一时刻至多一个）。 */
    vmBusy: null as VmAction | null,
    /** 终端抽屉是否可见（状态栏右侧 Shell 按钮与原生菜单切换）。 */
    vmPanelVisible: false,
  }),
  actions: {
    /** 追加一行 Shell 输出（环形缓冲）。 */
    appendShellLog(line: string): void {
      this.vmShellLogs.push(line);
      if (this.vmShellLogs.length > SHELL_LOG_LIMIT) {
        this.vmShellLogs.splice(0, this.vmShellLogs.length - SHELL_LOG_LIMIT);
      }
    },
    /** 互斥执行一个虚拟机动作：进行中拒绝并发，失败进全局错误，结束清 busy。 */
    async withBusy(action: VmAction, run: () => Promise<void>): Promise<void> {
      if (this.vmBusy !== null) {
        return;
      }
      this.vmBusy = action;
      useAppStore().error = null;
      try {
        await run();
      } catch (error) {
        useAppStore().setError(error);
      } finally {
        this.vmBusy = null;
      }
    },
    /** 部署求解环境：传输 bundle 进虚拟机并解压，日志实时滚动进终端面板；
     *  成功后刷新 VM 内版本标记（供「更新未部署」提醒比对）。 */
    async deployVmBundle(): Promise<void> {
      await this.withBusy("shell", async () => {
        await api.deployVmBundle((line) => this.appendShellLog(line));
        await this.refreshVmStatus();
        await this.refreshDeployedReleaseTag();
      });
    },
    /** 读取 VM 内已部署版本标签（multipass 通道；其余平台恒为 null）。 */
    async refreshDeployedReleaseTag(): Promise<void> {
      const app = useAppStore();
      try {
        this.deployedReleaseTag = await api.getDeployedReleaseTag();
      } catch (error) {
        app.setError(error);
      }
    },
    /** 展开虚拟机终端面板（原生菜单入口触发，非切换）。 */
    showPanel(): void {
      this.vmPanelVisible = true;
    },
    /** 切换终端抽屉显隐（状态栏右侧 Shell 按钮触发）。 */
    togglePanel(): void {
      this.vmPanelVisible = !this.vmPanelVisible;
    },
    /** 探测虚拟机运行时状态。 */
    async refreshVmStatus(): Promise<void> {
      try {
        this.vmStatus = await api.getVmStatus();
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 安装虚拟机运行时：日志实时滚动进 Shell 输出区，结束后重扫状态。 */
    async installVm(): Promise<void> {
      await this.withBusy("install", async () => {
        await api.installVm((line) => this.appendShellLog(line));
        await this.refreshVmStatus();
      });
    },
    /** 创建 / 启动受管实例（首次需下载镜像，耗时分钟级），结束后重扫状态。 */
    async startVm(): Promise<void> {
      await this.withBusy("start", async () => {
        await api.startVm((line) => this.appendShellLog(line));
        await this.refreshVmStatus();
      });
    },
    /** 开启应用内 Shell：invoke 很快返回，日志行在会话期间持续到达。 */
    async openShell(): Promise<void> {
      await this.withBusy("shell", () => api.vmShellStart((line) => this.appendShellLog(line)));
    },
    /** 发送一行命令（PTY 会回显输入行，无需前端手动记日志）。 */
    async sendShellLine(line: string): Promise<void> {
      try {
        await api.vmShellSend(line);
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 结束 Shell 会话。 */
    async stopShell(): Promise<void> {
      try {
        await api.vmShellStop();
        this.appendShellLog("── Shell 会话已结束 ──");
      } catch (error) {
        useAppStore().setError(error);
      }
    },
    /** 停止受管虚拟机实例（应用退出时 Rust 侧也会联动做一次）。 */
    async stopVm(): Promise<void> {
      await this.withBusy("stop", async () => {
        await api.stopVm();
        await this.refreshVmStatus();
      });
    },
  },
});
