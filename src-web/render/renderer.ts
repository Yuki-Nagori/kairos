import { mat4Identity, mat4LookAt, mat4Multiply, mat4Perspective, type Vec3 } from "./math";
import { THEME_CHANGED_EVENT } from "../theme";

/** 视口清屏色的深色缺省（CSS 变量缺失时的兜底）。 */
const FALLBACK_CLEAR: [number, number, number] = [0.06, 0.07, 0.09];

/** 把 #rrggbb 形式的 CSS 颜色解析为 0..1 的 RGB 分量。 */
function hexToRgb(hex: string): [number, number, number] | null {
  const match = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (match === null || match[1] === undefined) {
    return null;
  }
  const int = parseInt(match[1], 16);
  return [((int >> 16) & 0xff) / 255, ((int >> 8) & 0xff) / 255, (int & 0xff) / 255];
}

/** 渲染网格数据：扁平化顶点与三角形索引（可选每面标量值用于云图）。 */
interface RenderMesh {
  /** 扁平化顶点坐标（长度 = 3 × 顶点数）。 */
  positions: Float32Array;
  /** 三角形顶点索引（长度 = 3 × 三角形数）。 */
  indices: Uint32Array;
  /** 每个三角形所属单元索引（云图着色用；可为空）。 */
  faceCells?: Uint32Array;
}

/** 每顶点法向（按三角形面法向展开，非索引共享）。 */
function computeNormals(positions: Float32Array, indices: Uint32Array): Float32Array {
  const normals = new Float32Array(positions.length);
  for (let face = 0; face < indices.length; face += 3) {
    const a = (indices[face] ?? 0) * 3;
    const b = (indices[face + 1] ?? 0) * 3;
    const c = (indices[face + 2] ?? 0) * 3;
    const e1x = (positions[b] ?? 0) - (positions[a] ?? 0);
    const e1y = (positions[b + 1] ?? 0) - (positions[a + 1] ?? 0);
    const e1z = (positions[b + 2] ?? 0) - (positions[a + 2] ?? 0);
    const e2x = (positions[c] ?? 0) - (positions[a] ?? 0);
    const e2y = (positions[c + 1] ?? 0) - (positions[a + 1] ?? 0);
    const e2z = (positions[c + 2] ?? 0) - (positions[a + 2] ?? 0);
    const nx = e1y * e2z - e1z * e2y;
    const ny = e1z * e2x - e1x * e2z;
    const nz = e1x * e2y - e1y * e2x;
    for (const base of [a, b, c]) {
      normals[base] = (normals[base] ?? 0) + nx;
      normals[base + 1] = (normals[base + 1] ?? 0) + ny;
      normals[base + 2] = (normals[base + 2] ?? 0) + nz;
    }
  }
  for (let index = 0; index < normals.length; index += 3) {
    const length = Math.hypot(
      normals[index] ?? 0,
      normals[index + 1] ?? 0,
      normals[index + 2] ?? 0,
    );
    if (length > 0) {
      normals[index] = (normals[index] ?? 0) / length;
      normals[index + 1] = (normals[index + 1] ?? 0) / length;
      normals[index + 2] = (normals[index + 2] ?? 0) / length;
    }
  }
  return normals;
}

const VERTEX_SHADER = `#version 300 es
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_normal;
layout(location=2) in float a_value;
uniform mat4 u_mvp;
out vec3 v_normal;
out vec3 v_world;
out float v_value;
void main() {
  v_normal = a_normal;
  v_world = a_pos;
  v_value = a_value;
  gl_Position = u_mvp * vec4(a_pos, 1.0);
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision highp float;
in vec3 v_normal;
in vec3 v_world;
in float v_value;
uniform vec3 u_lightDir;
uniform vec3 u_colorCold;
uniform vec3 u_colorHot;
uniform float u_valueMin;
uniform float u_valueMax;
uniform int u_useField;
uniform int u_clipEnabled;
uniform float u_clipY;
out vec4 outColor;
void main() {
  if (u_clipEnabled == 1 && v_world.y > u_clipY) { discard; }
  vec3 n = normalize(v_normal);
  float diff = max(dot(n, normalize(u_lightDir)), 0.0);
  float t = clamp((v_value - u_valueMin) / max(u_valueMax - u_valueMin, 1e-6), 0.0, 1.0);
  vec3 fieldColor = mix(u_colorCold, u_colorHot, t);
  vec3 base = u_useField == 1 ? fieldColor : vec3(0.55, 0.58, 0.62);
  vec3 color = base * (0.35 + 0.65 * diff);
  outColor = vec4(color, 1.0);
}
`;

