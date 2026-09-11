/**
 * 工艺设置面板：参数表单、校验、应用到活跃研究、预设（localStorage）。
 * 输入值保持字符串形态（原生 input.value 即字符串）、空输入经 Number() 落为 0；
 * 出厂默认量级只进 placeholder，不预填 value。回填仅跟随活跃研究切换触发，
 * 其余状态变化不得覆盖用户正在编辑的表单。
 */
import { reactive, ref, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useProcessStore } from "../../stores/process";
import { useProjectStore } from "../../stores/project";
import type { ProcessSettings } from "../../types";

export function useProcessPanel() {
  const PRESETS_KEY = "kairos-process-presets";

  /** 出厂默认工艺（量级取通用热塑性塑料的典型值，用户可覆盖）。 */
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

  const app = useAppStore();
  const processStore = useProcessStore();
  const project = useProjectStore();
  const defaults = defaultProcess();

  const form = reactive({
    meltTempC: "",
    moldTempC: "",
    ejectionTempC: "",
    injectionTimeS: "",
    vpSwitchVolumePercent: "",
    packingPressureMpa: "",
    packingTimeS: "",
    coolingTimeS: "",
    coolantTempC: "",
  });

  /** 表单字段定义，顺序即渲染顺序；保压压力出厂量级固定 60。 */
  const FIELDS: { key: keyof typeof form; label: string; placeholder: string }[] = [
    { key: "meltTempC", label: "熔体温度 °C", placeholder: String(defaults.meltTempC) },
    { key: "moldTempC", label: "模具温度 °C", placeholder: String(defaults.moldTempC) },
    { key: "ejectionTempC", label: "顶出温度 °C", placeholder: String(defaults.ejectionTempC) },
    { key: "injectionTimeS", label: "注射时间 s", placeholder: String(defaults.injectionTimeS) },
    {
      key: "vpSwitchVolumePercent",
      label: "V/P 切换（体积 %）",
      placeholder: String(defaults.vpSwitchVolumePercent),
    },
    { key: "packingPressureMpa", label: "保压压力 MPa", placeholder: "60" },
    { key: "packingTimeS", label: "保压时间 s", placeholder: String(defaults.packingTimeS) },
    { key: "coolingTimeS", label: "冷却时间 s", placeholder: String(defaults.coolingTimeS) },
    { key: "coolantTempC", label: "介质温度 °C", placeholder: String(defaults.coolantTempC) },
  ];

  function collectSettings(): ProcessSettings {
    const packingPressure = Number(form.packingPressureMpa);
    const packingTime = Number(form.packingTimeS);
    return {
      meltTempC: Number(form.meltTempC),
      moldTempC: Number(form.moldTempC),
      ejectionTempC: Number(form.ejectionTempC),
      injectionTimeS: Number(form.injectionTimeS),
      vpSwitchVolumePercent: Number(form.vpSwitchVolumePercent),
      packingPressureMpaCurve: [
        [0, packingPressure],
        [packingTime, packingPressure * 0.8],
      ],
      packingTimeS: packingTime,
      coolingTimeS: Number(form.coolingTimeS),
      coolantTempC: Number(form.coolantTempC),
    };
  }

  function backfill(settings: ProcessSettings): void {
    form.meltTempC = String(settings.meltTempC);
    form.moldTempC = String(settings.moldTempC);
    form.ejectionTempC = String(settings.ejectionTempC);
    form.injectionTimeS = String(settings.injectionTimeS);
    form.vpSwitchVolumePercent = String(settings.vpSwitchVolumePercent);
    form.packingPressureMpa = String(settings.packingPressureMpaCurve.at(-1)?.[1] ?? 60);
    form.packingTimeS = String(settings.packingTimeS);
    form.coolingTimeS = String(settings.coolingTimeS);
    form.coolantTempC = String(settings.coolantTempC);
  }

  // 校验问题（红）与成功/引导提示（灰）互斥，每次应用后整体重建。
  const issueLines = ref<string[]>([]);
  const notice = ref<string | null>(null);

  function applyProcess(): void {
    const settings = collectSettings();
    // 校验经 process store（check_process 命令），应用经 touchActiveStudy
    // （不可变更新 + updatedMs 盖章统一走 project store 入口）。
    void processStore.checkProcess(settings).then((clean) => {
      notice.value = null;
      if (!clean) {
        issueLines.value = [...processStore.issues];
        return;
      }
      issueLines.value = [];
      if (project.project === null || project.activeStudyId === null) {
        notice.value = "请先选择一个研究。";
        return;
      }
      project.touchActiveStudy((study) => {
        study.process = settings;
      });
      notice.value = "已应用到当前研究";
    });
  }

  // —— 预设（localStorage，随应用保留）——
  const presetPrefix = `${PRESETS_KEY}:`;
  const presetName = ref("");
  const selectedPreset = ref("");
  const presetNames = ref<string[]>([]);

  // localStorage 非响应式，选项清单以显式刷新驱动（保存后面板内同步重建一次）。
  function refreshPresetSelect(): void {
    const names: string[] = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key?.startsWith(presetPrefix)) {
        names.push(key.slice(presetPrefix.length));
      }
    }
    presetNames.value = names;
  }

  function savePreset(): void {
    const name = presetName.value.trim();
    if (!name) {
      return;
    }
    localStorage.setItem(`${presetPrefix}${name}`, JSON.stringify(collectSettings()));
    refreshPresetSelect();
    selectedPreset.value = name;
  }

  function loadPreset(): void {
    const raw = localStorage.getItem(`${presetPrefix}${selectedPreset.value}`);
    if (raw) {
      backfill(JSON.parse(raw) as ProcessSettings);
    }
  }

  refreshPresetSelect();

  // 仅在切换活跃研究时回填该研究的工艺设置；其他状态变化不覆盖表单
  // （watch 自带「值变化」判定；immediate 覆盖首帧）。
  watch(
    () => project.activeStudyId,
    (activeStudyId) => {
      const study = project.project?.studies.find((s) => s.id === activeStudyId) ?? null;
      if (study?.process) {
        backfill(study.process);
      }
    },
    { immediate: true },
  );

  return {
    app,
    project,
    form,
    FIELDS,
    issueLines,
    notice,
    applyProcess,
    presetName,
    selectedPreset,
    presetNames,
    savePreset,
    loadPreset,
  };
}
