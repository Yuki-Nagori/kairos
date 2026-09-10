<script setup lang="ts">
/** 填充分析流程面板：逻辑见 usePipelinePanel。 */
import { usePipelinePanel } from "./usePipelinePanel";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const { stage, cores, steps, submitDisabled, liveText, submit } = usePipelinePanel();
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
