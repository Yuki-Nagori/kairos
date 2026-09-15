/** 结果面板：扫描 OpenFOAM case 结果目录、查看时间步与场统计；三维视口由视口面板负责。 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
import type { DeriveRequest, FieldSlot } from "../../types";
import { minMax } from "../../utils/stats";
import { significant } from "../../utils/format";
export function useResultsPanel() {
  const app = useAppStore();
  const results = useResultsStore();

  const dirPath = ref("");
  const working = computed(() => app.working);
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
    const remote = results.loadedFieldStats;
    const { min, max, count } =
      remote ??
      (values.length > 0
        ? { min: minMax(values).min, max: minMax(values).max, count: values.length }
        : { min: Number.NaN, max: Number.NaN, count: 0 });
    return {
      line: `已加载 ${field.field} @ ${field.timeDir}${field.isMagnitude ? "（模量）" : ""}：${count} 个值，min ${significant(min)} / max ${significant(max)}`,
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
      line: `矢量 ${field.field} @ ${field.timeDir}：${field.components.length} 个单元 · 首单元 (${first.map((value) => significant(value)).join(", ")}) · |v| ${significant(min)} ~ ${significant(max)}`,
      complete: field.complete,
    };
  });
  const vectorDisabled = computed(() => results.resultCatalog === null);

  // —— 对称张量场（残余应力 / 取向张量）——
  const tensorFieldName = ref("sigma");
  const tensorStats = computed(() => {
    const field = results.tensorField;
    if (field === null || field.magnitudes.length === 0) {
      return null;
    }
    const { min, max } = minMax(field.magnitudes);
    const axis = field.principalAxes[0] as [number, number, number];
    return {
      line: `张量 ${field.field} @ ${field.timeDir}：${field.magnitudes.length} 个单元 · |σ| ${significant(min)} ~ ${significant(max)} · 首单元主轴 (${axis.map((value) => significant(value)).join(", ")})`,
      complete: field.complete,
    };
  });

  /** 目标时间步：沿用当前已加载场的时间步，没有就用最后一个（张量 / 矢量同一取法）。 */
  function targetTimeDir(): string | null {
    const catalog = results.resultCatalog;
    if (catalog === null || catalog.times.length === 0) {
      return null;
    }
    const loadedTime = results.loadedField?.timeDir;
    return (
      catalog.times.find((step) => step.dirName === loadedTime)?.dirName ??
      catalog.times.at(-1)!.dirName
    );
  }

  /** 加载张量场（残余应力 / 取向张量）。 */
  function loadTensor(): void {
    const caseDir = results.resultCatalog?.caseDir;
    const timeDir = targetTimeDir();
    if (caseDir === undefined || timeDir === null) {
      return;
    }
    void results.loadTensorComponents(caseDir, timeDir, tensorFieldName.value.trim() || "sigma");
  }

  /** 加载矢量场（位移 / 速度三分量）。 */
  function loadVector(): void {
    const caseDir = results.resultCatalog?.caseDir;
    const timeDir = targetTimeDir();
    if (caseDir === undefined || timeDir === null) {
      return;
    }
    void results.loadVectorComponents(caseDir, timeDir, vectorFieldName.value.trim() || "D");
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
    tensorFieldName,
    tensorStats,
    loadTensor,
  };
}
