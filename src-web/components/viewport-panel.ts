import { appStore, loadField } from "../state";
import type { ScalarField } from "../types";
import { ViewportRenderer } from "../render/renderer";
import { detectRenderCapabilityInBrowser } from "../render/capability";
import { registerSnapshot } from "../render/snapshot";
import { getRenderMesh } from "../services/geometry";
import { button, card, hint } from "./ui";

/** 3D 视口面板：WebGL2 渲染器 + 云图/剖切/时间步动画控制（WebGPU 探测提示）。 */
export function createViewportPanel(): HTMLElement {
  const { root, body } = card("3D 视口");

  const note = hint("正在探测渲染能力…");
  const canvas = document.createElement("canvas");
  canvas.className = "w-full rounded-lg bg-zinc-950";
  canvas.width = 960;
  canvas.height = 540;
  registerSnapshot("viewport", canvas);
  canvas.style.touchAction = "none";

  const controls = document.createElement("div");
  controls.className = "flex flex-wrap items-center gap-2";
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
      note.textContent = "当前环境不支持 WebGL2，无法渲染视口。";
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
    const min = Math.min(...field.values);
    const max = Math.max(...field.values);
    renderer.setFieldRange(min, max);
  }
  function onStateChange(): void {
    applyField(appStore.get().loadedField);
  }

  body.append(note, canvas, controls);
  void detectRenderCapabilityInBrowser().then((capability) => {
    if (capability.backend === "webgpu") {
      note.textContent = `${capability.note}（视口渲染当前使用 WebGL2 后端）`;
    } else if (capability.backend === "webgl2") {
      note.textContent = capability.note;
    }
  });
  appStore.subscribe(onStateChange);
  return root;
}
