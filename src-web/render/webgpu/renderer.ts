/** WebGPU 渲染后端：与 WebGL2 主后端同一 RenderMesh / 场值 / 剖切平面语义的
 * 自研实现，是视口的主路径（适配器不可用时才回退 WebGL2）。
 *
 * 边界：相机平移（右键拖拽）未实现，旋转 / 缩放 / 视角重置可用。
 */
import {
  boundsOf,
  mat4Identity,
  mat4LookAt,
  mat4Multiply,
  mat4Perspective,
  meshFarPlane,
  type Vec3,
} from "../math";
import { computeVertexNormals } from "../normals";
import { THEME_CHANGED_EVENT, themeRgb } from "../../utils/theme";
import { errorMessage } from "../../utils/error";
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
/** WGSL Uniforms 结构对齐后的大小：64(mvp) + 16 + 16 + 16 = 112 字节。 */
const UNIFORM_FLOATS = 28;
/** 线段 uniform：64(mvp) + 16(color) = 80 字节。 */
const LINE_UNIFORM_BYTES = 80;
/** 视口清屏色的深色缺省（主题变量缺失时的兜底）。 */
const FALLBACK_CLEAR: [number, number, number] = [0.035, 0.035, 0.043];

export class WebGPURenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: GpuCanvasContext;
  private readonly device: GpuDevice;
  private readonly format: GpuTextureFormat;
  private readonly onFps?: (fps: number) => void;
  /** 绘制失败的上报通道（仅在与上次不同的原因时触发，避免每帧刷屏）。 */
  private readonly onError?: (message: string) => void;
  /** 最近一次绘制失败的原因（null = 正常）。 */
  private frameError: string | null = null;
  private readonly onView?: (state: {
    x: number;
    y: number;
    z: number;
    yaw: number;
    pitch: number;
    distance: number;
  }) => void;

  private meshPipeline: GpuRenderPipeline;
  private linePipeline: GpuRenderPipeline;
  private uniformBuffer: GpuBuffer;
  /** 网格管线与线管线的 bind group（按当前 uniform 缓冲懒建并缓存）。 */
  private meshBindGroup: GpuBindGroup | null = null;
  private depthTexture: GpuTexture | null = null;
  private depthView: GpuTextureView | null = null;
  private readonly depthFormat: GpuTextureFormat = "depth24plus";

  private vertexBuffers: (GpuBuffer | undefined)[] = [];
  private indexBuffer: GpuBuffer | null = null;
  private indexCount = 0;
  private meshVisible = true;

  private overlays = new Map<
    string,
    {
      vertexBuffer: GpuBuffer;
      uniform: GpuBuffer;
      count: number;
      color: [number, number, number];
      /** 该层的 bind group（颜色 uniform 固定，创建一次即可）。 */
      bindGroup?: GpuBindGroup;
    }
  >();
  private overlayVisible = new Map<string, boolean>();

  /** 最后上传网格的包围盒（上传时算一次；剖切滑块与 fitView 都读它）。 */
  private meshBounds: { min: Vec3; max: Vec3 } | null = null;
  private useField = 0;
  private valueMin = 0;
  private valueMax = 1;
  private clipEnabled = 0;
  private clipNormal: Vec3 = [0, 1, 0];
  private clipOffset = 0;
  /** 清屏色取主题变量；主题切换后由 onThemeChanged 重取。 */
  private clearColor: [number, number, number] = FALLBACK_CLEAR;

  private yaw = 0.6;
  private pitch = 0.4;
  private distance = 3;
  private target: Vec3 = [0, 0, 0];
  private frameTimes: number[] = [];
  private rafHandle = 0;
  private disposed = false;
  private depthWidth = 0;
  private depthHeight = 0;
  /** 左键拖拽中（mouseup 在 window 上收尾，故状态存字段）。 */
  private dragging = false;
  /** window 监听一律留成类字段：闭包里的匿名监听在 dispose 时摘不掉。 */
  private readonly onWindowMouseUp = (): void => {
    this.dragging = false;
  };
  private readonly onThemeChanged = (): void => {
    this.clearColor = themeRgb("--c-viewport-bg", FALLBACK_CLEAR);
  };

  private constructor(
    canvas: HTMLCanvasElement,
    context: GpuCanvasContext,
    device: GpuDevice,
    format: GpuTextureFormat,
    onFps?: (fps: number) => void,
    onView?: (state: {
      x: number;
      y: number;
      z: number;
      yaw: number;
      pitch: number;
      distance: number;
    }) => void,
    onError?: (message: string) => void,
  ) {
    this.onView = onView;
    this.canvas = canvas;
    this.context = context;
    this.device = device;
    this.format = format;
    this.onFps = onFps;
    this.onError = onError;
    this.context.configure({ device, format, alphaMode: "opaque" });
    this.meshPipeline = this.buildMeshPipeline();
    this.linePipeline = this.buildLinePipeline();
    this.uniformBuffer = device.createBuffer({
      size: UNIFORM_FLOATS * 4,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    this.onThemeChanged();
    window.addEventListener(THEME_CHANGED_EVENT, this.onThemeChanged);
    this.attachControls();
    this.startLoop();
  }

  /** 初始化 WebGPU：适配器 / 设备 / 画布上下文任一不可用即返回 null（上层回退 WebGL2）。 */
  static async create(
    canvas: HTMLCanvasElement,
    onFps?: (fps: number) => void,
    onView?: (state: {
      x: number;
      y: number;
      z: number;
      yaw: number;
      pitch: number;
      distance: number;
    }) => void,
    onError?: (message: string) => void,
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
      return new WebGPURenderer(canvas, context, device, format, onFps, onView, onError);
    } catch {
      // 适配器请求可能因驱动 / 黑名单抛错：按不可用处理，交上层回退。
      return null;
    }
  }

  private buildMeshPipeline(): GpuRenderPipeline {
    const vertexModule = this.device.createShaderModule({ code: MESH_VERTEX_SHADER });
    const fragmentModule = this.device.createShaderModule({ code: MESH_FRAGMENT_SHADER });
    return this.device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module: vertexModule,
        entryPoint: "vs",
        buffers: [
          { arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] },
          { arrayStride: 12, attributes: [{ shaderLocation: 1, offset: 0, format: "float32x3" }] },
          { arrayStride: 4, attributes: [{ shaderLocation: 2, offset: 0, format: "float32" }] },
        ],
      },
      fragment: { module: fragmentModule, entryPoint: "fs", targets: [{ format: this.format }] },
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthStencil: { format: this.depthFormat, depthWriteEnabled: true, depthCompare: "less" },
    });
  }

  private buildLinePipeline(): GpuRenderPipeline {
    const vertexModule = this.device.createShaderModule({ code: LINE_VERTEX_SHADER });
    const fragmentModule = this.device.createShaderModule({ code: LINE_FRAGMENT_SHADER });
    return this.device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module: vertexModule,
        entryPoint: "vs",
        buffers: [
          { arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] },
        ],
      },
      fragment: { module: fragmentModule, entryPoint: "fs", targets: [{ format: this.format }] },
      primitive: { topology: "line-list" },
      depthStencil: { format: this.depthFormat, depthWriteEnabled: true, depthCompare: "less" },
    });
  }

  /** 上传渲染网格：位置 / 法向 / 值 / 索引四个缓冲（法向在 CPU 侧重建）。 */
  uploadMesh(mesh: WebGPURenderMesh): void {
    this.meshBounds = boundsOf(mesh.positions);
    this.replaceVertexBuffer(0, mesh.positions);
    this.replaceVertexBuffer(1, computeVertexNormals(mesh.positions, mesh.indices));
    this.replaceVertexBuffer(2, new Float32Array(mesh.positions.length / 3));
    this.indexBuffer?.destroy();
    this.indexBuffer = this.device.createBuffer({
      size: align4(mesh.indices.byteLength),
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
    });
    this.device.queue.writeBuffer(this.indexBuffer, 0, mesh.indices);
    this.indexCount = mesh.indices.length;
    this.meshVisible = true;
    this.fitView();
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

  /** 网格包围盒（剖切位置滑块的世界坐标映射）；未载入网格返回 null。 */
  getMeshBounds(): { min: Vec3; max: Vec3 } | null {
    return this.meshBounds;
  }

  resetView(): void {
    this.yaw = 0.6;
    this.pitch = 0.4;
    this.distance = 3;
    this.target = [0, 0, 0];
    this.fitView();
  }

  zoomBy(factor: number): void {
    this.distance = Math.max(this.distance * factor, 0.1);
  }

  /** 轨道相机快照 / 恢复（多视口联动）。 */
  getOrbit(): {
    x: number;
    y: number;
    z: number;
    yaw: number;
    pitch: number;
    distance: number;
  } {
    return {
      x: this.target[0],
      y: this.target[1],
      z: this.target[2],
      yaw: this.yaw,
      pitch: this.pitch,
      distance: this.distance,
    };
  }

  /** 复刻轨道相机（多视口联动）。这里刻意**不**回报 onView：联动是单向下发，
   *  一旦回报就会变成「A 同步给 B → B 回报 → B 又同步给 A」的相互递归，
   *  四分格下第一次拖动相机会直接栈溢出。 */
  setOrbit(orbit: {
    x: number;
    y: number;
    z: number;
    yaw: number;
    pitch: number;
    distance: number;
  }): void {
    this.target = [orbit.x, orbit.y, orbit.z];
    this.yaw = orbit.yaw;
    this.pitch = orbit.pitch;
    this.distance = orbit.distance;
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
    window.removeEventListener(THEME_CHANGED_EVENT, this.onThemeChanged);
    window.removeEventListener("mouseup", this.onWindowMouseUp);
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
      // 画布尚未布局（宽或高为 0）时跳过绘制：以 0 尺寸创建纹理会被 WebGPU 判为
      // 校验错误，异常会让循环在**第一帧**就死掉——画布保持空白且 FPS 永远停在 —，
      // 用户看到的是「3D 显示不出来」而没有任何提示。
      // 画布 backing store 同步：CSS 尺寸 × DPR（上限 2）。缺少这一步时 backing
      // store 停在默认 300×150，画出来的是被拉伸的模糊像（或被判尺寸不合法的空帧）。
      this.syncCanvasSize();
      if (this.canvas.width === 0 || this.canvas.height === 0) {
        this.rafHandle = requestAnimationFrame(frame);
        return;
      }
      try {
        this.drawFrame();
        this.frameError = null;
      } catch (error) {
        // 单帧失败不终止循环：把原因留存并**主动上报**（面板据此显示提示），
        // 下一帧继续尝试，布局或尺寸恢复后画面能自己回来。
        const message = errorMessage(error);
        if (message !== this.frameError) {
          this.frameError = message;
          this.onError?.(message);
        }
      }
      this.rafHandle = requestAnimationFrame(frame);
    };
    this.rafHandle = requestAnimationFrame(frame);
  }

  /** 最近一次绘制失败的原因（null = 正常）；供 UI 诊断显示。 */
  renderError(): string | null {
    return this.frameError;
  }

  /** 按 CSS 尺寸 × DPR（上限 2）同步 backing store；与 WebGL2 后端同一口径。 */
  private syncCanvasSize(): void {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const rect = this.canvas.getBoundingClientRect();
    const width = Math.max(Math.round((rect.width || this.canvas.clientWidth) * dpr), 1);
    const height = Math.max(Math.round((rect.height || this.canvas.clientHeight) * dpr), 1);
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }

  private drawFrame(): void {
    const aspect = this.canvas.width / Math.max(this.canvas.height, 1);
    const projection = mat4Perspective(
      FOV_Y,
      aspect,
      NEAR,
      meshFarPlane(this.eye(), this.meshBounds),
    );
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
          clearValue: {
            r: this.clearColor[0],
            g: this.clearColor[1],
            b: this.clearColor[2],
            a: 1,
          },
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
    this.meshBindGroup ??= this.device.createBindGroup({
      layout: this.meshPipeline.getBindGroupLayout(0),
      entries: [{ binding: 0, resource: { buffer: this.uniformBuffer } }],
    });
    pass.setBindGroup(0, this.meshBindGroup);
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
      // 每层自己的颜色 uniform：bind group 随层缓存（覆盖层的缓冲不会变）
      const lineBindGroup =
        overlay.bindGroup ??
        this.device.createBindGroup({
          layout: this.linePipeline.getBindGroupLayout(0),
          entries: [{ binding: 0, resource: { buffer: overlay.uniform } }],
        });
      overlay.bindGroup = lineBindGroup;
      pass.setBindGroup(0, lineBindGroup);
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
    this.canvas.addEventListener("mousedown", (event) => {
      if (event.button === 0) {
        this.dragging = true;
      }
    });
    window.addEventListener("mouseup", this.onWindowMouseUp);
    this.canvas.addEventListener("mousemove", (event) => {
      if (!this.dragging) {
        return;
      }
      this.yaw -= event.movementX * 0.005;
      this.pitch = Math.min(
        Math.max(this.pitch + event.movementY * 0.005, -Math.PI / 2 + 0.01),
        Math.PI / 2 - 0.01,
      );
      this.onView?.(this.getOrbit());
    });
    this.canvas.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        this.zoomBy(event.deltaY > 0 ? 1.1 : 0.9);
        // zoomBy 是程序化命令（联动也会调它），用户滚轮这一路才回报。
        this.onView?.(this.getOrbit());
      },
      { passive: false },
    );
  }
}

function align4(size: number): number {
  return Math.ceil(size / 4) * 4;
}
