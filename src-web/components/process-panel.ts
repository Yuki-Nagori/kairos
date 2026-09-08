import { appStore } from "../state";
import type { ProcessSettings } from "../types";
import { checkProcess } from "../services/process";
import { button, card, dropdown, hint, numberInput, textInput } from "./ui";

const PRESETS_KEY = "kairos-process-presets";

/** 出厂默认工艺（量级参考通用热塑性塑料，用户可覆盖）。 */
function defaultProcess(): ProcessSettings {
  return {
    meltTempC: 230,
    moldTempC: 40,
    ejectionTempC: 90,
    injectionTimeS: 1.5,
    vpSwitchVolumePercent: 96,
    packingPressureMpaCurve: [
      [0, 60],
      [8, 48],
    ],
    packingTimeS: 8,
    coolingTimeS: 15,
    coolantTempC: 25,
  };
}

interface ProcessInputs {
  meltTempC: HTMLInputElement;
  moldTempC: HTMLInputElement;
  ejectionTempC: HTMLInputElement;
  injectionTimeS: HTMLInputElement;
  vpSwitchVolumePercent: HTMLInputElement;
  packingPressureMpa: HTMLInputElement;
  packingTimeS: HTMLInputElement;
  coolingTimeS: HTMLInputElement;
  coolantTempC: HTMLInputElement;
}

function numField(
  labelText: string,
  value: number,
): { label: HTMLSpanElement; input: HTMLInputElement } {
  const label = document.createElement("span");
  label.className = "text-xs text-zinc-400";
  label.textContent = labelText;
  const input = numberInput(String(value), "w-24");
  input.step = "any";
  return { label, input };
}

