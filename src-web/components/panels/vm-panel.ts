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
import { button, card, hint, textInput } from "../ui";

/** 虚拟机面板：Multipass（macOS）/ WSL2（Windows）的一键安装、启动、
 * 应用内 Shell 与关闭。全部逻辑在 state/vm.ts 与 Rust 适配层，这里只渲染。 */
export function createVmPanel(): HTMLElement {
  const { root, body } = card("虚拟机");

  const refreshButton = button("重新探测");
  const statusLine = hint("探测中…");

  const installButton = button("安装虚拟机");
  const startButton = button("启动虚拟机");
  const shellButton = button("进入 Shell");
  const shellStopButton = button("停止 Shell", "ghost");
  const vmStopButton = button("关闭虚拟机", "ghost");
  const buttons = document.createElement("div");
  buttons.className = "flex flex-wrap items-center gap-2";
  buttons.append(installButton, startButton, shellButton, shellStopButton, vmStopButton);

  const output = document.createElement("pre");
  output.className =
    "max-h-64 overflow-y-auto whitespace-pre-wrap break-all rounded-lg border border-zinc-800 bg-zinc-950 px-2 py-1 text-[10px] leading-4 text-zinc-400";

  const input = textInput("输入命令，回车发送…");
  const sendButton = button("发送", "ghost");
  const inputRow = document.createElement("div");
  inputRow.className = "flex items-center gap-2";
  inputRow.append(input, sendButton);

  body.append(refreshButton, statusLine, buttons, output, inputRow);

  refreshButton.addEventListener("click", () => void refreshVmStatus());
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
  sendButton.addEventListener("click", send);
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