/** WebGL2 渲染器：轨道相机 + Lambert 着色 + 场云图 + 剖切 + FPS 埋点。 */
export class ViewportRenderer {
  private program: WebGLProgram;
  private vao: WebGLVertexArrayObject | null = null;
  private valueBuffer: WebGLBuffer | null = null;
  private indexCount = 0;
  private indexType = 0;

  private yaw = 0.6;
  private pitch = 0.4;
  private distance = 3;
  private target: Vec3 = [0, 0, 0];
  private clipY = 0;

  private useField = 0;
  private valueMin = 0;
  private valueMax = 1;
  private clipEnabled = 0;

  private frameTimes: number[] = [];
  private rafHandle = 0;
  private disposed = false;
  private clearColor: [number, number, number] = FALLBACK_CLEAR;
  /** 最后一次上传的网格：上下文恢复时全部 GL 资源需按它重建。 */
  private lastMesh: RenderMesh | null = null;
  private contextLost = false;
  private readonly onThemeChanged = (): void => {
    this.refreshClearColor();
  };
  private readonly onContextLost = (event: Event): void => {
    // preventDefault 是触发浏览器恢复流程的前提。
    event.preventDefault();
    this.contextLost = true;
    cancelAnimationFrame(this.rafHandle);
  };
  private readonly onContextRestored = (): void => {
    this.contextLost = false;
    if (this.lastMesh !== null) {
      this.uploadGlResources(this.lastMesh);
    } else {
      this.indexCount = 0;
    }
    this.startLoop();
  };

