/** 渲染后端基准（验收协议）：确定性 100 万三角形资产上度量
 * 上传耗时 / 首帧 / 稳定 FPS / 剖切切换延迟。
 * 需要 GPU 后端的运行环境（Tauri WebView / 浏览器）；bun 直跑时
 * WebGPU 不可用会打印不支持提示而非伪造数字。
 * 运行：bun src-web/utils/bench/webgpu-render.ts
 */
import { WebGPURenderer } from "../../render/webgpu/renderer";
import { deterministicGridMesh } from "../../render/webgpu/mesh-asset";

const FRAME_SAMPLES = 120;

async function main(): Promise<void> {
  if (typeof document === "undefined") {
    console.error("本基准需要浏览器 / WebView DOM 环境（document 未定义）。");
    process.exit(1);
  }
  const canvas = document.createElement("canvas");
  canvas.width = 1280;
  canvas.height = 720;
  document.body.append(canvas);

  const renderer = await WebGPURenderer.create(canvas, () => {});
  if (renderer === null) {
    console.error("WebGPU 不可用：本基准仅度量 WebGPU 后端（WebGL2 基准另行触发）。");
    process.exit(1);
  }

  // 1M 三角形 = 2 × 707²（707² = 499849 → 999698 三角形，取最接近的合法值）。
  const triangles = 2 * 707 * 707;
  const mesh = deterministicGridMesh(triangles);

  const uploadStart = performance.now();
  renderer.uploadMesh(mesh);
  const uploadMs = performance.now() - uploadStart;

  renderer.fitView();
  renderer.setFieldRange(0, 1);
  // 首帧以渲染循环提交过命令（两帧 RAF）为界。
  const firstFrameStart = performance.now();
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
  const firstFrameMs = performance.now() - firstFrameStart;

  // 稳定 FPS：采样 FRAME_SAMPLES 帧。
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
  const p05 = fpsSamples[Math.floor(fpsSamples.length * 0.05)] ?? 0;

  // 剖切切换延迟：连续翻转 30 次取平均。
  const clipStart = performance.now();
  for (let i = 0; i < 30; i += 1) {
    renderer.setClipPlane(i % 2 === 0, [0, 1, 0], 0.5);
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }
  const clipToggleMs = (performance.now() - clipStart) / 30;

  console.table([
    { 指标: "三角形数", 值: triangles },
    { 指标: "上传耗时 (ms)", 值: uploadMs.toFixed(1) },
    { 指标: "首帧 (ms)", 值: firstFrameMs.toFixed(1) },
    { 指标: "中位 FPS", 值: median.toFixed(1) },
    { 指标: "P05 FPS", 值: p05.toFixed(1) },
    { 指标: "剖切切换 (ms/次)", 值: clipToggleMs.toFixed(2) },
  ]);
  renderer.dispose();
  canvas.remove();
}

void main();
