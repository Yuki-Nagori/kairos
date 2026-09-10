import { appStore, loadField } from "../../state";
import type { ScalarField } from "../../types";
import { ViewportRenderer } from "../../render/renderer";
import { detectRenderCapabilityInBrowser } from "../../render/capability";
import { registerSnapshot } from "../../render/snapshot";
import { minMax } from "../../lib/stats";
import { getRenderMesh } from "../../services/geometry";
import { button, card, hint } from "../ui";

/** 3D 视口面板：WebGL2 渲染器 + 云图/剖切/时间步动画控制（WebGPU 探测提示）。 */
export function createViewportPanel(): HTMLElement {
  const { root, body } = card("3D 视口");
  // 视口是工作台主角：卡片弹性充满中列剩余空间，画布随容器缩放。
  root.classList.add("flex", "min-h-[280px]", "flex-1", "flex-col", "overflow-hidden");
  body.classList.add("flex", "min-h-0", "flex-1", "flex-col");

  const canvasWrap = document.createElement("div");
  canvasWrap.className = "relative flex min-h-0 flex-1";
  const canvas = document.createElement("canvas");
  // 绘制缓冲由渲染器按 CSS 尺寸 + DPR 维护，这里只负责铺满容器。
  canvas.className = "h-full w-full rounded-lg bg-zinc-950";
  registerSnapshot("viewport", canvas);
  canvas.style.touchAction = "none";
  // 空态提示：载入网格前视口不应是一片空白
  const emptyHint = document.createElement("p");
  emptyHint.className =
    "pointer-events-none absolute inset-0 flex items-center justify-center text-xs text-zinc-600";
  emptyHint.textContent =
    "导入几何并生成网格后，点击「载入网格到视口」查看 3D 模型（WebGPU 可用时自动启用）";
  canvasWrap.append(canvas, emptyHint);
  // 悬浮色标图例：加载场后显示渐变标尺与 min/max 值（v2 设计稿对齐）。
  const legend = document.createElement("div");
  legend.className =
    "absolute left-3 top-3 z-10 flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-950/80 px-2 py-1.5";
  const legendScale = document.createElement("div");
  legendScale.className = "h-16 w-2 rounded-sm";
  legendScale.style.background = "linear-gradient(180deg, #f59e0b, #22c55e, #3b82f6)";
  const legendLabels = document.createElement("div");
  legendLabels.className = "flex h-16 flex-col justify-between font-mono text-[10px] text-zinc-400";
  legend.append(legendScale, legendLabels);
  legend.style.display = "none";
  canvasWrap.append(legend);

  // 色标数值随加载场更新（max / mid / min 自上而下）
  function updateLegend(field: { values: number[] } | null): void {
    if (field === null || field.values.length === 0) {
      legend.style.display = "none";
      return;
    }
    legend.style.display = "flex";
    const sorted = [...field.values].sort((a, b) => a - b);
    const max = sorted[sorted.length - 1] ?? 0;
    const mid = sorted[Math.floor(sorted.length / 2)] ?? 0;
    const min = sorted[0] ?? 0;
    const values = [max, mid, min];
    legendLabels.replaceChildren(
      ...values.map((v) => {
        const s = document.createElement("span");
        s.textContent = v.toFixed(2);
        return s;
      }),
    );
  }

  const controls = document.createElement("div");
  controls.className = "flex shrink-0 flex-wrap items-center gap-2";
  const loadMeshButton = button("载入网格到视口");
  const playButton = button("播放动画");
  playButton.disabled = true;
  const clipToggle = button("剖切：关");
  const resetButton = button("重置视角");
  const fpsLabel = hint("FPS: —");
  controls.append(loadMeshButton, playButton, clipToggle, resetButton, fpsLabel);

  // —— 时间步动画（T23）：遍历结果目录逐帧加载并热更新值缓冲 ——
  let playing = false;
  let playTimer: ReturnType<typeof setInterval> | null = null;
  let playIndex = 0;

  function stopPlay(): void {
    playing = false;
    if (playTimer !== null) {
      clearInterval(playTimer);
      playTimer = null;
    }
    playButton.textContent = "播放动画";
  }

  function startPlay(): void {
    const { resultCatalog, loadedField } = appStore.get();
    updateLegend(loadedField);
    if (resultCatalog === null || resultCatalog.times.length === 0 || renderer === null) {
      return;
    }
    const caseDir = resultCatalog.caseDir;
    const fieldName = loadedField?.field ?? "T";
    playing = true;
    playButton.textContent = "停止动画";
    playIndex = 0;
    playTimer = setInterval(() => {
      if (!playing) {
        stopPlay();
        return;
      }
      const step = resultCatalog.times[playIndex % resultCatalog.times.length]!;
      playIndex += 1;
      void loadField(caseDir, step.dirName, fieldName).then(() => {
        applyField(appStore.get().loadedField);
      });
    }, 400);
  }

  playButton.addEventListener("click", () => {
    if (playing) {
      stopPlay();
    } else {
      startPlay();
    }
  });

  let renderer: ViewportRenderer | null = null;
  let renderMesh: { positions: Float32Array; indices: Uint32Array; faceCells: Uint32Array } | null =
    null;
  let clipOn = false;

  loadMeshButton.disabled = true;
  clipToggle.disabled = true;
  resetButton.disabled = true;

  function ensureRenderer(): void {
    if (renderer !== null) {
      return;
    }
    renderer = ViewportRenderer.create(canvas, (fps) => {
      fpsLabel.textContent = `FPS: ${fps}`;
    });
    if (renderer === null) {
      emptyHint.textContent = "当前环境不支持 WebGL2，无法渲染视口。";
      emptyHint.classList.add("text-red-400");
      emptyHint.classList.remove("text-zinc-600");
    }
  }

  loadMeshButton.addEventListener("click", () => {
    ensureRenderer();
    const { geometries } = appStore.get();
    const geometry = geometries[0];
    if (geometry === null || geometry === undefined || renderer === null) {
      return;
    }
    void getRenderMesh(geometry.geometryId).then((data) => {
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
      emptyHint.classList.add("hidden");
      loadMeshButton.disabled = false;
    });
  });

  clipToggle.addEventListener("click", () => {
    clipOn = !clipOn;
    clipToggle.textContent = clipOn ? "剖切：开" : "剖切：关";
    renderer?.setClip(clipOn, 0);
  });
  resetButton.addEventListener("click", () => renderer?.resetView());

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
  function onStateChange(): void {
    applyField(appStore.get().loadedField);
  }

  body.append(canvasWrap, controls);
  void detectRenderCapabilityInBrowser().then((capability) => {
    if (capability.backend === "webgl2") {
      emptyHint.textContent = `${emptyHint.textContent}（${capability.note}）`;
    }
  });
  appStore.subscribe(onStateChange);
  return root;
}
