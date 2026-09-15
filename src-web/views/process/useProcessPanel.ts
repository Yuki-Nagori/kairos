/**
 * 工艺设置面板：参数表单、校验、应用到活跃方案、预设（localStorage）。
 * 输入值保持字符串形态（原生 input.value 即字符串）、空输入经 Number() 落为 0；
 * 方案没有工艺时表单直接带出厂默认值（用户改完即可应用，不必逐格手填），
 * 出厂默认同时常驻「选择预设」清单可随时回填。回填仅跟随活跃方案切换触发，
 * 其余状态变化不得覆盖用户正在编辑的表单。
 */
import { computed, reactive, ref, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useProcessStore } from "../../stores/process";
import { useGeometryStore } from "../../stores/geometry";
import { useProjectStore } from "../../stores/project";
import type { ProcessSettings } from "../../types";
import { storageGet, storageIndex, storageKey, storageSet } from "../../utils/storage";
import { fixed } from "../../utils/format";

export function useProcessPanel() {
  /** 预设存储域：清单在 kairos:process-preset:index，条目在 kairos:process-preset:<名>。 */
  const PRESET_DOMAIN = "process-preset";
  const presets = storageIndex();

  /** 内置预设名：常驻选择清单（不落存储），选中即回填出厂默认。 */
  const BUILTIN_PRESET = "出厂默认";
  const MUG_BASELINE_PRESET = "Mug 基线（220°C / 50°C）";

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

  /** Mug 基线工艺预设。 */
  function mugBaselineProcess(): ProcessSettings {
    return {
      meltTempC: 220,
      moldTempC: 50,
      ejectionTempC: 101,
      injectionTimeS: 5.5,
      vpSwitchVolumePercent: 96,
      packingPressureMpaCurve: [
        [0, 0.9229],
        [0.2, 27.6282],
        [315.0797, 27.6282],
      ],
      packingTimeS: 315.0797,
      coolingTimeS: 20,
      coolantTempC: 50,
    };
  }

  const app = useAppStore();
  const processStore = useProcessStore();
  const project = useProjectStore();
  const geometryStore = useGeometryStore();
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
    packingPressureCurve: "",
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
    const curve = form.packingPressureCurve
      .split(",")
      .map((point) => point.trim())
      .filter(Boolean)
      .map((point) => point.split("=").map(Number) as [number, number])
      .filter(([time, pressure]) => Number.isFinite(time) && Number.isFinite(pressure));
    return {
      meltTempC: Number(form.meltTempC),
      moldTempC: Number(form.moldTempC),
      ejectionTempC: Number(form.ejectionTempC),
      injectionTimeS: Number(form.injectionTimeS),
      vpSwitchVolumePercent: Number(form.vpSwitchVolumePercent),
      packingPressureMpaCurve:
        curve.length >= 2
          ? curve
          : [
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
    // 表单字段是曲线**峰值**（应用时按峰值铺成 100% → 80% 两点，
    // 见 collectSettings）：回填取起点才对得上，取末点会让「应用 → 回填 → 再应用」
    // 每来一次就衰减到 80%。
    form.packingPressureMpa = String(settings.packingPressureMpaCurve.at(0)?.[1] ?? 60);
    form.packingTimeS = String(settings.packingTimeS);
    form.coolingTimeS = String(settings.coolingTimeS);
    form.coolantTempC = String(settings.coolantTempC);
    form.packingPressureCurve =
      settings.packingPressureMpaCurve.length > 2
        ? settings.packingPressureMpaCurve
            .map(([time, pressure]) => `${time}=${pressure}`)
            .join(",")
        : "";
  }

  // 校验问题（红）与成功/引导提示（灰）互斥，每次应用后整体重建。
  const issueLines = ref<string[]>([]);
  const notice = ref<string | null>(null);

  /** 填充工况上下文：活跃几何的网格体积 + 浇口流通面积。
   *  面积优先用最近一次 case 生成回显的**有效面积**（网格实际表达出来的入口），
   *  没有回显时退回请求半径的等效面积。 */
  function fillLoadContext(): { volumeMm3?: number; inletAreaM2?: number } {
    const geometry = geometryStore.geometries[0];
    const report = geometry ? geometryStore.meshReports[geometry.geometryId] : undefined;
    const effective = processStore.effectiveInletAreaM2(project.activeStudyId);
    if (effective !== undefined) {
      return { volumeMm3: report?.totalVolume, inletAreaM2: effective };
    }
    const gate = project.activeStudy?.runnerElements.find(
      (element) => element.kind === "gate" && element.diameterMm > 0,
    );
    const radiusM = gate === undefined ? undefined : gate.diameterMm / 2000;
    return {
      volumeMm3: report?.totalVolume,
      inletAreaM2: radiusM === undefined ? undefined : Math.PI * radiusM * radiusM,
    };
  }

  /** case 生成回显行：请求 vs 实际入口面积（无回显时不显示）。 */
  const caseInlet = computed(() => {
    const record = processStore.caseInlet;
    if (record === null || record.studyId !== project.activeStudyId) {
      return null;
    }
    const { outcome } = record;
    return {
      text: `浇口入口（最近一次 case）：实际 ${fixed(outcome.inletAreaM2 * 1e6, 1)} mm² · 等效 Ø${fixed(outcome.inletEquivalentDiameterMm, 1)} mm`,
      gates: outcome.gates.map((gate) => ({
        key: gate.index,
        text: `浇口 #${gate.index}：请求 Ø${fixed(gate.requestedRadiusMm * 2, 1)} mm（${fixed(gate.requestedAreaMm2, 1)} mm²）→ 实际 ${fixed(gate.actualAreaMm2, 1)} mm² / ${gate.faceCount} 面（${fixed(gate.areaRatio, 2)}×）`,
        warn: !gate.expressible,
      })),
      warnings: outcome.warnings,
    };
  });

  function applyProcess(): void {
    const settings = collectSettings();
    // 校验经 process store（check_process 命令），应用经 touchActiveStudy
    // （不可变更新 + updatedMs 盖章统一走 project store 入口）。
    void processStore.checkProcess(settings, fillLoadContext()).then((clean) => {
      notice.value = null;
      if (!clean) {
        issueLines.value = [...processStore.issues];
        return;
      }
      issueLines.value = [];
      if (project.project === null || project.activeStudyId === null) {
        notice.value = "请先选择一个方案。";
        return;
      }
      project.touchActiveStudy((study) => {
        study.process = settings;
      });
      notice.value = "已应用到当前方案";
    });
  }

  // —— 预设（localStorage，随应用保留）——
  const presetName = ref("");
  const selectedPreset = ref("");
  const presetNames = ref<string[]>([]);

  // localStorage 非响应式，选项清单以显式刷新驱动（保存后面板内同步重建一次）。
  // 内置预设排在最前：不落存储，随时可回填出厂默认。
  function refreshPresetSelect(): void {
    presetNames.value = [BUILTIN_PRESET, MUG_BASELINE_PRESET, ...presets.list(PRESET_DOMAIN)];
  }

  function savePreset(): void {
    const name = presetName.value.trim();
    if (!name) {
      return;
    }
    if (name === BUILTIN_PRESET || name === MUG_BASELINE_PRESET) {
      // 内置预设不落存储：同名保存会让清单出现两项、载入语义分叉。
      notice.value = `「${BUILTIN_PRESET}」是内置预设名，请换一个名称。`;
      issueLines.value = [];
      return;
    }
    storageSet(storageKey(PRESET_DOMAIN, name), collectSettings());
    presets.add(PRESET_DOMAIN, name);
    refreshPresetSelect();
    selectedPreset.value = name;
  }

  function loadPreset(): void {
    if (selectedPreset.value === BUILTIN_PRESET) {
      backfill(defaults);
      return;
    }
    if (selectedPreset.value === MUG_BASELINE_PRESET) {
      backfill(mugBaselineProcess());
      return;
    }
    const saved = storageGet<ProcessSettings | null>(
      storageKey(PRESET_DOMAIN, selectedPreset.value),
      null,
    );
    if (saved !== null) {
      backfill(saved);
    }
  }

  refreshPresetSelect();

  // 仅在切换活跃方案时回填该方案的工艺设置；方案还没有工艺时带出厂默认值，
  // 其他状态变化不覆盖表单（watch 自带「值变化」判定；immediate 覆盖首帧）。
  watch(
    () => project.activeStudyId,
    (activeStudyId) => {
      const study = project.project?.studies.find((s) => s.id === activeStudyId) ?? null;
      backfill(study?.process ?? defaults);
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
    caseInlet,
    applyProcess,
    presetName,
    selectedPreset,
    presetNames,
    savePreset,
    loadPreset,
  };
}
