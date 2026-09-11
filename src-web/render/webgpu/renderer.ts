/** WebGPU 渲染后端 POC（T39）：与 WebGL2 主后端同一 RenderMesh / 场值 /
 * 剖切平面语义的自研实现，验证自研 WebGPU 主路径的可行性。
 *
 * POC 边界（决策材料，见 ai-docs/reviews/T39-review.md）：
 * - 冷热配色与背景色取规范常量，未接主题 CSS 变量；
 * - 相机平移（右键拖拽）未实现，旋转 / 缩放 / 视角重置可用。
 */
import { mat4Identity, mat4LookAt, mat4Multiply, mat4Perspective, type Vec3 } from "../math";
import {
  LINE_FRAGMENT_SHADER,
  LINE_VERTEX_SHADER,
  MESH_FRAGMENT_SHADER,
  MESH_VERTEX_SHADER,
} from "./shaders";

/** 渲染网格（与 WebGL2 主后端同一数据模型，零转换上传）。 */
interface WebGPURenderMesh {
  positions: Float32Array;
  indices: Uint32Array;
  faceCells?: Uint32Array;
}

/** 线段叠加层（与 WebGL2 同一数据模型）。 */
interface WebGPUOverlayLayer {
  positions: Float32Array;
  color: [number, number, number];
}

const FOV_Y = Math.PI / 4;
const NEAR = 0.01;
const FAR = 100;
/** WGSL Uniforms 结构对齐后的大小：64(mvp) + 16 + 16 + 16 = 112 字节。 */
const UNIFORM_FLOATS = 28;
/** 线段 uniform：64(mvp) + 16(color) = 80 字节。 */
const LINE_UNIFORM_BYTES = 80;
const CLEAR_COLOR: readonly [number, number, number] = [0.035, 0.035, 0.043];

