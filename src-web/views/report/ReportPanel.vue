<script setup lang="ts">
/** 仿真报告面板：逻辑见 useReportPanel。 */
import { useReportPanel } from "./useReportPanel";
import Card from "../../components/ui/UiCard.vue";
import UiButton from "../../components/ui/UiButton.vue";
import UiTextInput from "../../components/ui/UiTextInput.vue";

const { status, template, generateReport } = useReportPanel();
</script>

<template>
  <Card title="仿真报告">
    <div class="w-full space-y-2 border-t border-zinc-800 pt-3">
      <p class="text-xs font-semibold text-zinc-300">报告模板</p>
      <UiTextInput v-model="template.title" placeholder="报告标题（空 = 默认）" />
      <textarea
        v-model="template.notes"
        placeholder="备注（可选）"
        class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs text-zinc-200"
        rows="3"
      ></textarea>
      <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-zinc-400">
        <label class="flex items-center gap-1">
          <input v-model="template.sections.parameters" type="checkbox" /> 材料与工艺
        </label>
        <label class="flex items-center gap-1">
          <input v-model="template.sections.geometry" type="checkbox" /> 几何摘要
        </label>
        <label class="flex items-center gap-1">
          <input v-model="template.sections.fieldStats" type="checkbox" /> 结果统计
        </label>
        <label class="flex items-center gap-1">
          <input v-model="template.sections.probes" type="checkbox" /> 探针数值
        </label>
        <label class="flex items-center gap-1">
          <input v-model="template.sections.timeSeries" type="checkbox" /> 时间序列
        </label>
        <label class="flex items-center gap-1">
          <input v-model="template.sections.snapshots" type="checkbox" /> 云图与曲线
        </label>
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <UiButton variant="primary" @click="generateReport">生成 HTML 报告</UiButton>
      <p class="text-xs text-zinc-500">{{ status }}</p>
    </div>
  </Card>
</template>
