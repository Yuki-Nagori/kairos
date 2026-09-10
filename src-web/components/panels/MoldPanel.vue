<script setup lang="ts">
/**
 * 模具网络面板：流道 / 浇口 + 冷却水路的编辑、列表与连通性校验，
 * 作用于活跃研究；未选研究或有操作进行中时整个表单禁用。
 * 坐标输入按「起点 xyz → 终点 xyz」成组，数值统一 mm（入口温度 °C）。
 */
import { computed, reactive, ref } from "vue";
import {
  addCoolingChannel,
  addRunnerElement,
  checkNetwork,
  removeCoolingChannel,
  removeRunnerElement,
  useAppState,
} from "../../state";
import type { CoolingChannel, RunnerElement, RunnerKind } from "../../types";
import Card from "../ui/Card.vue";
import Dropdown from "../ui/Dropdown.vue";
import TextInput from "../ui/TextInput.vue";
import UiButton from "../ui/Button.vue";

// 坐标按 XYZ 键值存储（键为字面量联合，索引访问不引入 undefined）。
const AXES = [
  { key: "x", label: "X" },
  { key: "y", label: "Y" },
  { key: "z", label: "Z" },
] as const;

type AxisKey = (typeof AXES)[number]["key"];

function zeroCoords(): Record<AxisKey, string> {
  return { x: "0", y: "0", z: "0" };
}

const state = useAppState();

const working = computed(() => state.busy !== null);

const study = computed(
  () => state.project?.studies.find((s) => s.id === state.activeStudyId) ?? null,
);

const formDisabled = computed(() => study.value === null || working.value);

// —— 流道 / 浇口表单 ——
const runnerKind = ref("runner");
const runnerDiameter = ref("6");
const runnerStart = reactive(zeroCoords());
const runnerEnd = reactive(zeroCoords());

// —— 冷却水路表单 ——
const channelDiameter = ref("8");
const channelStart = reactive(zeroCoords());
const channelEnd = reactive(zeroCoords());
const inletTemp = ref("25");

// 空串按 0 处理（与 Number("") === 0 的原生行为一致）。
function xyz(values: Record<AxisKey, string>): [number, number, number] {
  return [Number(values.x ?? 0), Number(values.y ?? 0), Number(values.z ?? 0)];
}

function addRunner(): void {
  addRunnerElement(
    runnerKind.value as RunnerKind,
    Number(runnerDiameter.value),
    xyz(runnerStart),
    xyz(runnerEnd),
  );
}

function addChannel(): void {
  addCoolingChannel(
    Number(channelDiameter.value),
    xyz(channelStart),
    xyz(channelEnd),
    Number(inletTemp.value),
  );
}

function check(): void {
  void checkNetwork();
}

function runnerLabel(element: RunnerElement): string {
  return `${element.kind === "gate" ? "浇口" : "流道"} ${element.id} · Ø${element.diameterMm} mm`;
}

function channelLabel(channel: CoolingChannel): string {
  return `水路 ${channel.id} · Ø${channel.diameterMm} mm · ${channel.inletTempC}°C`;
}
</script>