  private constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly gl: WebGL2RenderingContext,
    private readonly onFps?: (fps: number) => void,
  ) {
    this.program = this.buildProgram();
    this.attachControls();
    this.attachContextHandlers();
    this.refreshClearColor();
    window.addEventListener(THEME_CHANGED_EVENT, this.onThemeChanged);
    this.startLoop();
  }

  /** 创建渲染器；需要 WebGL2 上下文，失败返回 null（调用方展示回退提示）。 */
  static create(canvas: HTMLCanvasElement, onFps?: (fps: number) => void): ViewportRenderer | null {
    const gl = canvas.getContext("webgl2");
    if (gl === null) {
      return null;
    }
    return new ViewportRenderer(canvas, gl, onFps);
  }

  private buildProgram(): WebGLProgram {
    const gl = this.gl;
    const compile = (type: number, source: string): WebGLShader => {
      const shader = gl.createShader(type);
      if (shader === null) {
        throw new Error("无法创建着色器");
      }
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        const log = gl.getShaderInfoLog(shader) ?? "";
        gl.deleteShader(shader);
        throw new Error(`着色器编译失败：${log}`);
      }
      return shader;
    };
    const program = gl.createProgram();
    if (program === null) {
      throw new Error("无法创建着色程序");
    }
    gl.attachShader(program, compile(gl.VERTEX_SHADER, VERTEX_SHADER));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAGMENT_SHADER));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      throw new Error(`着色程序链接失败：${gl.getProgramInfoLog(program) ?? ""}`);
    }
    return program;
  }

  /** 上传渲染网格：扁平顶点 + 三角形索引（可选每面单元索引用于云图）。 */
  uploadMesh(mesh: RenderMesh): void {
    this.lastMesh = mesh;
    this.uploadGlResources(mesh);
  }

  /** 重建全部 GL 资源（首次上传与上下文恢复共用路径）。 */
  private uploadGlResources(mesh: RenderMesh): void {
    const gl = this.gl;
    this.program = this.buildProgram();
    // 恢复后旧句柄全部失效，VAO 与缓冲必须全新创建（不能用 ??= 复用）。
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);

    const positionBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.positions, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);

    const normalBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, normalBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, computeNormals(mesh.positions, mesh.indices), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 0, 0);

    this.valueBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.valueBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, perFaceValues(mesh), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 1, gl.FLOAT, false, 0, 0);

    const indexBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.indices, gl.STATIC_DRAW);

    this.indexCount = mesh.indices.length;
    this.indexType =
      mesh.indices instanceof Uint32Array ? this.gl.UNSIGNED_INT : this.gl.UNSIGNED_SHORT;
    gl.bindVertexArray(null);
    this.fitToMesh(mesh.positions);
  }

  /** 设置场云图范围（开启云图着色）。 */
  setFieldRange(min: number, max: number): void {
    this.valueMin = min;
    this.valueMax = max;
    this.useField = 1;
  }

  /** 热更新每面值（长度 = 面数）：时间步动画逐帧更新，不重建网格缓冲。 */
  setFaceValues(perFaceValues: Float32Array): void {
    const gl = this.gl;
    if (this.valueBuffer === null || this.indexCount === 0) {
      return;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.valueBuffer);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, perFaceValues);
  }

  /** 关闭云图着色。 */
  clearField(): void {
    this.useField = 0;
  }

  /** 剖切：丢弃 y > clipY 的片段。 */
  setClip(enabled: boolean, clipY: number): void {
    this.clipEnabled = enabled ? 1 : 0;
    this.clipY = clipY;
  }

  resetView(): void {
    this.yaw = 0.6;
    this.pitch = 0.4;
    this.distance = 3;
  }

  /** 停止渲染循环（面板卸载时调用）。 */
  dispose(): void {
    this.disposed = true;
    cancelAnimationFrame(this.rafHandle);
    window.removeEventListener(THEME_CHANGED_EVENT, this.onThemeChanged);
    this.canvas.removeEventListener("webglcontextlost", this.onContextLost);
    this.canvas.removeEventListener("webglcontextrestored", this.onContextRestored);
  }

  /** 注册 WebGL 上下文丢失 / 恢复监听：丢失时暂停渲染，恢复后按缓存网格重建全部资源。 */
  private attachContextHandlers(): void {
    this.canvas.addEventListener("webglcontextlost", this.onContextLost);
    this.canvas.addEventListener("webglcontextrestored", this.onContextRestored);
  }

  /** 清屏色取主题变量 --c-viewport-bg；变量缺失时保留深色兜底。 */
  private refreshClearColor(): void {
    if (typeof getComputedStyle !== "function") {
      return;
    }
    const raw = getComputedStyle(document.documentElement).getPropertyValue("--c-viewport-bg");
    const rgb = hexToRgb(raw);
    if (rgb !== null) {
      this.clearColor = rgb;
    }
  }

  private attachControls(): void {
    let dragMode: "orbit" | "pan" | null = null;
    let lastX = 0;
    let lastY = 0;

    this.canvas.addEventListener("mousedown", (event) => {
      dragMode = event.button === 2 ? "pan" : "orbit";
      lastX = event.clientX;
      lastY = event.clientY;
    });
    window.addEventListener("mouseup", () => {
      dragMode = null;
    });
    this.canvas.addEventListener("contextmenu", (event) => event.preventDefault());
    this.canvas.addEventListener("mousemove", (event) => {
      if (dragMode === null) {
        return;
      }
      const dx = event.clientX - lastX;
      const dy = event.clientY - lastY;
      lastX = event.clientX;
      lastY = event.clientY;
      if (dragMode === "orbit") {
        this.yaw += dx * 0.01;
        this.pitch = Math.min(1.5, Math.max(-1.5, this.pitch + dy * 0.01));
      } else {
        this.target[0] -= dx * 0.002 * this.distance;
        this.target[2] += dy * 0.002 * this.distance;
      }
    });
    this.canvas.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        this.distance *= event.deltaY > 0 ? 1.1 : 0.9;
      },
      { passive: false },
    );
  }

  private startLoop(): void {
    const frame = (timestamp: number) => {
      if (this.disposed) {
        return;
      }
      this.frameTimes.push(timestamp);
      while (this.frameTimes.length > 0 && timestamp - (this.frameTimes[0] ?? 0) > 1000) {
        this.frameTimes.shift();
      }
      if (this.onFps && this.frameTimes.length > 1) {
        this.onFps(this.frameTimes.length);
      }
      this.draw();
      this.rafHandle = requestAnimationFrame(frame);
    };
    this.rafHandle = requestAnimationFrame(frame);
  }

  private draw(): void {
    if (this.contextLost) {
      return;
    }
    const gl = this.gl;
    gl.enable(gl.DEPTH_TEST);
    const [clearR, clearG, clearB] = this.clearColor;
    gl.clearColor(clearR, clearG, clearB, 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    if (this.indexCount === 0) {
      return;
    }

    const aspect = this.canvas.width / Math.max(this.canvas.height, 1);
    const projection = mat4Perspective(Math.PI / 4, aspect, 0.01, 100);
    const eye: Vec3 = [
      this.target[0] + this.distance * Math.cos(this.pitch) * Math.sin(this.yaw),
      this.target[1] + this.distance * Math.sin(this.pitch),
      this.target[2] + this.distance * Math.cos(this.pitch) * Math.cos(this.yaw),
    ];
    const view = mat4LookAt(eye, this.target, [0, 1, 0]);
    const model = mat4Identity();
    const mvp = mat4Multiply(mat4Multiply(projection, view), model);

    gl.useProgram(this.program);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.program, "u_mvp"), false, mvp);
    gl.uniform3f(gl.getUniformLocation(this.program, "u_lightDir"), 0.4, 0.8, 0.6);
    gl.uniform3f(gl.getUniformLocation(this.program, "u_colorCold"), 0.15, 0.35, 0.85);
    gl.uniform3f(gl.getUniformLocation(this.program, "u_colorHot"), 0.95, 0.4, 0.1);
    gl.uniform1f(gl.getUniformLocation(this.program, "u_valueMin"), this.valueMin);
    gl.uniform1f(gl.getUniformLocation(this.program, "u_valueMax"), this.valueMax);
    gl.uniform1i(gl.getUniformLocation(this.program, "u_useField"), this.useField);
    gl.uniform1i(gl.getUniformLocation(this.program, "u_clipEnabled"), this.clipEnabled);
    gl.uniform1f(gl.getUniformLocation(this.program, "u_clipY"), this.clipY);

    gl.bindVertexArray(this.vao);
    gl.drawElements(gl.TRIANGLES, this.indexCount, this.indexType, 0);
    gl.bindVertexArray(null);
  }

  private fitToMesh(positions: Float32Array): void {
    let min = Infinity;
    let max = -Infinity;
    for (let index = 0; index < positions.length; index += 1) {
      min = Math.min(min, positions[index] ?? 0);
      max = Math.max(max, positions[index] ?? 0);
    }
    const span = Math.max(max - min, 1e-6);
    this.distance = span * 2.5;
    this.target = [min + span / 2, min + span / 2, min + span / 2];
    this.clipY = min + span / 2;
  }
}

function perFaceValues(mesh: RenderMesh): Float32Array {
  // 每面值复制到 3 个顶点；无 faceCells 时全部为 0（材质着色不受影响）。
  const faceCount = mesh.indices.length / 3;
  const values = new Float32Array(faceCount * 3);
  if (mesh.faceCells === undefined) {
    return values;
  }
  for (let face = 0; face < faceCount; face += 1) {
    const cellValue = mesh.faceCells[face] ?? 0;
    values[face * 3] = cellValue;
    values[face * 3 + 1] = cellValue;
    values[face * 3 + 2] = cellValue;
  }
  return values;
}
