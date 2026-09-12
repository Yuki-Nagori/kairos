<script setup lang="ts">
/** 方案摘要面板：材料 / 成型工艺 / 最新作业三段只读属性行，常驻右列首位
 *  （对齐 Moldflow 方案窗格的配置速览）。逻辑见 useStudySummary。 */
import { useStudySummary } from "./useStudySummary";
import Card from "../../components/ui/UiCard.vue";

const { materialRows, processRows, jobRows } = useStudySummary();
</script>

<template>
  <Card title="方案摘要">
    <div class="space-y-3">
      <!-- 材料 -->
      <section>
        <h3 class="mb-1 text-[11px] font-semibold text-zinc-400">材料</h3>
        <template v-if="materialRows">
          <div
            v-for="row in materialRows"
            :key="row.k"
            class="flex justify-between border-b border-dashed border-zinc-800 py-1 text-xs"
          >
            <span class="text-zinc-500">{{ row.k }}</span>
            <span class="font-mono text-zinc-200">{{ row.v }}</span>
          </div>
        </template>
        <p v-else class="text-[11px] text-zinc-600">
          未登记材料——在材料库面板选择并「用于当前研究」。
        </p>
      </section>
      <!-- 成型工艺 -->
      <section>
        <h3 class="mb-1 text-[11px] font-semibold text-zinc-400">成型工艺</h3>
        <template v-if="processRows">
          <div
            v-for="row in processRows"
            :key="row.k"
            class="flex justify-between border-b border-dashed border-zinc-800 py-1 text-xs"
          >
            <span class="text-zinc-500">{{ row.k }}</span>
            <span class="font-mono text-zinc-200">{{ row.v }}</span>
          </div>
        </template>
        <p v-else class="text-[11px] text-zinc-600">未设置工艺——在工艺面板填写并应用到研究。</p>
      </section>
      <!-- 最新作业 -->
      <section>
        <h3 class="mb-1 text-[11px] font-semibold text-zinc-400">求解作业</h3>
        <template v-if="jobRows">
          <div
            v-for="row in jobRows"
            :key="row.k"
            class="flex justify-between border-b border-dashed border-zinc-800 py-1 text-xs"
          >
            <span class="text-zinc-500">{{ row.k }}</span>
            <span class="font-mono text-zinc-200">{{ row.v }}</span>
          </div>
        </template>
        <p v-else class="text-[11px] text-zinc-600">尚未提交作业。</p>
      </section>
    </div>
  </Card>
</template>
