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
  vectorField,
  vectorFieldName,
  vectorStats,
  vectorDisabled,
  loadVector,
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
    <hr class="border-zinc-800" />
    <div class="flex flex-wrap items-center gap-2">
      <p class="text-[11px] font-semibold tracking-wide text-zinc-400">矢量场三分量</p>
      <UiTextInput
        v-model="vectorFieldName"
        class="w-20"
        placeholder="D"
        title="矢量场名（如 D 位移 / U 速度）"
      />
      <UiButton
        variant="ghost"
        :disabled="vectorDisabled"
        title="加载三分量（变形显示与矢量派生使用）"
        @click="loadVector"
      >
        加载矢量场
      </UiButton>
      <p v-if="vectorStats === null" class="text-xs text-zinc-500">
        未加载矢量场（位移 D / 速度 U 等三分量场）。
      </p>
      <p v-else-if="!vectorStats.complete" class="text-xs text-amber-400">
        {{ vectorStats.line }}（不完整）
      </p>
      <p v-else class="text-xs text-zinc-300">{{ vectorStats.line }}</p>
      <p v-if="vectorField?.complete === false" class="text-[10px] text-amber-400">
        矢量数据不完整：求解中途取消的结果。
      </p>
    </div>
  </Card>
</template>
