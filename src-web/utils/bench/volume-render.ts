/** 体渲染基准（T50 验收协议）：确定性体素场上度量光线步进的
 * 上传耗时 / 首帧 / FPS（96 步 × 视口分辨率）。
 * 需要 GPU 后端的运行环境（Tauri WebView / 浏览器）。
 * 运行：bun src-web/utils/bench/volume-render.ts
 */
import { VolumeRaymarcher } from "../../render/webgpu/volume";

const DIMS = 128;
const FRAME_SAMPLES = 120;

async function main(): Promise<void> {
  if (typeof document === "undefined") {
    console.error("本基准需要浏览器 / WebView DOM 环境（document 未定义）。");
    process.exit(1);
  }
  // 确定性体素场：中心球体（密度随半径衰减），值域 0..1。
  const values = new Float32Array(DIMS * DIMS * DIMS);
  const center = (DIMS - 1) / 2;
  for (let iz = 0; iz < DIMS; iz += 1) {
    for (let iy = 0; iy < DIMS; iy += 1) {
      for (let ix = 0; ix < DIMS; ix += 1) {
        const radius = Math.hypot(ix - center, iy - center, iz - center) / (DIMS / 2);
        const inside = radius <= 1;
        values[ix + DIMS * (iy + DIMS * iz)] = inside ? 1 - radius : 0;
      }
    }
  }

  const canvas = document.createElement("canvas");
  canvas.width = 1280;
  canvas.height = 720;
  document.body.append(canvas);

  const uploadStart = performance.now();
  const renderer = await VolumeRaymarcher.create(
    canvas,
    {
      dims: [DIMS, DIMS, DIMS],
      origin: [0, 0, 0],
      spacing: [1 / DIMS, 1 / DIMS, 1 / DIMS],
      values,
    },
    () => {},
  );
  const uploadMs = performance.now() - uploadStart;
  if (renderer === null) {
    console.error("WebGPU 不可用：体渲染基准需要 GPU 环境实测。");
    process.exit(1);
  }

  const firstFrameStart = performance.now();
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
  const firstFrameMs = performance.now() - firstFrameStart;

  const fpsSamples: number[] = [];
  let last = performance.now();
  for (let i = 0; i < FRAME_SAMPLES; i += 1) {
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => resolve());
    });
    const now = performance.now();
    fpsSamples.push(1000 / (now - last));
    last = now;
  }
  fpsSamples.sort((a, b) => a - b);
  const median = fpsSamples[Math.floor(fpsSamples.length / 2)] ?? 0;

  console.table([
    { 指标: "体素数", 值: `${DIMS}³ = ${DIMS ** 3}` },
    { 指标: "上传耗时 (ms)", 值: uploadMs.toFixed(1) },
    { 指标: "首帧 (ms)", 值: firstFrameMs.toFixed(1) },
    { 指标: "中位 FPS", 值: median.toFixed(1) },
    { 指标: "光线步数", 值: 96 },
  ]);
  renderer.dispose();
  canvas.remove();
}

void main();
