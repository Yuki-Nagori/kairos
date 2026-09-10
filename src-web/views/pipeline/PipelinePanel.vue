<script setup lang="ts">
/**
 * 流水线面板：五步闭环的前置检查与下一步指引，引导从几何到求解提交。
 * 提交按钮仅在「前置全部就绪且空闲」时可用；运行中的作业实时显示
 * 物理时间（Job.lastTimeS 由 Rust 侧解析日志得到）。
 */
import { computed, ref } from "vue";
import { submitPipeline, useAppState } from "../../state";
import type { AnalysisStage } from "../../types";
import { allPrerequisitesDone, evaluatePipeline } from "../../utils/pipeline";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const state = useAppState();

// 提交表单：核数留空时按 2 核提交（与 Number("") || 2 的回退一致）。
const stage = ref("fill");
const cores = ref("");

const steps = computed(() =>
  evaluatePipeline({
    geometries: state.geometries,
    meshReports: state.meshReports,
    project: state.project,
    activeStudyId: state.activeStudyId,
    materials: state.materials,
    jobs: state.jobs,
  }),
);

const submitDisabled = computed(() => !allPrerequisitesDone(steps.value) || state.busy !== null);

// 求解实时状态行：显示运行中作业的物理时间，空闲时为空串（行仍占位）。
const liveText = computed(() => {
  const job = state.jobs.at(-1);
  if (job?.status === "running" && job.lastTimeS !== null) {
    return `⟳ 求解中 · T = ${job.lastTimeS.toFixed(2)} s`;
  }
  return "";
});

function submit(): void {
  void submitPipeline(Number(cores.value) || 2, stage.value as AnalysisStage);
}
</script>

<template>
  <Card title="填充分析流程" collapsible class="shrink-0">
    <ol class="space-y-2">
      <li v-for="(step, index) in steps" :key="step.id" class="relative flex gap-3 pb-3 last:pb-0">
        <!-- 竖直连接线（最后一项不画） -->
        <span
          v-if="index < steps.length - 1"
          class="absolute left-1.75 top-5 bottom-0 w-px bg-zinc-800"
        />
        <span
          class="relative z-10 mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full border text-[9px] font-semibold"
          :class="
            step.done
              ? 'border-emerald-500 bg-emerald-500/15 text-emerald-400'
              : 'border-zinc-700 bg-zinc-950 text-zinc-500'
          "
          >{{ step.done ? "✓" : index + 1 }}</span
        >
        <div>
          <p
            class="text-xs leading-5"
            :class="step.done ? 'text-zinc-500 line-through' : 'text-zinc-200'"
          >
            {{ step.label }}
          </p>
          <p v-if="!step.done && step.hint" class="text-[11px] text-zinc-500">{{ step.hint }}</p>
        </div>
      </li>
    </ol>
    <p class="text-[11px] tabular-nums text-amber-400">{{ liveText }}</p>
    <div class="flex flex-wrap items-center gap-2">
      <Dropdown v-model="stage">
        <option value="fill">填充</option>
        <option value="fill_pack">填充 + 保压</option>
        <option value="fill_pack_cool">填充 + 保压 + 冷却</option>
      </Dropdown>
      <p class="text-xs text-zinc-500">核数</p>
      <TextInput
        v-model="cores"
        type="number"
        placeholder="2"
        class="w-20"
        min="1"
        title="并行核数"
      />
      <UiButton variant="primary" :disabled="submitDisabled" @click="submit">
        提交求解作业
      </UiButton>
    </div>
  </Card>
</template>
