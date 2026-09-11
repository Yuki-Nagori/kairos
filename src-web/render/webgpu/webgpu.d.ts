/** 最小 WebGPU 类型声明（POC 专用）：只声明本后端用到的运行时面，
 * 库内置 WebGPU 类型就绪后整个文件删除。全局环境声明，无模块导出。 */

type GpuTextureFormat = string;

type GpuBuffer = {
  destroy(): void;
};
type GpuShaderModule = object;
type GpuBindGroup = object;
type GpuBindGroupLayout = object;
type GpuTextureView = object;
type GpuCommandBuffer = object;
interface GpuRenderPipeline {
  getBindGroupLayout(index: number): GpuBindGroupLayout;
}
interface GpuBindGroupDescriptor {
  layout: GpuBindGroupLayout;
  entries: { binding: number; resource: { buffer: GpuBuffer; offset?: number; size?: number } }[];
}
interface GpuBindGroupEntry {
  binding: number;
  resource: GpuBindGroupEntryResource;
}
interface GpuBindGroupEntryResource {
  buffer: GpuBuffer;
}
type GpuTexture = {
  createView(): GpuTextureView;
  destroy(): void;
};

interface GpuBufferDescriptor {
  size: number;
  usage: number;
  mappedAtCreation?: boolean;
}

interface GpuRenderPassEncoder {
  setPipeline(pipeline: GpuRenderPipeline): void;
  setBindGroup(index: number, group: GpuBindGroup): void;
  setVertexBuffer(slot: number, buffer: GpuBuffer): void;
  setIndexBuffer(buffer: GpuBuffer, indexFormat: "uint32"): void;
  draw(count: number): void;
  drawIndexed(count: number): void;
  end(): void;
}

interface GpuRenderPassDescriptor {
  colorAttachments: {
    view: GpuTextureView;
    clearValue: { r: number; g: number; b: number; a: number };
    loadOp: "clear" | "load";
    storeOp: "store";
  }[];
  depthStencilAttachment?: {
    view: GpuTextureView;
    depthClearValue: number;
    depthLoadOp: "clear" | "load";
    depthStoreOp: "store";
  };
}

interface GpuCommandEncoder {
  beginRenderPass(descriptor: GpuRenderPassDescriptor): GpuRenderPassEncoder;
  finish(): GpuCommandBuffer;
}

interface GpuRenderPipelineDescriptor {
  layout: "auto";
  vertex: {
    module: GpuShaderModule;
    entryPoint: string;
    buffers?: {
      arrayStride: number;
      attributes: { shaderLocation: number; offset: number; format: string }[];
    }[];
  };
  fragment: {
    module: GpuShaderModule;
    entryPoint: string;
    targets: { format: GpuTextureFormat }[];
  };
  primitive: { topology: "triangle-list" | "line-list"; cullMode?: "none" | "back" };
  depthStencil?: {
    format: GpuTextureFormat;
    depthWriteEnabled: boolean;
    depthCompare: "less" | "greater";
  };
}

interface GpuDevice {
  readonly queue: {
    submit(commandBuffer: GpuCommandBuffer): number;
    writeBuffer(
      buffer: GpuBuffer,
      bufferOffset: number,
      data: ArrayBufferView,
      offset?: number,
      size?: number,
    ): void;
  };
  createBuffer(descriptor: GpuBufferDescriptor): GpuBuffer;
  createTexture(descriptor: {
    size: [number, number];
    format: GpuTextureFormat;
    usage: number;
  }): GpuTexture;
  createShaderModule(descriptor: { code: string }): GpuShaderModule;
  createRenderPipeline(descriptor: GpuRenderPipelineDescriptor): GpuRenderPipeline;
  createBindGroup(descriptor: {
    layout: GpuBindGroupLayout;
    entries: { binding: number; resource: { buffer: GpuBuffer; offset?: number; size?: number } }[];
  }): GpuBindGroup;
  createCommandEncoder(): GpuCommandEncoder;
  destroy(): void;
}

interface GpuAdapter {
  requestDevice(): Promise<GpuDevice>;
}

interface GpuCanvasContext {
  configure(descriptor: {
    device: GpuDevice;
    format: GpuTextureFormat;
    alphaMode: "opaque" | "premultiplied";
  }): void;
  getCurrentTexture(): GpuTexture;
}

interface Navigator {
  readonly gpu?: {
    requestAdapter(options?: { powerPreference?: "high-performance" }): Promise<GpuAdapter | null>;
    getPreferredCanvasFormat(): GpuTextureFormat;
  };
}

/** 缓冲 / 纹理用途位标志（运行时全局常量，值取 WebGPU 规范）。 */
declare const GPUBufferUsage: {
  readonly MAP_READ: number;
  readonly MAP_WRITE: number;
  readonly COPY_SRC: number;
  readonly COPY_DST: number;
  readonly INDEX: number;
  readonly VERTEX: number;
  readonly UNIFORM: number;
  readonly STORAGE: number;
};
declare const GPUTextureUsage: {
  readonly COPY_DST: number;
  readonly RENDER_ATTACHMENT: number;
};
