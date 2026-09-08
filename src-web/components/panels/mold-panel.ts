import {
  addCoolingChannel,
  addRunnerElement,
  appStore,
  checkNetwork,
  removeCoolingChannel,
  removeRunnerElement,
} from "../../state";
import type { CoolingChannel, RunnerElement, RunnerKind } from "../../types";
import { button, card, dropdown, hint, numberInput, sectionLabel } from "../ui";

function numInput(value: number, extraClass = "w-20"): HTMLInputElement {
  const input = numberInput(String(value), extraClass);
  input.step = "any";
  return input;
}

/** 带起/终标记与 XYZ 轴标签的坐标输入组；输入按顺序收集进传入数组。 */
function coordGroup(kind: string, sink: HTMLInputElement[]): HTMLDivElement {
  const group = document.createElement("div");
  group.className =
    "flex items-center gap-0.5 rounded-md border border-zinc-800 bg-zinc-950/60 px-1.5 py-1";
  const tag = document.createElement("span");
  tag.className = "mr-0.5 text-[9px] font-semibold text-zinc-500";
  tag.textContent = kind;
  group.append(tag);
  for (const axis of ["X", "Y", "Z"]) {
    const chip = document.createElement("span");
    chip.className = "w-2.5 text-center text-[9px] font-medium text-zinc-600";
    chip.textContent = axis;
    const input = numInput(0, "w-12 px-1 py-0.5 text-[11px]");
    input.title = `${kind} ${axis}`;
    sink.push(input);
    group.append(chip, input);
  }
  return group;
}

/** 模具网络面板：流道 / 浇口 + 冷却水路的编辑、列表与连通性校验（作用于活跃研究）。 */
export function createMoldPanel(): HTMLElement {
  const { root, body } = card("模具网络（流道 · 浇口 · 冷却）");

  // —— 流道 / 浇口表单 ——
  const runnerForm = document.createElement("div");
  runnerForm.className = "flex flex-wrap items-center gap-1.5";
  const kindSelect = dropdown();
  for (const [value, label] of [
    ["runner", "流道"],
    ["gate", "浇口"],
  ] as Array<[RunnerKind, string]>) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    kindSelect.append(option);
  }
  const diameterInput = numInput(6);
  diameterInput.title = "直径 mm";
  const coords: HTMLInputElement[] = [];
  runnerForm.append(kindSelect, diameterInput, coordGroup("起", coords), coordGroup("终", coords));
  const addRunnerButton = button("添加单元");

  const runnerList = document.createElement("div");
  runnerList.className = "space-y-1";

  // —— 冷却水路表单 ——
  const channelForm = document.createElement("div");
  channelForm.className = "flex flex-wrap items-center gap-1.5";
  const channelDiameter = numInput(8);
  channelDiameter.title = "直径 mm";
  const channelInputs: HTMLInputElement[] = [];
  channelForm.append(
    channelDiameter,
    coordGroup("起", channelInputs),
    coordGroup("终", channelInputs),
  );
  const inletTemp = numInput(25);
  inletTemp.title = "入口温度 °C";
  channelForm.append(inletTemp);
  const addChannelButton = button("添加水路");

  const channelList = document.createElement("div");
  channelList.className = "space-y-1";

  // —— 校验 ——
  const checkButton = button("校验连通性", "primary");
  const issuesBox = document.createElement("div");

  const xyz = (inputs: HTMLInputElement[]): [number, number, number] => [
    Number(inputs[0]?.value ?? 0),
    Number(inputs[1]?.value ?? 0),
    Number(inputs[2]?.value ?? 0),
  ];

  addRunnerButton.addEventListener("click", () => {
    addRunnerElement(
      kindSelect.value as RunnerKind,
      Number(diameterInput.value),
      xyz(coords.slice(0, 3)),
      xyz(coords.slice(3, 6)),
    );
  });
  addChannelButton.addEventListener("click", () => {
    addCoolingChannel(
      Number(channelDiameter.value),
      xyz(channelInputs.slice(0, 3)),
      xyz(channelInputs.slice(3, 6)),
      Number(inletTemp.value),
    );
  });
  checkButton.addEventListener("click", () => void checkNetwork());

  function elementRow(label: string, onRemove: () => void): HTMLElement {
    const row = document.createElement("div");
    row.className =
      "flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-950/50 px-2.5 py-1.5 text-xs";
    const labelElement = document.createElement("span");
    labelElement.className = "text-zinc-300";
    labelElement.textContent = label;
    const removeButton = button("✕", "danger");
    removeButton.addEventListener("click", onRemove);
    row.append(labelElement, removeButton);
    return row;
  }

  function render(): void {
    const { project, activeStudyId, moldIssues, busy } = appStore.get();
    const working = busy !== null;
    const study = project?.studies.find((s) => s.id === activeStudyId) ?? null;

    for (const control of [
      addRunnerButton,
      addChannelButton,
      checkButton,
      kindSelect,
      diameterInput,
      channelDiameter,
      inletTemp,
      ...coords,
      ...channelInputs,
    ]) {
      control.disabled = study === null || working;
    }

    runnerList.replaceChildren();
    channelList.replaceChildren();
    if (study === null) {
      runnerList.append(hint("请先在上方项目栏选择或创建一个研究。"));
    } else {
      for (const element of study.runnerElements as RunnerElement[]) {
        const kindLabel = element.kind === "gate" ? "浇口" : "流道";
        runnerList.append(
          elementRow(`${kindLabel} ${element.id} · Ø${element.diameterMm} mm`, () =>
            removeRunnerElement(element.id),
          ),
        );
      }
      for (const channel of study.coolingChannels as CoolingChannel[]) {
        channelList.append(
          elementRow(
            `水路 ${channel.id} · Ø${channel.diameterMm} mm · ${channel.inletTempC}°C`,
            () => removeCoolingChannel(channel.id),
          ),
        );
      }
      if (study.runnerElements.length === 0) {
        runnerList.append(hint("尚无单元。"));
      }
      if (study.coolingChannels.length === 0) {
        channelList.append(hint("尚无水路。"));
      }
    }

    issuesBox.replaceChildren();
    issuesBox.className = moldIssues.length
      ? "space-y-1 rounded-lg border border-red-900 bg-red-950/40 p-3"
      : "space-y-1";
    for (const issue of moldIssues) {
      const line = document.createElement("p");
      line.className = "text-xs text-red-300";
      line.textContent = `• ${issue}`;
      issuesBox.append(line);
    }
  }

  body.append(
    sectionLabel("流道 / 浇口（起点 xyz → 终点 xyz，mm）"),
    runnerForm,
    runnerList,
    sectionLabel("冷却水路（起点 xyz → 终点 xyz，mm；入口温度 °C）"),
    channelForm,
    channelList,
    checkButton,
    issuesBox,
  );
  render();
  appStore.subscribe(render);
  return root;
}
