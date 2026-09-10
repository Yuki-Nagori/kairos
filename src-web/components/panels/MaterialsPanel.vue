<script setup lang="ts">
/**
 * 材料库面板：内置示例材料 + 自定义材料的浏览、详情、导入导出与复制。
 * 详情按 Cross-WLF / Tait PVT / 比热 / 导热分节展示（公式经 KaTeX 渲染）；
 * 选中 id 未命中时回退首个内置材料，删除后同样回落，保证详情与动作始终有目标。
 */
import { computed, ref, watch } from "vue";
import {
  assignMaterial,
  copyMaterialToCustom,
  deleteMaterial,
  exportMaterials,
  importMaterials,
  useAppState,
} from "../../state";
import type { Material, PropertyTable } from "../../types";
import Card from "../ui/Card.vue";
import UiButton from "../ui/Button.vue";
import LatexBlock from "../LatexBlock.vue";

const state = useAppState();

const working = computed(() => state.busy !== null);

const selectedId = ref<string | null>(null);

// 目标材料：按选中 id 查找，未命中（初始 / 删除后）回退首个内置材料。
const selected = computed<Material | undefined>(
  () =>
    [...state.materials.builtin, ...state.materials.custom].find(
      (m) => m.id === selectedId.value,
    ) ?? state.materials.builtin[0],
);

// 回退结果同步回选中 id：动作按钮与高亮都以 selectedId 为准（与原 render 同步一致）。
watch(selected, (material) => {
  if (material && selectedId.value !== material.id) {
    selectedId.value = material.id;
  }
});

const CROSS_WLF_TEX =
  "\\eta = \\dfrac{\\eta_0}{1 + \\left(\\dfrac{\\eta_0\\dot\\gamma}{\\tau^*}\\right)^{1-n}}, \\quad \\eta_0 = D_1 e^{-\\frac{A_1(T-T^*)}{A_2+(T-T^*)}}, \\quad T^* = D_2 + D_3 p";
const TAIT_TEX =
  "\\hat{v} = v_0(T)\\left[1 - C\\ln\\left(1 + \\frac{p}{B(T)}\\right)\\right], \\quad B(T) = b_3 e^{-b_4 T}, \\quad T_t = b_5 + b_6 p";

const wlfRows = computed<[string, string][]>(() => {
  const rheology = selected.value?.rheology;
  if (!rheology) {
    return [];
  }
  return [
    ["n", String(rheology.n)],
    ["τ*", `${rheology.tauStar} Pa`],
    ["D1", `${rheology.d1} Pa·s`],
    ["D2", `${rheology.d2} K`],
    ["D3", `${rheology.d3} K/Pa`],
    ["A1", String(rheology.a1)],
    ["A2", `${rheology.a2} K`],
  ];
});

const pvtRows = computed<[string, string][]>(() => {
  const pvt = selected.value?.pvt;
  if (!pvt) {
    return [];
  }
  return [
    ["b1m", `${pvt.b1m} m³/kg`],
    ["b1s", `${pvt.b1s} m³/kg`],
    ["b2m", `${pvt.b2m} m³/(kg·K)`],
    ["b2s", `${pvt.b2s} m³/(kg·K)`],
    ["b3", `${pvt.b3} Pa`],
    ["b4m", `${pvt.b4m} 1/K`],
    ["b4s", `${pvt.b4s} 1/K`],
    ["b5", `${pvt.b5} K`],
  ];
});

// 温度相关的性质表统一转成「温度 K | 值 单位」的键值行。
function valueRows(table: PropertyTable, unit: string): [string, string][] {
  return table.map(([temperature, value]) => [`${temperature} K`, `${value} ${unit}`]);
}

const cpRows = computed(() =>
  selected.value ? valueRows(selected.value.specificHeat, "J/(kg·K)") : [],
);
const lambdaRows = computed(() =>
  selected.value ? valueRows(selected.value.conductivity, "W/(m·K)") : [],
);

