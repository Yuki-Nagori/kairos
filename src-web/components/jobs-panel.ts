import { appStore, cancelJobAction, refreshJobs, submitJobAction } from "../state";
import type { Job } from "../types";
import { button, card, hint, textInput } from "./ui";

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
  const caseDirInput = textInput("case 目录路径");
  caseDirInput.className += " flex-1 min-w-48";
  const coresInput = textInput("2");
  coresInput.type = "number";
  coresInput.min = "1";
  coresInput.className += " w-20";
  const submitButton = button("提交作业", "primary");
  const refreshButton = button("刷新");
  form.append(caseDirInput, coresInput, submitButton, refreshButton);

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
    const { jobs, busy } = appStore.get();
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
      status.className = STATUS_CLASS[job.status];
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
    }
  }

  body.append(form, listBox);
  render();
  appStore.subscribe(render);
  return root;
}
