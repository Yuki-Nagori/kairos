/** 结果面板：扫描 OpenFOAM case 结果目录、查看时间步与场统计；三维视口由视口面板负责。 */
import { computed, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useResultsStore } from "../../stores/results";
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
    results.loadedField = {
      ...source,
      field: `${source.field} · ${suffix}`,
      isMagnitude: false,
      values: derived,
      complete: source.complete,
    };
  }

  return { results, dirPath, catalog, loadedField, stats, scan, deriveKind, deriveField };
}
