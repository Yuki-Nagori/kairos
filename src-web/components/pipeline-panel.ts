import { appStore, submitPipeline } from "../state";
import type { AnalysisStage } from "../types";
import { allPrerequisitesDone, evaluatePipeline } from "../lib/pipeline";
import { button, card, hint, textInput } from "./ui";

/** 流水线引导面板：五步闭环的状态检查与下一步指引（T11）。 */
export function createPipelinePanel(): HTMLElement {
  const { root, body } = card("填充分析流程");

  const stepsBox = document.createElement("ol");
  stepsBox.className = "space-y-2";

  const submitRow = document.createElement("div");
  submitRow.className = "flex flex-wrap items-center gap-2";
  const coresInput = textInput("2");
  coresInput.type = "number";
  coresInput.min = "1";
  coresInput.className += " w-20";
  coresInput.title = "并行核数";
  const coresLabel = hint("核数");
  const stageSelect = document.createElement("select");
  stageSelect.className =
    "rounded-lg border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs text-zinc-300";
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
    for (const step of steps) {
      const item = document.createElement("li");
      item.className = "flex items-start gap-2 text-xs";
      const mark = document.createElement("span");
      mark.className = step.done ? "text-emerald-400" : "text-zinc-500";
      mark.textContent = step.done ? "✓" : "○";
      const box = document.createElement("div");
      const label = document.createElement("p");
      label.className = step.done ? "text-zinc-400 line-through" : "text-zinc-200";
      label.textContent = step.label;
      box.append(label);
      if (!step.done && step.hint) {
        const hintLine = document.createElement("p");
        hintLine.className = "text-zinc-500";
        hintLine.textContent = step.hint;
        box.append(hintLine);
      }
      item.append(mark, box);
      stepsBox.append(item);
    }

    submitButton.disabled = !allDone || appStore.get().busy !== null;
  }

  body.append(stepsBox, submitRow);
  render();
  appStore.subscribe(render);
  return root;
}