/** 工艺设置面板：参数表单、校验、应用到活跃研究、预设（localStorage）。 */
export function createProcessPanel(): HTMLElement {
  const { root, body } = card("工艺设置（作用于活跃研究）");

  // 仅在切换活跃研究时回填表单，避免覆盖用户正在编辑的内容。
  let filledForStudyId: string | null = null;
  const defaults = defaultProcess();

  const form = document.createElement("div");
  form.className = "flex flex-wrap gap-3";

  function addField(labelText: string, value: number): HTMLInputElement {
    const { label, input } = numField(labelText, value);
    const wrapper = document.createElement("label");
    wrapper.className = "flex flex-col gap-1";
    wrapper.append(label, input);
    form.append(wrapper);
    return input;
  }

  const inputs: ProcessInputs = {
    meltTempC: addField("熔体温度 °C", defaults.meltTempC),
    moldTempC: addField("模具温度 °C", defaults.moldTempC),
    ejectionTempC: addField("顶出温度 °C", defaults.ejectionTempC),
    injectionTimeS: addField("注射时间 s", defaults.injectionTimeS),
    vpSwitchVolumePercent: addField("V/P 切换（体积 %）", defaults.vpSwitchVolumePercent),
    packingPressureMpa: addField("保压压力 MPa", 60),
    packingTimeS: addField("保压时间 s", defaults.packingTimeS),
    coolingTimeS: addField("冷却时间 s", defaults.coolingTimeS),
    coolantTempC: addField("介质温度 °C", defaults.coolantTempC),
  };

  function collectSettings(): ProcessSettings {
    const packingPressure = Number(inputs.packingPressureMpa.value);
    const packingTime = Number(inputs.packingTimeS.value);
    return {
      meltTempC: Number(inputs.meltTempC.value),
      moldTempC: Number(inputs.moldTempC.value),
      ejectionTempC: Number(inputs.ejectionTempC.value),
      injectionTimeS: Number(inputs.injectionTimeS.value),
      vpSwitchVolumePercent: Number(inputs.vpSwitchVolumePercent.value),
      packingPressureMpaCurve: [
        [0, packingPressure],
        [packingTime, packingPressure * 0.8],
      ],
      packingTimeS: packingTime,
      coolingTimeS: Number(inputs.coolingTimeS.value),
      coolantTempC: Number(inputs.coolantTempC.value),
    };
  }

  function backfill(settings: ProcessSettings): void {
    inputs.meltTempC.value = String(settings.meltTempC);
    inputs.moldTempC.value = String(settings.moldTempC);
    inputs.ejectionTempC.value = String(settings.ejectionTempC);
    inputs.injectionTimeS.value = String(settings.injectionTimeS);
    inputs.vpSwitchVolumePercent.value = String(settings.vpSwitchVolumePercent);
    inputs.packingPressureMpa.value = String(settings.packingPressureMpaCurve.at(-1)?.[1] ?? 60);
    inputs.packingTimeS.value = String(settings.packingTimeS);
    inputs.coolingTimeS.value = String(settings.coolingTimeS);
    inputs.coolantTempC.value = String(settings.coolantTempC);
  }

  const applyButton = button("校验并应用到研究", "primary");
  const issuesBox = document.createElement("div");
  issuesBox.className = "space-y-1";

  applyButton.addEventListener("click", () => {
    const settings = collectSettings();
    void checkProcess(settings).then((issues) => {
      issuesBox.replaceChildren();
      if (issues.length > 0) {
        for (const issue of issues) {
          const line = document.createElement("p");
          line.className = "text-xs text-red-300";
          line.textContent = `• ${issue}`;
          issuesBox.append(line);
        }
        return;
      }
      const { project, activeStudyId } = appStore.get();
      if (project === null || activeStudyId === null) {
        issuesBox.append(hint("请先选择一个研究。"));
        return;
      }
      const studies = project.studies.map((study) =>
        study.id === activeStudyId ? { ...study, process: settings } : study,
      );
      appStore.set({ project: { ...project, studies, updatedMs: Date.now() } });
      issuesBox.append(hint("已应用到当前研究"));
    });
  });

  // —— 预设（localStorage，随应用保留）——
  const presetPrefix = `${PRESETS_KEY}:`;
  const presetName = textInput("预设名称", "w-32");
  const savePresetButton = button("保存预设");
  const presetSelect = dropdown();
  const loadPresetButton = button("载入预设");

  function refreshPresetSelect(): void {
    presetSelect.replaceChildren();
    const placeholder = document.createElement("option");
    placeholder.textContent = "选择预设…";
    placeholder.value = "";
    placeholder.selected = true;
    presetSelect.append(placeholder);
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key?.startsWith(presetPrefix)) {
        const option = document.createElement("option");
        option.value = key.slice(presetPrefix.length);
        option.textContent = option.value;
        presetSelect.append(option);
      }
    }
  }

  savePresetButton.addEventListener("click", () => {
    const name = presetName.value.trim();
    if (!name) {
      return;
    }
    localStorage.setItem(`${presetPrefix}${name}`, JSON.stringify(collectSettings()));
    refreshPresetSelect();
    presetSelect.value = name;
  });

  loadPresetButton.addEventListener("click", () => {
    const raw = localStorage.getItem(`${presetPrefix}${presetSelect.value}`);
    if (raw) {
      backfill(JSON.parse(raw) as ProcessSettings);
    }
  });

  const presetBar = document.createElement("div");
  presetBar.className = "flex flex-wrap items-center gap-2";
  presetBar.append(presetName, savePresetButton, presetSelect, loadPresetButton);

  const divider = document.createElement("hr");
  divider.className = "border-zinc-800";

  body.append(form, applyButton, issuesBox, divider, presetBar);
  refreshPresetSelect();
  render();
  appStore.subscribe(render);
  return root;

  function render(): void {
    const { project, activeStudyId, busy } = appStore.get();
    const study = project?.studies.find((s) => s.id === activeStudyId) ?? null;
    applyButton.disabled = study === null || busy !== null;
    // 切换研究时回填该研究的工艺设置；其他状态变化不覆盖表单。
    if (activeStudyId !== filledForStudyId) {
      filledForStudyId = activeStudyId;
      if (study?.process) {
        backfill(study.process);
      }
    }
  }
}