export class WebGPURenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: GpuCanvasContext;
  private readonly device: GpuDevice;
  private readonly format: GpuTextureFormat;
  private readonly onFps?: (fps: number) => void;

  private meshPipeline: GpuRenderPipeline;
  private linePipeline: GpuRenderPipeline;
  private uniformBuffer: GpuBuffer;
  private depthTexture: GpuTexture | null = null;
  private depthView: GpuTextureView | null = null;
  private readonly depthFormat: GpuTextureFormat = "depth24plus";

  private vertexBuffers: (GpuBuffer | undefined)[] = [];
  private indexBuffer: GpuBuffer | null = null;
  private indexCount = 0;
  private meshVisible = true;

  private overlays = new Map<
    string,
    { vertexBuffer: GpuBuffer; uniform: GpuBuffer; count: number; color: [number, number, number] }
  >();
  private overlayVisible = new Map<string, boolean>();

  private positions: Float32Array = new Float32Array(0);
  private useField = 0;
  private valueMin = 0;
  private valueMax = 1;
  private clipEnabled = 0;
  private clipNormal: Vec3 = [0, 1, 0];
  private clipOffset = 0;

  private yaw = 0.6;
  private pitch = 0.4;
  private distance = 3;
  private target: Vec3 = [0, 0, 0];
  private frameTimes: number[] = [];
  private rafHandle = 0;
  private disposed = false;
  private depthWidth = 0;
  private depthHeight = 0;

  private constructor(
    canvas: HTMLCanvasElement,
    context: GpuCanvasContext,
    device: GpuDevice,
    format: GpuTextureFormat,
    onFps?: (fps: number) => void,
  ) {
    this.canvas = canvas;
    this.context = context;
    this.device = device;
    this.format = format;
    this.onFps = onFps;
    this.context.configure({ device, format, alphaMode: "opaque" });
    this.meshPipeline = this.buildMeshPipeline();
    this.linePipeline = this.buildLinePipeline();
    this.uniformBuffer = device.createBuffer({
      size: UNIFORM_FLOATS * 4,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    this.attachControls();
    this.startLoop();
  }

  /** 初始化 WebGPU：适配器 / 设备 / 画布上下文任一不可用即返回 null（上层回退 WebGL2）。 */
  static async create(
    canvas: HTMLCanvasElement,
    onFps?: (fps: number) => void,
  ): Promise<WebGPURenderer | null> {
    try {
      const gpu = navigator.gpu;
      if (gpu === undefined) {
        return null;
      }
      const adapter = await gpu.requestAdapter({ powerPreference: "high-performance" });
      const device = (await adapter?.requestDevice()) ?? null;
      // d.ts 不合并 HTMLCanvasElement（避免污染 2d 重载解析），此处显式转换。
      const context =
        device !== null
          ? (canvas.getContext as (contextId: string) => GpuCanvasContext | null)("webgpu")
          : null;
      if (device === null || context === null) {
        return null;
      }
      const format = gpu.getPreferredCanvasFormat();
      return new WebGPURenderer(canvas, context, device, format, onFps);
    } catch {
      // 适配器请求可能因驱动 / 黑名单抛错：按不可用处理，交上层回退。
      return null;
    }
  }

  private buildMeshPipeline(): GpuRenderPipeline {
    const module = this.device.createShaderModule({
      code: MESH_VERTEX_SHADER + MESH_FRAGMENT_SHADER,
    });
    return this.device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vs",
        buffers: [
          { arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] },
          { arrayStride: 12, attributes: [{ shaderLocation: 1, offset: 0, format: "float32x3" }] },
          { arrayStride: 4, attributes: [{ shaderLocation: 2, offset: 0, format: "float32" }] },
        ],
      },
      fragment: { module, entryPoint: "fs", targets: [{ format: this.format }] },
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthStencil: { format: this.depthFormat, depthWriteEnabled: true, depthCompare: "less" },
    });
  }

  private buildLinePipeline(): GpuRenderPipeline {
    const module = this.device.createShaderModule({
      code: LINE_VERTEX_SHADER + LINE_FRAGMENT_SHADER,
    });
    return this.device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vs",
        buffers: [
          { arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] },
        ],
      },
      fragment: { module, entryPoint: "fs", targets: [{ format: this.format }] },
      primitive: { topology: "line-list" },
      depthStencil: { format: this.depthFormat, depthWriteEnabled: true, depthCompare: "less" },
    });
  }

  /** 上传渲染网格：位置 / 法向 / 值 / 索引四个缓冲（法向在 CPU 侧重建）。 */
  uploadMesh(mesh: WebGPURenderMesh): void {
    this.positions = mesh.positions;
    this.replaceVertexBuffer(0, mesh.positions);
    this.replaceVertexBuffer(1, computeFaceNormals(mesh.positions, mesh.indices));
    this.replaceVertexBuffer(2, new Float32Array(mesh.positions.length / 3));
    this.indexBuffer?.destroy();
    this.indexBuffer = this.device.createBuffer({
      size: align4(mesh.indices.byteLength),
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
    });
    this.device.queue.writeBuffer(this.indexBuffer, 0, mesh.indices);
    this.indexCount = mesh.indices.length;
    this.meshVisible = true;
  }

  /** 云图逐面值热更新（与 WebGL2 的 setFaceValues 同语义）。 */
  setFaceValues(perFace: Float32Array): void {
    const values = new Float32Array(perFace.length * 3);
    for (let face = 0; face < perFace.length; face += 1) {
      for (let corner = 0; corner < 3; corner += 1) {
        values[face * 3 + corner] = perFace[face] ?? 0;
      }
    }
    this.replaceVertexBuffer(2, values);
  }

  setFieldRange(min: number, max: number): void {
    this.valueMin = min;
    this.valueMax = max;
    this.useField = 1;
  }

  setClipPlane(enabled: boolean, normal: Vec3, offset: number): void {
    this.clipEnabled = enabled ? 1 : 0;
    this.clipNormal = normal;
    this.clipOffset = offset;
  }

  /** 线段叠加层上传（每层一个顶点缓冲 + 一个颜色 uniform）。 */
  uploadOverlay(id: string, layer: WebGPUOverlayLayer): void {
    const vertexBuffer = this.makeVertexBuffer(layer.positions);
    const uniform = this.device.createBuffer({
      size: LINE_UNIFORM_BYTES,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    this.overlays.set(id, {
      vertexBuffer,
      uniform,
      count: layer.positions.length / 3,
      color: layer.color,
    });
  }

  setMeshVisible(visible: boolean): void {
    this.meshVisible = visible;
  }

  setOverlayVisible(id: string, visible: boolean): void {
    this.overlayVisible.set(id, visible);
  }

  getCamera(): {
    eye: Vec3;
    target: Vec3;
    fovY: number;
    aspect: number;
    width: number;
    height: number;
  } {
    return {
      eye: this.eye(),
      target: [this.target[0], this.target[1], this.target[2]],
      fovY: FOV_Y,
      aspect: this.canvas.width / Math.max(this.canvas.height, 1),
      width: this.canvas.width,
      height: this.canvas.height,
    };
  }

  getMeshBounds(): { min: Vec3; max: Vec3 } | null {
    if (this.positions.length === 0) {
      return null;
    }
    const min: Vec3 = [Infinity, Infinity, Infinity];
    const max: Vec3 = [-Infinity, -Infinity, -Infinity];
    for (let i = 0; i < this.positions.length; i += 3) {
      for (let axis = 0; axis < 3; axis += 1) {
        const value = this.positions[i + axis] as number;
        if (value < (min[axis] as number)) {
          min[axis] = value;
        }
        if (value > (max[axis] as number)) {
          max[axis] = value;
        }
      }
    }
    return { min, max };
  }

  resetView(): void {
    this.yaw = 0.6;
    this.pitch = 0.4;
    this.distance = 3;
    this.target = [0, 0, 0];
  }

  zoomBy(factor: number): void {
    this.distance = Math.min(Math.max(this.distance * factor, 0.1), 500);
  }

  fitView(): void {
    const bounds = this.getMeshBounds();
    if (bounds === null) {
      return;
    }
    let span = 1e-6;
    for (let axis = 0; axis < 3; axis += 1) {
      span = Math.max(span, (bounds.max[axis] as number) - (bounds.min[axis] as number));
    }
    this.distance = span * 2.5;
    this.target = [
      (bounds.min[0] + bounds.max[0]) / 2,
      (bounds.min[1] + bounds.max[1]) / 2,
      (bounds.min[2] + bounds.max[2]) / 2,
    ];
  }

  dispose(): void {
    this.disposed = true;
    cancelAnimationFrame(this.rafHandle);
    this.device.destroy();
  }

  private makeVertexBuffer(data: Float32Array): GpuBuffer {
    const buffer = this.device.createBuffer({
      size: align4(data.byteLength),
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });
    this.device.queue.writeBuffer(buffer, 0, data);
    return buffer;
  }

  private replaceVertexBuffer(slot: number, data: Float32Array): void {
    this.vertexBuffers[slot]?.destroy();
    this.vertexBuffers[slot] = this.makeVertexBuffer(data);
  }

  private eye(): Vec3 {
    return [
      this.target[0] + this.distance * Math.cos(this.pitch) * Math.sin(this.yaw),
      this.target[1] + this.distance * Math.sin(this.pitch),
      this.target[2] + this.distance * Math.cos(this.pitch) * Math.cos(this.yaw),
    ];
  }

  private ensureDepth(): GpuTextureView {
    if (this.canvas.width !== this.depthWidth || this.canvas.height !== this.depthHeight) {
      this.depthWidth = this.canvas.width;
      this.depthHeight = this.canvas.height;
      this.depthTexture?.destroy();
      this.depthTexture = this.device.createTexture({
        size: [Math.max(this.depthWidth, 1), Math.max(this.depthHeight, 1)],
        format: this.depthFormat,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      });
      this.depthView = this.depthTexture.createView();
    }
    return this.depthView!;
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

  private drawFrame(): void {
    const aspect = this.canvas.width / Math.max(this.canvas.height, 1);
    const projection = mat4Perspective(FOV_Y, aspect, NEAR, FAR);
    const view = mat4LookAt(this.eye(), this.target, [0, 1, 0]);
    const mvp = mat4Multiply(mat4Multiply(projection, view), mat4Identity());

    // WGSL 结构对齐：mvp @0(64B)、light_dir @64(12B)、clip_offset @76、
    // clip_normal @80(12B)、use_field @92、value_min @96、value_max @100、pad @104。
    const uniforms = new Float32Array(UNIFORM_FLOATS);
    uniforms.set(mvp, 0);
    uniforms.set([0.4, 0.8, 0.5], 16);
    uniforms[19] = this.clipOffset;
    uniforms.set(this.clipNormal, 20);
    uniforms[23] = this.clipEnabled;
    uniforms[24] = this.useField;
    uniforms[25] = this.valueMin;
    uniforms[26] = this.valueMax;
    this.device.queue.writeBuffer(this.uniformBuffer, 0, uniforms);

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: CLEAR_COLOR[0], g: CLEAR_COLOR[1], b: CLEAR_COLOR[2], a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
      depthStencilAttachment: {
        view: this.ensureDepth(),
        depthClearValue: 1,
        depthLoadOp: "clear",
        depthStoreOp: "store",
      },
    });
    pass.setPipeline(this.meshPipeline);
    pass.setBindGroup(0, this.meshPipeline.getBindGroupLayout(0));
    if (this.meshVisible && this.indexBuffer !== null && this.indexCount > 0) {
      pass.setVertexBuffer(0, this.vertexBuffers[0]!);
      pass.setVertexBuffer(1, this.vertexBuffers[1]!);
      pass.setVertexBuffer(2, this.vertexBuffers[2]!);
      pass.setIndexBuffer(this.indexBuffer, "uint32");
      pass.drawIndexed(this.indexCount);
    }
    for (const [id, overlay] of this.overlays) {
      if ((this.overlayVisible.get(id) ?? true) === false) {
        continue;
      }
      const lineUniforms = new Float32Array(LINE_UNIFORM_BYTES / 4);
      lineUniforms.set(mvp, 0);
      lineUniforms.set([...overlay.color, 1], 16);
      this.device.queue.writeBuffer(overlay.uniform, 0, lineUniforms);
      pass.setPipeline(this.linePipeline);
      pass.setBindGroup(0, this.linePipeline.getBindGroupLayout(0));
      pass.setVertexBuffer(0, overlay.vertexBuffer);
      pass.draw(overlay.count);
    }
    pass.end();
    this.device.queue.submit([encoder.finish()]);

    const now = performance.now();
    this.frameTimes.push(now);
    while (this.frameTimes.length > 0 && now - (this.frameTimes[0] ?? now) > 1000) {
      this.frameTimes.shift();
    }
    this.onFps?.(this.frameTimes.length);
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
        this.zoomBy(event.deltaY > 0 ? 1.1 : 0.9);
      },
      { passive: false },
    );
  }
}

