<script setup lang="ts">
/**
 * 几何面板：STL 导入（文件对话框 / 样例）、网格健康摘要与逐几何的
 * 体积网格生成。每个几何行自带目标尺寸 + 引擎（体素 / Gmsh）表单，
 * 初始尺寸取几何最大边的二十分之一；生成动作按引擎分派到对应服务。
 */
import { computed, reactive, watch } from "vue";
import {
  generateGmshMesh,
  generateMesh,
  importGeometry,
  importSampleGeometry,
  removeGeometryById,
  useAppState,
} from "../../state";
import type { GeometrySummary, MeshIssues, MeshingReport } from "../../types";
import Card from "../../components/ui/UiCard.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const state = useAppState();

const working = computed(() => state.busy !== null);

// 每几何行的表单状态（目标尺寸 + 引擎），几何首次出现时以建议尺寸初始化，
// 之后用户编辑独立于渲染保留（列表重建不回填默认值）。
interface MeshFormState {
  size: string;
  engine: string;
}
const meshForms = reactive<Record<string, MeshFormState>>({});

watch(
  () => state.geometries,
  (geometries) => {
    for (const geometry of geometries) {
      if (meshForms[geometry.geometryId] === undefined) {
        meshForms[geometry.geometryId] = {
          size: (Math.max(...geometry.size) / 20).toPrecision(3),
          engine: "voxel",
        };
      }
    }
  },
  { immediate: true },
);

function meshForm(geometry: GeometrySummary): MeshFormState {
  const existing = meshForms[geometry.geometryId];
  if (existing) {
    return existing;
  }
  const created: MeshFormState = {
    size: (Math.max(...geometry.size) / 20).toPrecision(3),
    engine: "voxel",
  };
  meshForms[geometry.geometryId] = created;
  return created;
}

const rows = computed(() =>
  state.geometries.map((geometry) => ({ geometry, form: meshForm(geometry) })),
);

function issueText(issues: MeshIssues): string {
  const parts: string[] = [];
  if (issues.openEdges > 0) {
    parts.push(`开放边 ${issues.openEdges}`);
  }
  if (issues.degenerate > 0) {
    parts.push(`退化三角形 ${issues.degenerate}`);
  }
  if (issues.nonManifoldEdges > 0) {
    parts.push(`非流形边 ${issues.nonManifoldEdges}`);
  }
  if (issues.normalInconsistentEdges > 0) {
    parts.push(`法向不一致 ${issues.normalInconsistentEdges}`);
  }
  return parts.length > 0 ? parts.join("，") : "网格健康";
}

function isClean(geometry: GeometrySummary): boolean {
  const issues = geometry.issues;
  return (
    issues.degenerate === 0 &&
    issues.openEdges === 0 &&
    issues.nonManifoldEdges === 0 &&
    issues.normalInconsistentEdges === 0
  );
}

function statsText(geometry: GeometrySummary): string {
  return `${geometry.triangleCount} 三角形 · ${geometry.size
    .map((value) => value.toFixed(2))
    .join(" × ")} ${geometry.suggestedUnit}`;
}

function generate(geometry: GeometrySummary): void {
  const form = meshForm(geometry);
  const size = Number(form.size);
  void (form.engine === "gmsh"
    ? generateGmshMesh(geometry.geometryId, size)
    : generateMesh(geometry.geometryId, size));
}

function reportText(report: MeshingReport | undefined): string {
  if (!report) {
    return "划分体积网格供求解使用。";
  }
  return `节点 ${report.nodeCount} · 四面体 ${report.elementCount} · 表面 ${report.surfaceFaceCount} · 体积 ${report.totalVolume.toFixed(3)} · 质量比 min ${report.quality.minEdgeRatio.toFixed(2)} / avg ${report.quality.avgEdgeRatio.toFixed(2)} / max ${report.quality.maxEdgeRatio.toFixed(2)}`;
}

function onImport(): void {
  void importGeometry();
}
function onSample(): void {
  void importSampleGeometry(10);
}
</script>

<template>
  <Card title="几何">
    <div class="flex flex-wrap items-center gap-2">
      <UiButton variant="primary" :disabled="working" @click="onImport">导入 STL</UiButton>
      <UiButton @click="onSample">导入样例</UiButton>
    </div>

    <div class="space-y-2">
      <p v-if="state.geometries.length === 0" class="text-xs text-zinc-500">
        尚未导入几何。支持二进制 / ASCII STL。
      </p>
      <div
        v-for="row in rows"
        :key="row.geometry.geometryId"
        class="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs"
      >
        <span class="font-semibold text-zinc-200">{{ row.geometry.fileName }}</span>
        <span class="text-zinc-500">{{ statsText(row.geometry) }}</span>
        <p :class="isClean(row.geometry) ? 'text-emerald-400' : 'text-amber-400'">
          {{ issueText(row.geometry.issues) }}
        </p>
        <UiButton
          variant="danger"
          :disabled="working"
          @click="removeGeometryById(row.geometry.geometryId)"
        >
          移除
        </UiButton>

        <!-- 网格生成区：目标尺寸输入 + 引擎选择 + 生成按钮 + 报告 -->
        <div class="flex w-full flex-wrap items-center gap-2 border-t border-zinc-800 pt-2">
          <TextInput
            v-model="row.form.size"
            type="number"
            placeholder="目标尺寸"
            class="w-28"
            step="any"
            min="0"
          />
          <select v-model="row.form.engine" class="text-xs">
            <option value="voxel">体素</option>
            <option value="gmsh">Gmsh</option>
          </select>
          <UiButton :disabled="working" @click="generate(row.geometry)">生成体积网格</UiButton>
          <p class="text-xs text-zinc-500">
            {{ reportText(state.meshReports[row.geometry.geometryId]) }}
          </p>
        </div>
      </div>
    </div>
  </Card>
</template>
