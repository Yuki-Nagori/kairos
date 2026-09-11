/**
 * 材料库面板：内置参考牌号 + 自定义材料的浏览、详情、导入导出与复制。
 * 详情按 Cross-WLF / Tait PVT / 比热 / 导热分节展示（公式经 KaTeX 渲染）；
 * 选中 id 未命中时回退首个内置材料，删除后同样回落，保证详情与动作始终有目标。
 */
import { computed, ref, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useMaterialsStore } from "../../stores/materials";
import type { Material, PropertyTable } from "../../types";

export function useMaterialsPanel() {
  const app = useAppStore();
  const materials = useMaterialsStore();

  const working = computed(() => app.busy !== null);

  const selectedId = ref<string | null>(null);

  // 目标材料：按选中 id 查找，未命中（初始 / 删除后）回退首个内置材料。
  const selected = computed<Material | undefined>(
    () =>
      [...materials.materials.builtin, ...materials.materials.custom].find(
        (m) => m.id === selectedId.value,
      ) ?? materials.materials.builtin[0],
  );

  // 回退结果同步回选中 id：动作按钮与高亮都以 selectedId 为准。
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

  /** 纤维 / 填料参数组（无填料牌号为 null）。 */
  const fillerRows = computed<[string, string][] | null>(() => {
    const filler = selected.value?.filler;
    if (!filler) {
      return null;
    }
    return [
      ["类型", filler.kind],
      ["质量分数", `${(filler.weightFraction * 100).toFixed(1)} %`],
      ["长径比", String(filler.aspectRatio)],
      ["备注", filler.note],
    ];
  });

  const importDisabled = computed(() => working.value);
  const exportDisabled = computed(() => materials.materials.custom.length === 0 || working.value);
  const copyDisabled = computed(() => !selectedId.value || working.value);
  const useDisabled = computed(() => !selectedId.value || working.value);
  const deleteDisabled = computed(
    () =>
      !selectedId.value ||
      !materials.materials.custom.some((m) => m.id === selectedId.value) ||
      working.value,
  );

  function doImport(): void {
    void materials.importMaterials();
  }
  function doExport(): void {
    void materials.exportMaterials();
  }
  function doCopy(): void {
    if (selectedId.value) {
      void materials.copyMaterialToCustom(selectedId.value);
    }
  }
  function doUse(): void {
    if (selectedId.value) {
      materials.assignMaterial(selectedId.value);
    }
  }
  async function doDelete(): Promise<void> {
    const id = selectedId.value;
    if (!id) {
      return;
    }
    // 删除失败（材料仍在清单）时保留选中，便于用户直接重试；
    // 成功后的回落由 selected 的 watch 自动完成。
    await materials.deleteMaterial(id);
  }

  return {
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
  };
}
