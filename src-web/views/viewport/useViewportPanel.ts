/** 3D 视口面板：WebGL2 渲染与云图 / 剖切 / 时间步动画控制（WebGPU 探测提示）。
 * 视口是工作台主角：卡片弹性充满中列剩余空间，画布随容器缩放。 */
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { useGeometryStore } from "../../stores/geometry";
import { useProjectStore } from "../../stores/project";
import { useResultsStore } from "../../stores/results";
import { useViewportStore } from "../../stores/viewport";
import type { ScalarField } from "../../types";
import { ViewportRenderer } from "../../render/renderer";
import { detectRenderCapabilityInBrowser } from "../../render/capability";
import { registerSnapshot } from "../../render/snapshot";
import { minMax } from "../../utils/stats";
import { getRenderMesh } from "../../api/geometry";

/** 叠加层配色：浇口红、流道琥珀、冷却水路蓝（深色视口下高对比）。 */
const GATE_COLOR: [number, number, number] = [0.95, 0.32, 0.3];
const RUNNER_COLOR: [number, number, number] = [0.95, 0.62, 0.12];
const COOLING_COLOR: [number, number, number] = [0.3, 0.6, 0.95];

/** 端点对压平成线段坐标（长度 = 6 × 段数）。 */
function flattenSegments(
  elements: { start: [number, number, number]; end: [number, number, number] }[],
): Float32Array {
  const positions = new Float32Array(elements.length * 6);
  elements.forEach((element, index) => {
    positions.set(element.start, index * 6);
    positions.set(element.end, index * 6 + 3);
  });
  return positions;
}

