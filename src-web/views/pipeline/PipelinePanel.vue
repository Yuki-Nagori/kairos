<script setup lang="ts">
/** 方案任务面板：步骤三态清单（完成 ✓ / 进行中 ⟳ / 待办）+ 提交表单，逻辑见 usePipelinePanel。 */
import { usePipelinePanel } from "./usePipelinePanel";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const { stage, cores, steps, submitDisabled, liveText, submit } = usePipelinePanel();
</script>

<template>
  <Card title="方案任务" collapsible class="shrink-0">
    <ol class="space-y-1">
      <li v-for="step in steps" :key="step.id" class="flex gap-2.5">
        <!-- 三态图标：完成实心 ✓ / 进行中琥珀 ⟳ / 待办空心 -->
        <span
          class="mt-0.5 flex size-3.5 shrink-0 items-center justify-center rounded text-[9px] font-semibold"
          :class="
            step.done
              ? 'bg-emerald-800 text-emerald-300'
              : step.doing
                ? 'border-[1.5px] border-amber-500 text-amber-500'
                : 'border-[1.5px] border-zinc-700'
          "
          >{{ step.done ? "✓" : step.doing ? "⟳" : "" }}</span
        >
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span
              class="text-xs"
              :class="step.done ? 'text-zinc-500' : step.doing ? 'text-zinc-100' : 'text-zinc-400'"
              >{{ step.label }}</span
            >
            <!-- 进行中步骤右侧流式进度（运行中作业的物理时间） -->
            <span
              v-if="step.doing && liveText"
              class="ml-auto text-[11px] tabular-nums text-amber-400"
              >{{ liveText }}</span
            >
          </div>
          <p v-if="!step.done && step.hint" class="text-[11px] leading-4 text-zinc-500">
            {{ step.hint }}
          </p>
        </div>
      </li>
    </ol>
    <!-- 空闲时占位保持行高，运行中显示全局实时状态 -->
    <p class="text-[11px] tabular-nums" :class="liveText ? 'text-amber-400' : ''">{{ liveText }}</p>
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
