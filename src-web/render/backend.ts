/** 视口后端工厂：优先自研 WebGPU（主路径），不可用回退 WebGL2。
 * 两个后端实现同一公共方法面（ViewportBackend），面板逻辑不感知差异。 */
import { ViewportRenderer } from "./renderer";
import { WebGPURenderer } from "./webgpu/renderer";
import type { Vec3 } from "./math";

export interface ViewportBackend {
  uploadMesh(mesh: {
    positions: Float32Array;
    indices: Uint32Array;
    faceCells?: Uint32Array;
  }): void;
  setFaceValues(perFace: Float32Array): void;
  setFieldRange(min: number, max: number): void;
  setClipPlane(enabled: boolean, normal: Vec3, offset: number): void;
  uploadOverlay(
    id: string,
    layer: { positions: Float32Array; color: [number, number, number] },
  ): void;
  setMeshVisible(visible: boolean): void;
  setOverlayVisible(id: string, visible: boolean): void;
  getCamera(): {
    eye: Vec3;
    target: Vec3;
    fovY: number;
    aspect: number;
    width: number;
    height: number;
  };
  /** 轨道相机快照 / 恢复：多视口联动时把源视口相机复制到其余视口。 */
  getOrbit(): {
    x: number;
    y: number;
    z: number;
    yaw: number;
    pitch: number;
    distance: number;
  };
  setOrbit(orbit: {
    x: number;
    y: number;
    z: number;
    yaw: number;
    pitch: number;
    distance: number;
  }): void;
  getMeshBounds(): { min: Vec3; max: Vec3 } | null;
  resetView(): void;
  zoomBy(factor: number): void;
  fitView(): void;
  dispose(): void;
}

type BackendKind = "webgpu" | "webgl2";

/** 创建视口后端：WebGPU 适配器可用即用之，否则回退 WebGL2；全败返回 null。 */
export async function createViewportRenderer(
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
): Promise<{ backend: ViewportBackend; kind: BackendKind } | null> {
  const gpu = await WebGPURenderer.create(canvas, onFps);
  if (gpu !== null) {
    return { backend: gpu, kind: "webgpu" };
  }
  const gl = ViewportRenderer.create(canvas, onFps, onView);
  if (gl !== null) {
    return { backend: gl, kind: "webgl2" };
  }
  return null;
}
