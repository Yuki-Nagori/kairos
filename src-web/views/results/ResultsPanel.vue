<script setup lang="ts">
/** 结果面板：逻辑见 useResultsPanel。 */
import { useResultsPanel } from "./useResultsPanel";
import UiButton from "../../components/ui/UiButton.vue";
import Card from "../../components/ui/UiCard.vue";
import UiTextInput from "../../components/ui/UiTextInput.vue";

const {
  results,
  dirPath,
  catalog,
  loadedField,
  stats,
  scan,
  loadSlot,
  deriveKind,
  linearScale,
  linearOffset,
  deriveField,
  deriveDifference,
} = useResultsPanel();
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
                @click="results.loadField(catalog.caseDir, time.dirName, 'T', loadSlot)"
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
      <select v-model="loadSlot" class="text-xs">
        <option value="primary">加载到主场</option>
        <option value="compare">加载到对比场</option>
      </select>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <select v-model="deriveKind" :disabled="loadedField === null" class="flex-1 min-w-0 text-xs">
        <option value="normalize">归一化 (0–1)</option>
        <option value="threshold">阈值掩码（中点）</option>
        <option value="linear">线性映射 (×scale + offset)</option>
      </select>
      <template v-if="deriveKind === 'linear'">
        <UiTextInput
          v-model="linearScale"
          type="number"
          placeholder="scale"
          class="w-20"
          step="any"
        />
        <UiTextInput
          v-model="linearOffset"
          type="number"
          placeholder="offset"
          class="w-20"
          step="any"
        />
      </template>
      <UiButton variant="ghost" :disabled="loadedField === null" @click="deriveField">
        生成派生场
      </UiButton>
    </div>
    <div class="flex items-center gap-2">
      <UiButton
        variant="ghost"
        :disabled="loadedField === null || results.compareField === null"
        title="主场 − 对比场（两者均已加载时可用）"
        @click="deriveDifference"
      >
        两场差值
      </UiButton>
      <p class="text-xs text-zinc-500">
        {{
          results.compareField === null
            ? "尚未加载对比场。"
            : `对比场：${results.compareField.field} @ ${results.compareField.timeDir}`
        }}
      </p>
    </div>
  </Card>
</template>
