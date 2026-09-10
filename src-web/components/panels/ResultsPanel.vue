<script setup lang="ts">
/** 结果面板：扫描 OpenFOAM case 结果目录、查看时间步与场统计；三维视口由视口面板负责。 */
import { computed, ref } from "vue";
import { appStore, loadField, loadResultsCatalog, useAppState } from "../../state";
import { minMax } from "../../lib/stats";
import UiButton from "../ui/UiButton.vue";
import Card from "../ui/UiCard.vue";
import UiTextInput from "../ui/UiTextInput.vue";

const state = useAppState();

const dirPath = ref("");
const working = computed(() => state.busy !== null);
const catalog = computed(() => state.resultCatalog);
const loadedField = computed(() => state.loadedField);

// 原扫描按钮在加载进行中禁用——Card 的刷新按钮不外联 disabled，以守卫等价。
function scan(): void {
  if (working.value) {
    return;
  }
  const caseDir = dirPath.value.trim();
  if (caseDir) {
    void loadResultsCatalog(caseDir);
  }
}

// 场统计行：空场用 NaN 占位，格式化保持原样。
const stats = computed(() => {
  const field = state.loadedField;
  if (field === null) {
    return null;
  }
  const values = field.values;
  const { min, max } = values.length > 0 ? minMax(values) : { min: Number.NaN, max: Number.NaN };
  return {
    line: `已加载 ${field.field} @ ${field.timeDir}${field.isMagnitude ? "（模量）" : ""}：${values.length} 个值，min ${min.toFixed(3)} / max ${max.toFixed(3)}`,
    complete: field.complete,
  };
});

// 派生场：对已加载场做标量运算生成新场，图表与视口即时可用。
const deriveKind = ref("normalize");

function deriveField(): void {
  const source = loadedField.value;
  if (!source || source.values.length === 0) {
    return;
  }
  const values = source.values;
  let derived: number[];
  let suffix: string;
  if (deriveKind.value === "threshold") {
    const { min, max } = minMax(values);
    const threshold = (min + max) / 2;
    derived = values.map((v) => (v >= threshold ? 1 : 0));
    suffix = "阈值掩码";
  } else {
    const { min, max } = minMax(values);
    const range = max - min;
    derived = range > 0 ? values.map((v) => (v - min) / range) : values.map(() => 0);
    suffix = "归一化";
  }
  appStore.set({
    loadedField: {
      ...source,
      field: `${source.field} · ${suffix}`,
      isMagnitude: false,
      values: derived,
      complete: source.complete,
    },
  });
}
</script>

<template>
  <Card title="结果" refresh-label="扫描结果" @refresh="scan">
    <div class="flex flex-wrap items-center gap-2">
      <UiTextInput v-model="dirPath" class="flex-1 min-w-48" placeholder="OpenFOAM case 目录路径" />
    </div>
    <div class="space-y-1">
      <p v-if="catalog === null" class="text-xs text-zinc-500">尚未扫描结果目录。</p>
      <p v-else-if="catalog.times.length === 0" class="text-xs text-zinc-500">
        结果目录中未发现时间步。
      </p>
      <table v-else class="w-full text-xs">
        <thead>
          <tr>
            <th class="border-b border-zinc-700 px-2 py-1 text-left text-zinc-400">时间步</th>
            <th class="border-b border-zinc-700 px-2 py-1 text-left text-zinc-400">时间 (s)</th>
            <th class="border-b border-zinc-700 px-2 py-1 text-left text-zinc-400">可用场</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="time in catalog.times" :key="time.dirName">
            <td class="px-2 py-1 text-zinc-300">
              {{ time.dirName
              }}<button
                type="button"
                class="text-emerald-400 hover:underline"
                @click="loadField(catalog.caseDir, time.dirName, 'T')"
              >
                加载 T 场
              </button>
            </td>
            <td class="px-2 py-1 text-zinc-400">{{ time.timeS.toFixed(3) }}</td>
            <td class="px-2 py-1 text-zinc-400">{{ time.fields.join(", ") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="space-y-1">
      <template v-if="stats">
        <p class="text-xs text-zinc-400">{{ stats.line }}</p>
        <p :class="stats.complete ? 'text-xs text-emerald-400' : 'text-xs text-amber-400'">
          {{ stats.complete ? "结果完整" : "结果不完整（求解中途取消）" }}
        </p>
      </template>
    </div>
    <div class="flex items-center gap-2">
      <select v-model="deriveKind" :disabled="loadedField === null" class="flex-1 min-w-0 text-xs">
        <option value="normalize">归一化 (0–1)</option>
        <option value="threshold">阈值掩码（中位幅值）</option>
      </select>
      <UiButton variant="ghost" :disabled="loadedField === null" @click="deriveField">
        生成派生场
      </UiButton>
    </div>
  </Card>
</template>
