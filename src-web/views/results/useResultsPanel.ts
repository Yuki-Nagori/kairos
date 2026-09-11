/** 结果面板：扫描 OpenFOAM case 结果目录、查看时间步与场统计；三维视口由视口面板负责。 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
import type { DeriveRequest, FieldSlot } from "../../types";
import { minMax } from "../../utils/stats";

export function useResultsPanel() {
  const app = useAppStore();
  const results = useResultsStore();

  const dirPath = ref("");
  const working = computed(() => app.busy !== null);
  const catalog = computed(() => results.resultCatalog);
  const loadedField = computed(() => results.loadedField);

  // Card 的刷新按钮不外联 disabled，改在扫描函数内守卫 busy。
  function scan(): void {
    if (working.value) {
      return;
    }
    const caseDir = dirPath.value.trim();
    if (caseDir) {
      void results.loadResultsCatalog(caseDir);
    }
  }

  // 场统计行：空场（0 个值）以 NaN 占位，仍按数值格式渲染。
  const stats = computed(() => {
    const field = results.loadedField;
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

  // 场加载槽位：主场供展示与单场派生，对比场供两场差值。
  const loadSlot = ref<FieldSlot>("primary");

  // 派生场：数值变换在 Rust 侧完成（derive_scalar_field / derive_difference），
  // 此处只收集算子类型与线性参数。
  const deriveKind = ref<"normalize" | "threshold" | "linear">("normalize");
  const linearScale = ref("1");
  const linearOffset = ref("0");

  function buildDeriveRequest(): DeriveRequest {
    if (deriveKind.value === "linear") {
      return {
        kind: "linear",
        scale: Number(linearScale.value) || 1,
        offset: Number(linearOffset.value) || 0,
      };
    }
    return { kind: deriveKind.value };
  }

  function deriveField(): void {
    const source = loadedField.value;
    if (!source || source.values.length === 0) {
      return;
    }
    void results.deriveField(buildDeriveRequest());
  }

  function deriveDifference(): void {
    void results.deriveDifference();
  }

  return {
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
  };
}
