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
  working,
  formDisabled,
  study,
  runnerKind,
  runnerDiameter,
  runnerStart,
  runnerEnd,
  addRunner,
  fillPreview,
  fillPreviewDisabled,
  fillPreviewHint,
  runPreview,
  gateSuggestions,
  gateLocationBasis,
  applySuggestion,
  placementActive,
  placementContinuous,
  toggleContinuous,
  placementDisabled,
  placementHint,
  startPlacement,
  cancelPlacement,
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
      <UiButton
        v-if="placementActive"
        variant="danger"
        :disabled="working"
        @click="cancelPlacement"
      >
        取消拾取
      </UiButton>
      <UiButton v-else :disabled="placementDisabled" @click="startPlacement">
        视口拾取放置
      </UiButton>
      <UiButton
        :disabled="placementDisabled"
        :title="placementContinuous ? '拾取后继续等待下一次点击' : '拾取一次后自动退出放置模式'"
        @click="toggleContinuous"
      >
        {{ placementContinuous ? "连续放置：开" : "连续放置：关" }}
      </UiButton>
    </div>
    <p class="text-[11px]" :class="placementActive ? 'text-emerald-400' : 'text-zinc-500'">
      {{ placementHint }}
    </p>

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

    <div class="space-y-1 rounded-lg border border-zinc-800 p-2">
      <div class="flex flex-wrap items-center gap-2">
        <UiButton :disabled="fillPreviewDisabled" @click="runPreview">填充预览</UiButton>
        <p class="text-[11px] text-zinc-500">{{ fillPreviewHint }}</p>
      </div>
      <p
        v-for="warning in fillPreview?.warnings ?? []"
        :key="warning"
        class="text-[11px] text-amber-400"
      >
        {{ warning }}
      </p>
      <p v-if="fillPreview !== null" class="text-[10px] text-zinc-600">{{ fillPreview.basis }}</p>
    </div>

    <div v-if="gateSuggestions.length > 0" class="space-y-1 rounded-lg border border-zinc-800 p-2">
      <p class="text-[11px] font-semibold tracking-wide text-zinc-400">浇口位置建议（Top-N）</p>
      <div
        v-for="suggestion in gateSuggestions"
        :key="suggestion.cell"
        class="flex items-center justify-between gap-2 text-[11px]"
      >
        <span class="text-zinc-400">{{ suggestion.text }}</span>
        <UiButton :disabled="formDisabled" @click="applySuggestion(suggestion)">设为浇口</UiButton>
      </div>
      <p v-if="gateLocationBasis !== null" class="text-[10px] text-zinc-600">
        {{ gateLocationBasis }}
      </p>
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
