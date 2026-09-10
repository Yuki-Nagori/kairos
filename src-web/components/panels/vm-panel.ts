import {
  appStore,
  installVmAction,
  openVmShellAction,
  refreshVmStatus,
  sendVmShellLine,
  startVmAction,
  stopVmAction,
  stopVmShellAction,
} from "../../state";
import { createShellIcon } from "../icons";
import { button, card } from "../ui";

/** 虚拟机面板（浮于工作区右下角）：Multipass（macOS）/ WSL2（Windows）的
 * 一键安装、启动、应用内 Shell 与关闭。全部逻辑在 state/vm.ts 与 Rust 适配层，
 * 这里只渲染；Shell 区模拟终端外观（图标标题栏 + 提示符行内输入）。 */
export function createVmPanel(): HTMLElement {
  const shell = card("虚拟机", {
    icon: createShellIcon("h-4 w-4 text-emerald-400"),
    statusHint: "探测中…",
    onRefresh: () => void refreshVmStatus(),
    refreshLabel: "重新探测",
  });
  const { root, body, statusLine } = shell;
  const refreshButton = shell.refreshButton as HTMLButtonElement;

  const installButton = button("安装虚拟机");
  const startButton = button("启动虚拟机");
  const shellButton = button("进入 Shell");
  const shellStopButton = button("停止 Shell", "ghost");
  const vmStopButton = button("关闭虚拟机", "ghost");
  const buttons = document.createElement("div");
  buttons.className = "flex flex-wrap items-center gap-2";
  buttons.append(installButton, startButton, shellButton, shellStopButton, vmStopButton);

  // 模拟终端：深色窗口 + 输出滚动区 + 提示符行内输入。
  const terminal = document.createElement("div");
  terminal.className =
    "overflow-hidden rounded-lg border border-zinc-800 bg-black font-mono select-none";
  const terminalHeader = document.createElement("div");
  terminalHeader.className =
    "flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[10px] text-zinc-400";
  const terminalTitle = document.createElement("span");
  terminalTitle.textContent = "shell";
  terminalHeader.append(createShellIcon("h-3.5 w-3.5 text-emerald-400"), terminalTitle);
  const output = document.createElement("pre");
  output.className =
    "h-44 overflow-y-auto whitespace-pre-wrap break-all px-3 py-2 text-[11px] leading-4 text-emerald-300/90";
  const promptRow = document.createElement("div");
  promptRow.className = "flex items-center gap-1.5 px-3 pb-2 text-[11px]";
  const prompt = document.createElement("span");
  prompt.className = "text-amber-400";
  prompt.textContent = "❯";
  const input = document.createElement("input");
  input.className =
    "min-w-0 flex-1 border-none bg-transparent font-mono text-[11px] text-emerald-200 outline-none placeholder:text-zinc-600";
  input.placeholder = "输入命令，回车发送…";
  input.autocomplete = "off";
  input.spellcheck = false;
  promptRow.append(prompt, input);
  terminal.append(terminalHeader, output, promptRow);
  // 点终端任意处聚焦输入行。
  terminal.addEventListener("click", () => input.focus());

  body.append(buttons, terminal);

  installButton.addEventListener("click", () => void installVmAction());
  startButton.addEventListener("click", () => void startVmAction());
  shellButton.addEventListener("click", () => void openVmShellAction());
  shellStopButton.addEventListener("click", () => void stopVmShellAction());
  vmStopButton.addEventListener("click", () => void stopVmAction());

  const send = (): void => {
    const line = input.value.trim();
    if (line.length === 0) {
      return;
    }
    input.value = "";
    void sendVmShellLine(line);
  };
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      send();
    }
  });

  function render(): void {
    const { vmStatus, vmBusy, vmShellLogs } = appStore.get();
    const busy = vmBusy !== null;
    refreshButton.disabled = busy;
    statusLine.textContent = vmStatus?.hint ?? "尚未探测。点击「重新探测」检查虚拟机运行时。";
    terminalTitle.textContent = vmStatus === null ? "shell" : `shell · ${vmStatus.instanceName}`;

    const toolInstalled = vmStatus?.toolInstalled ?? false;
    const instanceState = vmStatus?.instanceState ?? "missing";
    // Linux 原生环境无虚拟机语义：安装/启动/关闭一律短路。
    const native = vmStatus?.provider === "native";
    installButton.disabled = busy || native || toolInstalled;
    startButton.disabled = busy || native || !toolInstalled || instanceState === "running";
    shellButton.disabled = busy || !toolInstalled || instanceState === "missing";
    shellStopButton.disabled = busy;
    vmStopButton.disabled = busy || native || !toolInstalled || instanceState === "missing";

    output.textContent = vmShellLogs.join("\n");
    output.scrollTop = output.scrollHeight;
  }

  void refreshVmStatus();
  render();
  appStore.subscribe(render);
  return root;
}