const mechanicsRows = computed<[string, string][]>(() => {
  const mechanics = selected.value?.mechanics;
  if (!mechanics) {
    return [];
  }
  return [
    ["E", `${mechanics.elasticModulus} Pa`],
    ["ν", String(mechanics.poissonRatio)],
  ];
});

const importDisabled = computed(() => working.value);
const exportDisabled = computed(() => state.materials.custom.length === 0 || working.value);
const copyDisabled = computed(() => !selectedId.value || working.value);
const deleteDisabled = computed(
  () =>
    !selectedId.value ||
    !state.materials.custom.some((m) => m.id === selectedId.value) ||
    working.value,
);

function doImport(): void {
  void importMaterials();
}
function doExport(): void {
  void exportMaterials();
}
function doCopy(): void {
  if (selectedId.value) {
    void copyMaterialToCustom(selectedId.value);
  }
}
function doUse(): void {
  if (selectedId.value) {
    assignMaterial(selectedId.value);
  }
}
function doDelete(): void {
  if (selectedId.value) {
    void deleteMaterial(selectedId.value);
    selectedId.value = null;
  }
}
</script>

<template>
  <Card title="材料库">
    <div class="flex flex-wrap items-center gap-2">
      <UiButton :disabled="importDisabled" @click="doImport">导入 JSON</UiButton>
      <UiButton :disabled="exportDisabled" @click="doExport">导出自定义</UiButton>
      <UiButton :disabled="copyDisabled" @click="doCopy">复制为自定义</UiButton>
      <UiButton @click="doUse">用于当前研究</UiButton>
      <UiButton variant="danger" :disabled="deleteDisabled" @click="doDelete">删除</UiButton>
    </div>

    <div class="flex gap-4">
      <div>
        <p class="mb-1 text-xs font-semibold text-zinc-400">内置示例</p>
        <div class="w-44 shrink-0 space-y-1">
          <button
            v-for="material in state.materials.builtin"
            :key="material.id"
            type="button"
            class="block w-full truncate rounded-lg border border-zinc-700 px-2 py-1.5 text-left text-xs hover:border-emerald-500"
            :class="{ 'border-emerald-500 bg-emerald-500/10': material.id === selectedId }"
            @click="selectedId = material.id"
          >
            {{ material.family }} · {{ material.name }}
          </button>
          <p v-if="state.materials.builtin.length === 0" class="text-xs text-zinc-500">加载中…</p>
        </div>
      </div>
      <div>
        <p class="mb-1 text-xs font-semibold text-zinc-400">自定义</p>
        <div class="w-44 shrink-0 space-y-1">
          <button
            v-for="material in state.materials.custom"
            :key="material.id"
            type="button"
            class="block w-full truncate rounded-lg border border-zinc-700 px-2 py-1.5 text-left text-xs hover:border-emerald-500"
            :class="{ 'border-emerald-500 bg-emerald-500/10': material.id === selectedId }"
            @click="selectedId = material.id"
          >
            {{ material.family }} · {{ material.name }}
          </button>
          <p v-if="state.materials.custom.length === 0" class="text-xs text-zinc-500">
            无自定义材料。
          </p>
        </div>
      </div>

      <div class="min-w-0 flex-1 space-y-2">
        <template v-if="selected">
          <p class="text-sm font-semibold text-zinc-200">
            {{ selected.name }}（{{ selected.manufacturer }}）
          </p>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">Cross-WLF 黏度</p>
          <LatexBlock :tex="CROSS_WLF_TEX" />
          <table class="text-xs">
            <tbody>
              <tr v-for="[key, value] in wlfRows" :key="key">
                <td class="pr-4 py-0.5 text-zinc-500">{{ key }}</td>
                <td class="py-0.5 font-mono text-zinc-300">{{ value }}</td>
              </tr>
            </tbody>
          </table>

          <p class="text-[11px] font-semibold tracking-wide text-zinc-400">Tait PVT</p>
          <LatexBlock :tex="TAIT_TEX" />
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

          <p class="text-xs text-zinc-500">{{ selected.dataNote }}</p>
        </template>
        <p v-else class="text-xs text-zinc-500">选择左侧材料查看参数。</p>
      </div>
    </div>
  </Card>
</template>
