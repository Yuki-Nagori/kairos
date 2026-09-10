<script setup lang="ts">
// 逻辑全部抽至同目录 useReportPanel.ts，此处仅保留模板与解构。
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
