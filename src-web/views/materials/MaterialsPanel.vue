<script setup lang="ts">
/** 材料库面板：逻辑见 useMaterialsPanel。 */
import { useMaterialsPanel } from "./useMaterialsPanel";
import Card from "../../components/ui/UiCard.vue";
import UiButton from "../../components/ui/UiButton.vue";
import LatexBlock from "../../components/latex-block/LatexBlock.vue";

const {
  materials,
  selectedId,
  selected,
  CROSS_WLF_TEX,
  TAIT_TEX,
  wlfRows,
  pvtRows,
  cpRows,
  lambdaRows,
  mechanicsRows,
  fillerRows,
  importDisabled,
  exportDisabled,
  copyDisabled,
  useDisabled,
  deleteDisabled,
  doImport,
  doExport,
  doCopy,
  doUse,
  doDelete,
} = useMaterialsPanel();
</script>

<template>
  <Card title="材料库">
    <div class="flex flex-wrap items-center gap-2">
      <UiButton :disabled="importDisabled" @click="doImport">导入 JSON</UiButton>
      <UiButton :disabled="exportDisabled" @click="doExport">导出自定义</UiButton>
      <UiButton :disabled="copyDisabled" @click="doCopy">复制为自定义</UiButton>
      <UiButton :disabled="useDisabled" @click="doUse">用于当前研究</UiButton>
      <UiButton variant="danger" :disabled="deleteDisabled" @click="doDelete">删除</UiButton>
    </div>

    <!-- 左列仅 ~280px：横排三栏会把详情区挤成 0 宽，列表收两列网格、详情整宽在下 -->
    <div class="grid grid-cols-2 gap-2">
      <div class="min-w-0">
        <p class="mb-1 text-xs font-semibold text-zinc-400">内置参考库</p>
        <div class="space-y-1">
          <button
            v-for="material in materials.materials.builtin"
            :key="material.id"
            type="button"
            class="block w-full truncate rounded-lg border border-zinc-700 px-2 py-1.5 text-left text-xs hover:border-emerald-500"
            :class="{ 'border-emerald-500 bg-emerald-500/10': material.id === selectedId }"
            @click="selectedId = material.id"
          >
            {{ material.family }} · {{ material.name }}
          </button>
          <p v-if="materials.materials.builtin.length === 0" class="text-xs text-zinc-500">
            加载中…
          </p>
        </div>
      </div>
      <div class="min-w-0">
        <p class="mb-1 text-xs font-semibold text-zinc-400">自定义</p>
        <div class="space-y-1">
          <button
            v-for="material in materials.materials.custom"
            :key="material.id"
            type="button"
            class="block w-full truncate rounded-lg border border-zinc-700 px-2 py-1.5 text-left text-xs hover:border-emerald-500"
            :class="{ 'border-emerald-500 bg-emerald-500/10': material.id === selectedId }"
            @click="selectedId = material.id"
          >
            {{ material.family }} · {{ material.name }}
          </button>
          <p v-if="materials.materials.custom.length === 0" class="text-xs text-zinc-500">
            无自定义材料。
          </p>
        </div>
      </div>

      <div class="min-w-0 space-y-2">
        <template v-if="selected">
          <p class="text-sm font-semibold text-zinc-200">
            {{ selected.name }}（{{ selected.manufacturer }}）
          </p>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">Cross-WLF 黏度</p>
          <LatexBlock :tex="CROSS_WLF_TEX" label="Cross-WLF 黏度模型公式" />
          <table class="text-xs">
            <tbody>
              <tr v-for="[key, value] in wlfRows" :key="key">
                <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
              </tr>
            </tbody>
          </table>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">Tait PVT</p>
          <LatexBlock :tex="TAIT_TEX" label="Tait PVT 状态方程公式" />
          <table class="text-xs">
            <tbody>
              <tr v-for="[key, value] in pvtRows" :key="key">
                <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
              </tr>
            </tbody>
          </table>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">比热 Cp</p>
          <table class="text-xs">
            <tbody>
              <tr v-for="[key, value] in cpRows" :key="key">
                <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
              </tr>
            </tbody>
          </table>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">导热系数 λ</p>
          <table class="text-xs">
            <tbody>
              <tr v-for="[key, value] in lambdaRows" :key="key">
                <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
              </tr>
            </tbody>
          </table>

          <template v-if="selected.mechanics">
            <p class="text-[11px] font-semibold tracking-wide text-zinc-400">力学（预留）</p>
            <table class="text-xs">
              <tbody>
                <tr v-for="[key, value] in mechanicsRows" :key="key">
                  <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                  <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
                </tr>
              </tbody>
            </table>
          </template>

          <template v-if="fillerRows">
            <p class="text-[11px] font-semibold tracking-wide text-zinc-400">纤维 / 填料</p>
            <table class="text-xs">
              <tbody>
                <tr v-for="[key, value] in fillerRows" :key="key">
                  <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                  <td class="py-0.5 text-zinc-300">{{ value }}</td>
                </tr>
              </tbody>
            </table>
          </template>

          <p class="text-xs text-zinc-500">{{ selected.dataNote }}</p>
        </template>
        <p v-else class="text-xs text-zinc-500">选择左侧材料查看参数。</p>
      </div>
    </div>
  </Card>
</template>
