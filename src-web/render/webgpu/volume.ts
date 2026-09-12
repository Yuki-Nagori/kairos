/** 三维体渲染 POC：WebGPU 光线步进体绘制。
 * 数据源为 core `services/volume_field` 重采样的结构化体素场
 * （四面体场 → 规则网格）；固定步数 + 前向合成 alpha 混色。
 * POC 边界：无交互式迁移函数编辑、无光照、步数固定、相机仅旋转缩放。 */

import type { Vec3 } from "../math";
import {
  VOLUME_FRAGMENT_SHADER,
  VOLUME_UNIFORM_FLOATS,
  VOLUME_VERTEX_SHADER,
} from "./shaders_volume";

interface VolumeGrid {
  /** 三轴体素数。 */
  dims: [number, number, number];
  /** 网格原点（体素 [0,0,0] 角点）。 */
  origin: Vec3;
  /** 体素间距。 */
  spacing: Vec3;
  /** 体素值（x 主序，f32）。 */
  values: Float32Array;
}

export class VolumeRaymarcher {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: GpuCanvasContext;
  private readonly device: GpuDevice;
  private readonly grid: VolumeGrid;
  private readonly pipeline: GpuRenderPipeline;
  private readonly uniformBuffer: GpuBuffer;
  private readonly bindGroup: GpuBindGroup;

  private yaw = 0.7;
  private pitch = 0.5;
  private distance = 3;
  private rafHandle = 0;
  private disposed = false;
  private frames = 0;
  private lastReportAt = performance.now();
  private valueMax = 1;
  private onFps?: (fps: number) => void;

  private constructor(
    canvas: HTMLCanvasElement,
    context: GpuCanvasContext,
    device: GpuDevice,
    format: GpuTextureFormat,
    grid: VolumeGrid,
    onFps?: (fps: number) => void,
  ) {
    this.canvas = canvas;
    this.context = context;
    this.device = device;
    this.grid = grid;
    this.onFps = onFps;
    this.valueMax = maxValue(grid.values);
    this.context.configure({ device, format, alphaMode: "opaque" });

    const volumeData = new Float32Array(grid.values);
    const volumeBuffer = device.createBuffer({
      size: align16(volumeData.byteLength),
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    device.queue.writeBuffer(volumeBuffer, 0, volumeData);
    this.uniformBuffer = device.createBuffer({
      size: VOLUME_UNIFORM_FLOATS * 4,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    const module = device.createShaderModule({
      code: VOLUME_VERTEX_SHADER + VOLUME_FRAGMENT_SHADER,
    });
    this.pipeline = device.createRenderPipeline({
      layout: "auto",
      vertex: { module, entryPoint: "vs" },
      fragment: { module, entryPoint: "fs", targets: [{ format }] },
      primitive: { topology: "triangle-list" },
    });
    this.bindGroup = device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.uniformBuffer } },
        { binding: 1, resource: { buffer: volumeBuffer } },
      ],
    });