function align4(size: number): number {
  return Math.ceil(size / 4) * 4;
}

/** 逐面法向（复制到三个顶点），与 WebGL2 主后端同算法。 */
function computeFaceNormals(positions: Float32Array, indices: Uint32Array): Float32Array {
  const normals = new Float32Array(positions.length);
  const faceCount = indices.length / 3;
  for (let face = 0; face < faceCount; face += 1) {
    const ia = indices[face * 3] ?? 0;
    const ib = indices[face * 3 + 1] ?? 0;
    const ic = indices[face * 3 + 2] ?? 0;
    const ax = positions[ia * 3] ?? 0;
    const ay = positions[ia * 3 + 1] ?? 0;
    const az = positions[ia * 3 + 2] ?? 0;
    const e1x = (positions[ib * 3] ?? 0) - ax;
    const e1y = (positions[ib * 3 + 1] ?? 0) - ay;
    const e1z = (positions[ib * 3 + 2] ?? 0) - az;
    const e2x = (positions[ic * 3] ?? 0) - ax;
    const e2y = (positions[ic * 3 + 1] ?? 0) - ay;
    const e2z = (positions[ic * 3 + 2] ?? 0) - az;
    const nx = e1y * e2z - e1z * e2y;
    const ny = e1z * e2x - e1x * e2z;
    const nz = e1x * e2y - e1y * e2x;
    const length = Math.hypot(nx, ny, nz) || 1;
    for (const index of [ia, ib, ic]) {
      normals[index * 3] = nx / length;
      normals[index * 3 + 1] = ny / length;
      normals[index * 3 + 2] = nz / length;
    }
  }
  return normals;
}
