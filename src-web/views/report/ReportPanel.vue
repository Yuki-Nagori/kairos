<script setup lang="ts">
/** 仿真报告面板：逻辑见 useReportPanel。 */
import { useReportPanel } from "./useReportPanel";
import Card from "../../components/ui/UiCard.vue";
import UiButton from "../../components/ui/UiButton.vue";

const { status, generateReport } = useReportPanel();
</script>

<template>
  <Card title="仿真报告">
    <UiButton variant="primary" @click="generateReport">生成 HTML 报告</UiButton>
    <p class="text-xs text-zinc-500">{{ status }}</p>
  </Card>
</template>