    this.attachControls();
    this.startLoop();
  }

  /** 初始化 WebGPU：任一环节不可用返回 null（调用方提示体渲染不受支持）。 */
  static async create(
    canvas: HTMLCanvasElement,
    grid: VolumeGrid,
    onFps?: (fps: number) => void,
  ): Promise<VolumeRaymarcher | null> {
    try {
      const gpu = navigator.gpu;
      if (gpu === undefined) {
        return null;
      }
      const adapter = await gpu.requestAdapter({ powerPreference: "high-performance" });
      const device = (await adapter?.requestDevice()) ?? null;
      const context =
        device !== null
          ? (canvas.getContext as (contextId: string) => GpuCanvasContext | null)("webgpu")
          : null;
      if (device === null || context === null) {
        return null;
      }
      const format = gpu.getPreferredCanvasFormat();
      return new VolumeRaymarcher(canvas, context, device, format, grid, onFps);
    } catch {
      return null;
    }
  }

  private drawFrame(): void {
    // 目标点恒为网格中心；轨道半径缩放整个包围盒。
    const extent: Vec3 = [
      this.grid.dims[0] * this.grid.spacing[0],
      this.grid.dims[1] * this.grid.spacing[1],
      this.grid.dims[2] * this.grid.spacing[2],
    ];
    const center: Vec3 = [extent[0] / 2, extent[1] / 2, extent[2] / 2];
    const target: Vec3 = center;
    const eye: Vec3 = [
      center[0] + this.distance * extent[0] * Math.cos(this.pitch) * Math.sin(this.yaw),
      center[1] + this.distance * extent[1] * Math.sin(this.pitch),
      center[2] + this.distance * extent[2] * Math.cos(this.pitch) * Math.cos(this.yaw),
    ];
    const forward = normalize(sub(target, eye));
    const worldUp: Vec3 = [0, 1, 0];
    const right = normalize(cross(forward, worldUp));
    const up = normalize(cross(right, forward));
    const aspect = this.canvas.width / Math.max(this.canvas.height, 1);
    const tanHalf = Math.tan(Math.PI / 8);

    const uniforms = new Float32Array(VOLUME_UNIFORM_FLOATS);
    uniforms.set([this.grid.dims[0], this.grid.dims[1], this.grid.dims[2], 0], 0);
    uniforms.set([this.grid.origin[0], this.grid.origin[1], this.grid.origin[2], 0], 4);
    uniforms.set([this.grid.spacing[0], this.grid.spacing[1], this.grid.spacing[2], 0], 8);
    uniforms.set([eye[0], eye[1], eye[2], 0], 12);
    uniforms.set([right[0], right[1], right[2], 0], 16);
    uniforms.set([up[0], up[1], up[2], 0], 20);
    uniforms.set([forward[0], forward[1], forward[2], 0], 24);
    uniforms.set([this.canvas.width, this.canvas.height, aspect, tanHalf], 28);
    // value_max = 场最大值（网格生成时已归一化到 0..1，此处恒 1）；step_alpha、步数。
    uniforms.set([this.valueMax, 0.12, STEPS, 0], 32);
    this.device.queue.writeBuffer(this.uniformBuffer, 0, uniforms);

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.035, g: 0.035, b: 0.043, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);
    pass.draw(3);
    pass.end();
    this.device.queue.submit([encoder.finish()]);

    this.frames += 1;
    const now = performance.now();
    if (now - this.lastReportAt >= 1000) {
      this.onFps?.((this.frames * 1000) / (now - this.lastReportAt));
      this.frames = 0;
      this.lastReportAt = now;
    }
  }

  private startLoop(): void {
    const frame = () => {
      if (this.disposed) {
        return;
      }
      this.drawFrame();
      this.rafHandle = requestAnimationFrame(frame);
    };
    this.rafHandle = requestAnimationFrame(frame);
  }

  private attachControls(): void {
    let dragging = false;
    this.canvas.addEventListener("mousedown", (event) => {
      if (event.button === 0) {
        dragging = true;
      }
    });
    window.addEventListener("mouseup", () => {
      dragging = false;
    });
    this.canvas.addEventListener("mousemove", (event) => {
      if (!dragging) {
        return;
      }
      this.yaw -= event.movementX * 0.005;
      this.pitch = Math.min(
        Math.max(this.pitch + event.movementY * 0.005, -Math.PI / 2 + 0.01),
        Math.PI / 2 - 0.01,
      );
    });
    this.canvas.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        this.distance = Math.min(Math.max(this.distance * (event.deltaY > 0 ? 1.1 : 0.9), 0.5), 20);
      },
      { passive: false },
    );
  }

  dispose(): void {
    this.disposed = true;
    cancelAnimationFrame(this.rafHandle);
    this.device.destroy();
  }
}

const STEPS = 96;

function maxValue(values: Float32Array): number {
  let max = 0.0;
  for (const value of values) {
    max = Math.max(max, value);
  }
  return max > 0 ? max : 1;
}

function align16(size: number): number {
  return Math.ceil(size / 16) * 16;
}

function normalize(v: Vec3): Vec3 {
  const length = Math.hypot(v[0], v[1], v[2]) || 1;
  return [v[0] / length, v[1] / length, v[2] / length];
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function sub(a: Vec3, b: Vec3): Vec3 {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}