<template>
  <Card title="模具网络（流道 · 浇口 · 冷却）">
    <p class="text-[11px] font-semibold tracking-wide text-zinc-400">
      流道 / 浇口（起点 xyz → 终点 xyz，mm）
    </p>
    <div class="flex flex-wrap items-center gap-1.5">
      <Dropdown v-model="runnerKind" :disabled="formDisabled">
        <option value="runner">流道</option>
        <option value="gate">浇口</option>
      </Dropdown>
      <TextInput
        v-model="runnerDiameter"
        type="number"
        class="w-20"
        step="any"
        title="直径 mm"
        :disabled="formDisabled"
      />
      <!-- 起点坐标组 -->
      <div
        class="flex items-center gap-0.5 rounded-md border border-zinc-800 bg-zinc-950/60 px-1.5 py-1"
      >
        <span class="mr-0.5 text-[9px] font-semibold text-zinc-500">起</span>
        <template v-for="axis in AXES" :key="axis.key">
          <span class="w-2.5 text-center text-[9px] font-medium text-zinc-600">{{
            axis.label
          }}</span>
          <TextInput
            v-model="runnerStart[axis.key]"
            type="number"
            class="w-12 px-1 py-0.5 text-[11px]"
            :title="`起 ${axis.label}`"
            :disabled="formDisabled"
          />
        </template>
      </div>
      <!-- 终点坐标组 -->
      <div
        class="flex items-center gap-0.5 rounded-md border border-zinc-800 bg-zinc-950/60 px-1.5 py-1"
      >
        <span class="mr-0.5 text-[9px] font-semibold text-zinc-500">终</span>
        <template v-for="axis in AXES" :key="axis.key">
          <span class="w-2.5 text-center text-[9px] font-medium text-zinc-600">{{
            axis.label
          }}</span>
          <TextInput
            v-model="runnerEnd[axis.key]"
            type="number"
            class="w-12 px-1 py-0.5 text-[11px]"
            :title="`终 ${axis.label}`"
            :disabled="formDisabled"
          />
        </template>
      </div>
      <UiButton :disabled="formDisabled" @click="addRunner">添加单元</UiButton>
    </div>

    <div class="space-y-1">
      <p v-if="study === null" class="text-xs text-zinc-500">
        请先在上方项目栏选择或创建一个研究。
      </p>
      <template v-else>
        <div
          v-for="element in study.runnerElements"
          :key="element.id"
          class="flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-950/50 px-2.5 py-1.5 text-xs"
        >
          <span class="text-zinc-300">{{ runnerLabel(element) }}</span>
          <UiButton variant="danger" @click="removeRunnerElement(element.id)">✕</UiButton>
        </div>
        <p v-if="study.runnerElements.length === 0" class="text-xs text-zinc-500">尚无单元。</p>
      </template>
    </div>

    <p class="text-[11px] font-semibold tracking-wide text-zinc-400">
      冷却水路（起点 xyz → 终点 xyz，mm；入口温度 °C）
    </p>
    <div class="flex flex-wrap items-center gap-1.5">
      <TextInput
        v-model="channelDiameter"
        type="number"
        class="w-20"
        step="any"
        title="直径 mm"
        :disabled="formDisabled"
      />
      <!-- 起点坐标组 -->
      <div
        class="flex items-center gap-0.5 rounded-md border border-zinc-800 bg-zinc-950/60 px-1.5 py-1"
      >
        <span class="mr-0.5 text-[9px] font-semibold text-zinc-500">起</span>
        <template v-for="axis in AXES" :key="axis.key">
          <span class="w-2.5 text-center text-[9px] font-medium text-zinc-600">{{
            axis.label
          }}</span>
          <TextInput
            v-model="channelStart[axis.key]"
            type="number"
            class="w-12 px-1 py-0.5 text-[11px]"
            :title="`起 ${axis.label}`"
            :disabled="formDisabled"
          />
        </template>
      </div>
      <!-- 终点坐标组 -->
      <div
        class="flex items-center gap-0.5 rounded-md border border-zinc-800 bg-zinc-950/60 px-1.5 py-1"
      >
        <span class="mr-0.5 text-[9px] font-semibold text-zinc-500">终</span>
        <template v-for="axis in AXES" :key="axis.key">
          <span class="w-2.5 text-center text-[9px] font-medium text-zinc-600">{{
            axis.label
          }}</span>
          <TextInput
            v-model="channelEnd[axis.key]"
            type="number"
            class="w-12 px-1 py-0.5 text-[11px]"
            :title="`终 ${axis.label}`"
            :disabled="formDisabled"
          />
        </template>
      </div>
      <TextInput
        v-model="inletTemp"
        type="number"
        class="w-20"
        step="any"
        title="入口温度 °C"
        :disabled="formDisabled"
      />
      <UiButton :disabled="formDisabled" @click="addChannel">添加水路</UiButton>
    </div>

    <div class="space-y-1">
      <p v-if="study === null" class="text-xs text-zinc-500">
        请先在上方项目栏选择或创建一个研究。
      </p>
      <template v-else>
        <div
          v-for="channel in study.coolingChannels"
          :key="channel.id"
          class="flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-950/50 px-2.5 py-1.5 text-xs"
        >
          <span class="text-zinc-300">{{ channelLabel(channel) }}</span>
          <UiButton variant="danger" @click="removeCoolingChannel(channel.id)">✕</UiButton>
        </div>
        <p v-if="study.coolingChannels.length === 0" class="text-xs text-zinc-500">尚无水路。</p>
      </template>
    </div>

    <UiButton variant="primary" :disabled="formDisabled" @click="check">校验连通性</UiButton>

    <div
      :class="
        state.moldIssues.length > 0
          ? 'space-y-1 rounded-lg border border-red-900 bg-red-950/40 p-3'
          : 'space-y-1'
      "
    >
      <p v-for="(issue, index) in state.moldIssues" :key="index" class="text-xs text-red-300">
        • {{ issue }}
      </p>
    </div>
  </Card>
</template>
