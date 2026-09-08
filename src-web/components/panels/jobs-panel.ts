import { probeOpenfoam } from "../../services/solver";
import { appStore, cancelJobAction, refreshJobs, submitJobAction } from "../../state";
import type { Job } from "../../types";
import { button, card, hint, numberInput, statusDot, textInput } from "../ui";

const STATUS_LABEL: Record<Job["status"], string> = {
  queued: "排队中",
  running: "运行中",
  done: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

const STATUS_CLASS: Record<Job["status"], string> = {
  queued: "text-zinc-400",
  running: "text-amber-300",
  done: "text-emerald-400",
  failed: "text-red-400",
  cancelled: "text-zinc-500",
};

/** 求解作业面板：列表、提交与取消（并发预算由调度器控制）。 */
export function createJobsPanel(): HTMLElement {
  const { root, body } = card("求解作业");

  const form = document.createElement("div");
  form.className = "flex flex-wrap items-center gap-2";
  const caseDirInput = textInput("case 目录路径", "flex-1 min-w-48");
  const coresInput = numberInput("2", "w-20");
  coresInput.min = "1";
  const submitButton = button("提交作业", "primary");
  const refreshButton = button("刷新");
  form.append(caseDirInput, coresInput, submitButton, refreshButton);

  const envHint = hint("正在探测 OpenFOAM 环境…");
  void probeOpenfoam().then((check) => {
    envHint.textContent = check.hint;
    envHint.className = check.openfoam && check.solver ? "text-emerald-400" : "text-amber-400";
  });

  const listBox = document.createElement("div");
  listBox.className = "space-y-2";

  submitButton.addEventListener("click", () => {
    const caseDir = caseDirInput.value.trim();
    if (!caseDir) {
      return;
    }
    void submitJobAction(caseDir, Number(coresInput.value) || 2);
  });
  refreshButton.addEventListener("click", () => void refreshJobs());

  function render(): void {
    const { jobs, jobLogs, busy } = appStore.get();
    const working = busy !== null;
    submitButton.disabled = working;
    refreshButton.disabled = working;

    listBox.replaceChildren();
    if (jobs.length === 0) {
      listBox.append(hint("暂无作业。"));
      return;
    }
    for (const job of jobs) {
      const row = document.createElement("div");
      row.className =
        "flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs";

      const id = document.createElement("span");
      id.className = "font-mono text-zinc-300";
      id.textContent = job.id;

      const status = document.createElement("span");
      status.className = `flex items-center gap-1.5 ${STATUS_CLASS[job.status]}`;
      status.append(statusDot(STATUS_CLASS[job.status].replace("text-", "bg-")));
      status.textContent = STATUS_LABEL[job.status];

      const dir = document.createElement("span");
      dir.className = "truncate text-zinc-500";
      dir.textContent = job.caseDir;

      const time = document.createElement("span");
      time.className = "text-zinc-500";
      time.textContent = job.lastTimeS !== null ? `t = ${job.lastTimeS.toFixed(2)} s` : "";

      const cancel = button("取消", "danger");
      cancel.disabled = working || (job.status !== "queued" && job.status !== "running");
      cancel.addEventListener("click", () => void cancelJobAction(job.id));

      row.append(id, status, dir, time, cancel);
      listBox.append(row);

      // 求解日志尾部（环形缓冲的最后 8 行），运行中与结束后都可查看。
      const logs = jobLogs[job.id] ?? [];
      if (logs.length > 0) {
        const tail = document.createElement("pre");
        tail.className =
          "max-h-32 overflow-y-auto whitespace-pre-wrap break-all rounded border border-zinc-800 bg-zinc-950 px-2 py-1 text-[10px] leading-4 text-zinc-500";
        tail.textContent = logs.slice(-8).join("\n");
        listBox.append(tail);
      }
    }
  }

  body.append(envHint, form, listBox);
  render();
  appStore.subscribe(render);
  return root;
}
