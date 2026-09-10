<script setup lang="ts">
// 逻辑全部抽至同目录 useResultsPanel.ts，此处仅保留模板与解构。
import { useResultsPanel } from "./useResultsPanel";
import UiButton from "../../components/ui/UiButton.vue";
import Card from "../../components/ui/UiCard.vue";
import UiTextInput from "../../components/ui/UiTextInput.vue";

const { results, dirPath, catalog, loadedField, stats, scan, deriveKind, deriveField } =
  useResultsPanel();
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
                @click="results.loadField(catalog.caseDir, time.dirName, 'T')"
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
