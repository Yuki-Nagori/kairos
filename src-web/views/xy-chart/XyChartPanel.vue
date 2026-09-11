<script setup lang="ts">
/** XY 图表面板：逻辑见 useXyChartPanel。 */
import UiButton from "../../components/ui/UiButton.vue";
import UiTextInput from "../../components/ui/UiTextInput.vue";
import Card from "../../components/ui/UiCard.vue";
import { useResultsStore } from "../../stores/results";
import { useXyChartPanel } from "./useXyChartPanel";

const results = useResultsStore();
const {
  nodeInput,
  working,
  mode,
  canLoadTimeSeries,
  loadTimeSeries,
  selectedTimeDir,
  jumpToTime,
  addProbeFromInput,
  draw,
} = useXyChartPanel();
</script>

<template>
  <Card class="shrink-0" title="XY 曲线与探针">
    <canvas ref="canvasRef" width="720" height="200" class="w-full rounded-lg bg-zinc-950" />
    <div class="flex flex-wrap items-center gap-2">
      <select v-model="mode" class="text-xs">
        <option value="spatial">空间分布</option>
        <option value="time">探针时间曲线</option>
      </select>
      <template v-if="mode === 'time'">
        <UiButton :disabled="!canLoadTimeSeries || working" @click="loadTimeSeries()">
          加载时间曲线
        </UiButton>
        <select v-model="selectedTimeDir" class="text-xs" @change="jumpToTime()">
          <option value="">跳转到时间步…</option>
          <option
            v-for="time in results.resultCatalog?.times ?? []"
            :key="time.dirName"
            :value="time.dirName"
          >
            {{ time.dirName }} ({{ time.timeS.toFixed(3) }}s)
          </option>
        </select>
      </template>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <UiTextInput v-model="nodeInput" placeholder="节点序号" class="w-28" :disabled="working" />
      <UiButton :disabled="working" @click="addProbeFromInput()">添加探针</UiButton>
      <UiButton :disabled="working" @click="results.exportFieldCsv()">导出 CSV</UiButton>
      <UiButton @click="draw()">重绘</UiButton>
    </div>
    <div class="space-y-1">
      <p v-if="results.probes.length === 0" class="text-xs text-zinc-500">
        暂无探针。输入节点序号后添加。
      </p>
      <template v-else>
        <div
          v-for="probe in results.probes"
          :key="probe.id"
          class="flex items-center justify-between rounded border border-zinc-800 px-2 py-1"
        >
          <span class="text-zinc-300">探针 #{{ probe.id }} · 节点 {{ probe.nodeIndex }}</span>
          <UiButton variant="danger" @click="results.removeProbe(probe.id)">✕</UiButton>
        </div>
      </template>
    </div>
  </Card>
</template>
