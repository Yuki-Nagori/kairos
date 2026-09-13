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

  // —— 矢量场三分量（变形显示 / 矢量派生用）——
  const vectorFieldName = ref("D");
  const vectorField = computed(() => results.vectorField);
  /** 矢量统计行：单元数 + 首个单元分量 + 模量极值（无数据时为 null）。 */
  const vectorStats = computed(() => {
    const field = results.vectorField;
    if (field === null || field.components.length === 0) {
      return null;
    }
    const first = field.components[0] as [number, number, number];
    const norms = field.components.map((group) => Math.hypot(group[0], group[1], group[2]));
    const { min, max } = minMax(norms);
    return {
      line: `矢量 ${field.field} @ ${field.timeDir}：${field.components.length} 个单元 · 首单元 (${first.map((value) => value.toExponential(2)).join(", ")}) · |v| ${min.toExponential(2)} ~ ${max.toExponential(2)}`,
      complete: field.complete,
    };
  });
  const vectorDisabled = computed(() => results.resultCatalog === null);

  /** 加载矢量场：时间步取当前已加载场（没有就用最后一个时间步）。 */
  function loadVector(): void {
    const catalog = results.resultCatalog;
    if (catalog === null || catalog.times.length === 0) {
      return;
    }
    const loadedTime = results.loadedField?.timeDir;
    const timeDir =
      catalog.times.find((step) => step.dirName === loadedTime)?.dirName ??
      catalog.times.at(-1)!.dirName;
    void results.loadVectorComponents(
      catalog.caseDir,
      timeDir,
      vectorFieldName.value.trim() || "D",
    );
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
    vectorField,
    vectorFieldName,
    vectorStats,
    vectorDisabled,
    loadVector,
  };
}
