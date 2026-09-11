/** 渲染后端能力探测：WebGPU 可用则标记，否则回退 WebGL2（Tauri WebView 现状）。 */

type RenderBackend = "webgpu" | "webgl2" | "unsupported";

interface RenderCapability {
  backend: RenderBackend;
  /** 面向用户的说明（含回退原因）。 */
  note: string;
}

async function detectRenderCapability(
  hasWebGPU: boolean,
  hasWebGL2: boolean,
): Promise<RenderCapability> {
  if (hasWebGPU) {
    // WebGPU 适配器仍可能创建失败（驱动/黑名单），由渲染器二次兜底。
    return { backend: "webgpu", note: "WebGPU 后端" };
  }
  if (hasWebGL2) {
    return {
      backend: "webgl2",
      note: "WebGPU 不可用，已回退 WebGL2（当前 WebView 尚未支持 WebGPU）。",
    };
  }
  return { backend: "unsupported", note: "当前环境不支持硬件渲染。" };
}

/** 浏览器环境探测入口：WebGPU 以真实适配器请求为准（"gpu" 存在 ≠ 可用）。 */
export async function detectRenderCapabilityInBrowser(): Promise<RenderCapability> {
  const gpu = (navigator as Navigator & { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
  if (gpu !== undefined) {
    try {
      const adapter = await gpu.requestAdapter();
      if (adapter !== null && adapter !== undefined) {
        return { backend: "webgpu", note: "WebGPU 后端" };
      }
    } catch {
      // 适配器探测失败按 WebGPU 不可用处理，继续回退探测。
    }
  }
  let hasWebGL2 = false;
  try {
    const canvas = document.createElement("canvas");
    hasWebGL2 = canvas.getContext("webgl2") !== null;
  } catch {
    hasWebGL2 = false;
  }
  return detectRenderCapability(false, hasWebGL2);
}
