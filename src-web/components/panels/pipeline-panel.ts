import { appStore, submitPipeline } from "../../state";
import type { AnalysisStage } from "../../types";
import { allPrerequisitesDone, evaluatePipeline } from "../../lib/pipeline";
import { button, card, dropdown, hint, numberInput } from "../ui";

/** 流水线引导面板：五步闭环的状态检查与下一步指引（T11）。 */
export function createPipelinePanel(): HTMLElement {
  const { root, body } = card("填充分析流程", { collapsible: true });
  root.classList.add("shrink-0");

  const stepsBox = document.createElement("ol");
  stepsBox.className = "space-y-2";

  const submitRow = document.createElement("div");
  submitRow.className = "flex flex-wrap items-center gap-2";
  const coresInput = numberInput("2", "w-20");
  coresInput.min = "1";
  coresInput.title = "并行核数";
  const coresLabel = hint("核数");
  const stageSelect = dropdown();
  for (const [value, label] of [
    ["fill", "填充"],
    ["fill_pack", "填充 + 保压"],
    ["fill_pack_cool", "填充 + 保压 + 冷却"],
  ] as Array<[string, string]>) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    stageSelect.append(option);
  }
  const submitButton = button("提交求解作业", "primary");
  submitRow.append(stageSelect, coresLabel, coresInput, submitButton);

  submitButton.addEventListener("click", () => {
    void submitPipeline(Number(coresInput.value) || 2, stageSelect.value as AnalysisStage);
  });

  function render(): void {
    const { project, activeStudyId, geometries, meshReports, materials, jobs } = appStore.get();
    const steps = evaluatePipeline({
      geometries,
      meshReports,
      project,
      activeStudyId,
      materials,
      jobs,
    });
    const allDone = allPrerequisitesDone(steps);

    stepsBox.replaceChildren();
    steps.forEach((step, index) => {
      const item = document.createElement("li");
      item.className = "relative flex gap-3 pb-3 last:pb-0";

      // 竖直连接线（最后一项不画）
      if (index < steps.length - 1) {
        const rail = document.createElement("span");
        rail.className = "absolute left-[7px] top-5 bottom-0 w-px bg-zinc-800";
        item.append(rail);
      }

      const mark = document.createElement("span");
      mark.className = `relative z-10 mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full border text-[9px] font-semibold ${
        step.done
          ? "border-emerald-500 bg-emerald-500/15 text-emerald-400"
          : "border-zinc-700 bg-zinc-950 text-zinc-500"
      }`;
      mark.textContent = step.done ? "✓" : String(index + 1);

      const box = document.createElement("div");
      const label = document.createElement("p");
      label.className = `text-xs leading-5 ${step.done ? "text-zinc-500 line-through" : "text-zinc-200"}`;
      label.textContent = step.label;
      box.append(label);
      if (!step.done && step.hint) {
        const hintLine = document.createElement("p");
        hintLine.className = "text-[11px] text-zinc-500";
        hintLine.textContent = step.hint;
        box.append(hintLine);
      }
      item.append(mark, box);
      stepsBox.append(item);
    });

    submitButton.disabled = !allDone || appStore.get().busy !== null;

    // 求解实时进度：运行中作业的物理时间（Job.lastTimeS 由 Rust 侧解析日志）
    const job = appStore.get().jobs.at(-1);
    if (job?.status === "running" && job.lastTimeS !== null) {
      liveLine.textContent = `⟳ 求解中 · T = ${job.lastTimeS.toFixed(2)} s`;
    } else {
      liveLine.textContent = "";
    }
  }

  // 求解实时状态行：显示运行中作业的物理时间（来自日志 Time 解析）
  const liveLine = document.createElement("p");
  liveLine.className = "text-[11px] tabular-nums text-amber-400";

  body.append(stepsBox, liveLine, submitRow);
  render();
  appStore.subscribe(render);
  return root;
}
