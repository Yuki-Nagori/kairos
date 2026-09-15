<script setup lang="ts">
/** 工艺设置面板：逻辑见 useProcessPanel。 */
import { useProcessPanel } from "./useProcessPanel";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const {
  app,
  project,
  form,
  FIELDS,
  issueLines,
  notice,
  caseInlet,
  applyProcess,
  presetName,
  selectedPreset,
  presetNames,
  savePreset,
  loadPreset,
} = useProcessPanel();
</script>

<template>
  <Card title="工艺设置（作用于活跃方案）">
    <div class="flex flex-wrap gap-3">
      <label v-for="field in FIELDS" :key="field.key" class="flex flex-col gap-1">
        <span class="text-xs text-zinc-400">{{ field.label }}</span>
        <TextInput
          v-model="form[field.key]"
          type="number"
          :placeholder="field.placeholder"
          class="w-24"
          step="any"
        />
      </label>
    </div>
    <label class="mt-2 flex flex-col gap-1">
      <span class="text-xs text-zinc-400">完整保压曲线 MPa（可选，格式：时间=压力，逗号分隔）</span>
      <TextInput
        v-model="form.packingPressureCurve"
        type="text"
        placeholder="0=0.9229,0.2=27.6282,315.0797=27.6282"
        class="w-full max-w-xl"
      />
      <span class="text-[11px] text-zinc-500">留空时沿用上方保压压力和时间生成两点曲线。</span>
    </label>
    <UiButton
      variant="primary"
      :disabled="project.activeStudy === null || app.working"
      @click="applyProcess"
    >
      校验并应用到方案
    </UiButton>
    <div class="space-y-1">
      <template v-if="issueLines.length > 0">
        <p v-for="(issue, index) in issueLines" :key="index" class="text-xs text-red-300">
          • {{ issue }}
        </p>
      </template>
      <p v-else-if="notice !== null" class="text-xs text-zinc-500">{{ notice }}</p>
      <template v-if="caseInlet !== null">
        <p class="border-t border-zinc-800 pt-1 text-[11px] text-zinc-400">
          {{ caseInlet.text }}
        </p>
        <p
          v-for="gate in caseInlet.gates"
          :key="gate.key"
          class="text-[11px]"
          :class="gate.warn ? 'text-rose-400' : 'text-zinc-500'"
        >
          {{ gate.text }}
        </p>
        <p v-for="warning in caseInlet.warnings" :key="warning" class="text-[11px] text-rose-400">
          {{ warning }}
        </p>
      </template>
    </div>
    <hr class="border-zinc-800" />
    <div class="flex flex-wrap items-center gap-2">
      <TextInput v-model="presetName" type="text" placeholder="预设名称" class="w-32" />
      <UiButton @click="savePreset">保存预设</UiButton>
      <Dropdown v-model="selectedPreset">
        <option value="">选择预设…</option>
        <option v-for="name in presetNames" :key="name" :value="name">{{ name }}</option>
      </Dropdown>
      <UiButton @click="loadPreset">载入预设</UiButton>
    </div>
  </Card>
</template>