export function useViewportPanel() {
  const geometry = useGeometryStore();
  const results = useResultsStore();
  const project = useProjectStore();
  const viewport = useViewportStore();

  // 模板 ref 经 useTemplateRef 按名绑定（静态 ref="canvasRef" 不算 setup 变量读取，
  // 解构返回会触发 noUnusedLocals）。
  const canvasRef = useTemplateRef<HTMLCanvasElement>("canvasRef");

  // 空态提示：载入网格前视口不应是一片空白；WebGPU 探测备注与失败文案动态替换。
  const emptyText = ref(
    "导入几何并生成网格后，点击「载入网格到视口」查看 3D 模型（WebGPU 可用时自动启用）",
  );
  const emptyError = ref(false);
  const meshLoaded = ref(false);

  // 悬浮色标图例：数值自上而下 max/mid/min，与渐变条方向对应。
  const legendValues = ref<number[] | null>(null);
  const legendVisible = computed(() => legendValues.value !== null);

  function updateLegend(field: { values: number[] } | null): void {
    if (field === null || field.values.length === 0) {
      legendValues.value = null;
      return;
    }
    const sorted = [...field.values].sort((a, b) => a - b);
    const max = sorted[sorted.length - 1] ?? 0;
    const mid = sorted[Math.floor(sorted.length / 2)] ?? 0;
    const min = sorted[0] ?? 0;
    legendValues.value = [max, mid, min];
  }

  // —— 控制条 ——
  // 启用条件按数据就绪度推导：
  // 载入需已有几何；剖切/重置/播放需网格已上传；播放另需结果时间步目录。
  const loadDisabled = computed(() => geometry.geometries.length === 0);
  const meshReady = ref(false);
  const playDisabled = computed(() => {
    const catalog = results.resultCatalog;
    return !meshReady.value || catalog === null || catalog.times.length === 0;
  });
  const fpsText = ref("FPS: —");
  const clipOn = ref(false);
  const playing = ref(false);
  const playLabel = computed(() => (playing.value ? "停止动画" : "播放动画"));

  let renderer: ViewportRenderer | null = null;
  let renderMesh: { positions: Float32Array; indices: Uint32Array; faceCells: Uint32Array } | null =
    null;
  let playTimer: ReturnType<typeof setInterval> | null = null;
  let playIndex = 0;

  // 相机注视点读数（模型坐标）：旋转 / 平移 / 缩放 / 复位时由渲染器回报。
  const viewCenter = ref({ x: 0, y: 0, z: 0 });
  const centerText = computed(
    () =>
      `X ${viewCenter.value.x.toFixed(1)} · Y ${viewCenter.value.y.toFixed(1)} · Z ${viewCenter.value.z.toFixed(1)}`,
  );

  // 视口标题：当前几何与研究（未载入网格时不显示）。
  const title = computed(() => {
    if (!meshLoaded.value) {
      return "";
    }
    const fileName = geometry.geometries[0]?.fileName ?? "—";
    return `${fileName} · ${project.activeStudy?.name ?? "未选择方案"}`;
  });

  function stopPlay(): void {
    playing.value = false;
    if (playTimer !== null) {
      clearInterval(playTimer);
      playTimer = null;
    }
  }

  function startPlay(): void {
    updateLegend(results.loadedField);
    const catalog = results.resultCatalog;
    if (catalog === null || catalog.times.length === 0 || renderer === null) {
      return;
    }
    const caseDir = catalog.caseDir;
    const fieldName = results.loadedField?.field ?? "T";
    playing.value = true;
    playIndex = 0;
    playTimer = setInterval(() => {
      if (!playing.value) {
        stopPlay();
        return;
      }
      const step = catalog.times[playIndex % catalog.times.length]!;
      playIndex += 1;
      void results.loadField(caseDir, step.dirName, fieldName).then(() => {
        applyField(results.loadedField);
      });
    }, 400);
  }

  function togglePlay(): void {
    if (playing.value) {
      stopPlay();
    } else {
      startPlay();
    }
  }

  function toggleClip(): void {
    clipOn.value = !clipOn.value;
    renderer?.setClip(clipOn.value, 0);
  }

  function resetView(): void {
    renderer?.resetView();
  }

  function zoomBy(factor: number): void {
    renderer?.zoomBy(factor);
  }

  function fitView(): void {
    renderer?.fitView();
  }

  function ensureRenderer(): void {
    if (renderer !== null) {
      return;
    }
    const canvas = canvasRef.value;
    if (canvas === null) {
      return;
    }
    renderer = ViewportRenderer.create(
      canvas,
      (fps) => {
        fpsText.value = `FPS: ${fps}`;
      },
      (state) => {
        viewCenter.value = state;
      },
    );
    if (renderer === null) {
      emptyText.value = "当前环境不支持 WebGL2，无法渲染视口。";
      emptyError.value = true;
    }
  }

  function loadMesh(): void {
    ensureRenderer();
    const first = geometry.geometries[0];
    if (first === undefined || renderer === null) {
      return;
    }
    void getRenderMesh(first.geometryId).then((data) => {
      renderMesh = {
        positions: new Float32Array(data.positions),
        indices: new Uint32Array(data.indices),
        faceCells: new Uint32Array(data.faceCells),
      };
      renderer?.uploadMesh({
        positions: renderMesh.positions,
        indices: renderMesh.indices,
        faceCells: renderMesh.faceCells,
      });
      uploadOverlays();
      applyLayerVisibility();
      meshLoaded.value = true;
      meshReady.value = true;
    });
  }

  /** 把当前研究的浇注系统 / 冷却水路上传为线段叠加层。 */
  function uploadOverlays(): void {
    const study = project.activeStudy;
    if (study === null || renderer === null) {
      return;
    }
    const gates = study.runnerElements.filter((element) => element.kind === "gate");
    const runners = study.runnerElements.filter((element) => element.kind === "runner");
    renderer.uploadOverlay("gates", { positions: flattenSegments(gates), color: GATE_COLOR });
    renderer.uploadOverlay("runners", { positions: flattenSegments(runners), color: RUNNER_COLOR });
    renderer.uploadOverlay("cooling", {
      positions: flattenSegments(study.coolingChannels),
      color: COOLING_COLOR,
    });
  }

  /** 把图层可见性意图同步到渲染器（层管理面板与视口的桥）。 */
  function applyLayerVisibility(): void {
    if (renderer === null) {
      return;
    }
    renderer.setMeshVisible(viewport.layers.mesh);
    renderer.setOverlayVisible("gates", viewport.layers.gates);
    renderer.setOverlayVisible("runners", viewport.layers.runners);
    renderer.setOverlayVisible("cooling", viewport.layers.cooling);
  }

  watch(
    () => viewport.layers,
    () => applyLayerVisibility(),
  );

  // 场数据加载后自动开启云图着色（值域取自场 min/max），并热更新每面值。
  function applyField(field: ScalarField | null): void {
    if (field === null || renderer === null || field.values.length === 0) {
      return;
    }
    if (renderMesh !== null) {
      const perFace = new Float32Array(renderMesh.faceCells.length);
      for (let face = 0; face < perFace.length; face += 1) {
        perFace[face] = field.values[renderMesh.faceCells[face] ?? 0] ?? 0;
      }
      renderer.setFaceValues(perFace);
    }
    const { min, max } = minMax(field.values);
    renderer.setFieldRange(min, max);
  }

  watch(
    () => results.loadedField,
    (field) => {
      updateLegend(field);
      applyField(field);
    },
  );

  onMounted(() => {
    const canvas = canvasRef.value;
    if (canvas !== null) {
      registerSnapshot("viewport", canvas);
    }
    void detectRenderCapabilityInBrowser().then((capability) => {
      if (capability.backend === "webgl2") {
        emptyText.value = `${emptyText.value}（${capability.note}）`;
      }
    });
  });
  onUnmounted(stopPlay);

  return {
    emptyText,
    emptyError,
    meshLoaded,
    legendVisible,
    legendValues,
    loadDisabled,
    meshReady,
    playDisabled,
    playLabel,
    fpsText,
    clipOn,
    title,
    centerText,
    loadMesh,
    togglePlay,
    toggleClip,
    resetView,
    zoomBy,
    fitView,
  };
}
