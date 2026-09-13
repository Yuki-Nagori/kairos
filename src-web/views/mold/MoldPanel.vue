<script setup lang="ts">
/** 模具网络面板：逻辑见 useMoldPanel。 */
import { useMoldPanel } from "./useMoldPanel";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const {
  AXES,
  project,
  formDisabled,
  study,
  runnerKind,
  runnerDiameter,
  runnerStart,
  runnerEnd,
  addRunner,
  runnerLabel,
  channelDiameter,
  channelStart,
  channelEnd,
  inletTemp,
  addChannel,
  channelLabel,
  check,
} = useMoldPanel();
</script>

<template>
  <Card title="模具网络（流道 · 浇口 · 冷却）" collapsible>
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
      <p v-if="study === null" class="text-xs text-zinc-500">请先在左侧工程面板新建或选择方案。</p>
      <template v-else>
        <div
          v-for="element in study.runnerElements"
          :key="element.id"
          class="flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-950/50 px-2.5 py-1.5 text-xs"
        >
          <span class="text-zinc-300">{{ runnerLabel(element) }}</span>
          <UiButton variant="danger" @click="project.removeRunnerElement(element.id)">✕</UiButton>
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
      <p v-if="study === null" class="text-xs text-zinc-500">请先在左侧工程面板新建或选择方案。</p>
      <template v-else>
        <div
          v-for="channel in study.coolingChannels"
          :key="channel.id"
          class="flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-950/50 px-2.5 py-1.5 text-xs"
        >
          <span class="text-zinc-300">{{ channelLabel(channel) }}</span>
          <UiButton variant="danger" @click="project.removeCoolingChannel(channel.id)">✕</UiButton>
        </div>
        <p v-if="study.coolingChannels.length === 0" class="text-xs text-zinc-500">尚无水路。</p>
      </template>
    </div>

    <UiButton variant="primary" :disabled="formDisabled" @click="check">校验连通性</UiButton>

    <div
      :class="
        project.moldIssues.length > 0
          ? 'space-y-1 rounded-lg border border-red-900 bg-red-950/40 p-3'
          : 'space-y-1'
      "
    >
      <p v-for="(issue, index) in project.moldIssues" :key="index" class="text-xs text-red-300">
        • {{ issue }}
      </p>
    </div>
  </Card>
</template>
