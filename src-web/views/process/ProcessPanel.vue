<script setup lang="ts">
/**
 * 工艺设置面板：参数表单、校验、应用到活跃研究、预设（localStorage）。
 * 输入值保持字符串形态、空输入经 Number() 落为 0，与原生 input 取值语义一致
 * （出厂默认量级只进 placeholder，不预填 value）；回填仅跟随活跃研究切换触发，
 * 其余状态变化不得覆盖用户正在编辑的表单。
 */
import { computed, reactive, ref, watch } from "vue";
import { appStore, useAppState } from "../../state";
import type { ProcessSettings } from "../../types";
import { checkProcess } from "../../api/process";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

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

const state = useAppState();
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

const activeStudy = computed(
  () => state.project?.studies.find((s) => s.id === state.activeStudyId) ?? null,
);

// 校验问题（红）与成功/引导提示（灰）互斥：每次应用后整体重建，与原生 replaceChildren 一致。
const issueLines = ref<string[]>([]);
const notice = ref<string | null>(null);

function applyProcess(): void {
  const settings = collectSettings();
  void checkProcess(settings).then((found) => {
    issueLines.value = [];
    notice.value = null;
    if (found.length > 0) {
      issueLines.value = found;
      return;
    }
    const { project, activeStudyId } = appStore.get();
    if (project === null || activeStudyId === null) {
      notice.value = "请先选择一个研究。";
      return;
    }
    const studies = project.studies.map((study) =>
      study.id === activeStudyId ? { ...study, process: settings } : study,
    );
    appStore.set({ project: { ...project, studies, updatedMs: Date.now() } });
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
// （watch 自带「值变化」判定，等价原生版的 filledForStudyId 哨兵；immediate 覆盖首帧）。
watch(
  () => state.activeStudyId,
  (activeStudyId) => {
    const study = state.project?.studies.find((s) => s.id === activeStudyId) ?? null;
    if (study?.process) {
      backfill(study.process);
    }
  },
  { immediate: true },
);
</script>

<template>
  <Card title="工艺设置（作用于活跃研究）">
    <div class="flex flex-wrap gap-3">
      <label v-for="field in FIELDS" :key="field.key" class="flex flex-col gap-1">
        <span class="text-xs text-zinc-400">{{ field.label }}</span>
        <TextInput
          v-model="form[field.key]"
          type="number"
          :placeholder="field.placeholder"
          class="w-24"
          step="any"
        />
      </label>
    </div>
    <UiButton
      variant="primary"
      :disabled="activeStudy === null || state.busy !== null"
      @click="applyProcess"
    >
      校验并应用到研究
    </UiButton>
    <div class="space-y-1">
      <template v-if="issueLines.length > 0">
        <p v-for="(issue, index) in issueLines" :key="index" class="text-xs text-red-300">
          • {{ issue }}
        </p>
      </template>
      <p v-else-if="notice !== null" class="text-xs text-zinc-500">{{ notice }}</p>
    </div>
    <hr class="border-zinc-800" />
    <div class="flex flex-wrap items-center gap-2">
      <TextInput v-model="presetName" type="text" placeholder="预设名称" class="w-32" />
      <UiButton @click="savePreset">保存预设</UiButton>
      <Dropdown v-model="selectedPreset">
        <option value="">选择预设…</option>
        <option v-for="name in presetNames" :key="name" :value="name">{{ name }}</option>
      </Dropdown>
      <UiButton @click="loadPreset">载入预设</UiButton>
    </div>
  </Card>
</template>
