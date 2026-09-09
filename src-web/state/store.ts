import { createStore } from "../lib/store";
import { IpcUnavailableError } from "../lib/ipc";
import type {
  ComponentStageState,
  DependencyStatus,
  VmAction,
  VmStatus,
  DownloadedEntry,
  GeometrySummary,
  Job,
  Material,
  MeshingReport,
  Project,
  RecentProject,
  ResultCatalog,
  ScalarField,
  SavedDownload,
  SystemInfo,
} from "../types";

/** 材料库切片：内置 + 自定义（详情见 T04）。 */
export interface MaterialLibrary {
  builtin: Material[];
  custom: Material[];
}

/** 探针：节点序号的命名标记（results 分片维护）。 */
export interface Probe {
  id: number;
  nodeIndex: number;
}

/** 全局应用状态：组件经 select/subscribe 订阅，只能通过各分片的动作函数修改。 */
export interface AppState {
  /** 应用信息，bootstrap 成功后填充；非 null 即代表 IPC 链路通畅。 */
  info: SystemInfo | null;
  /** 当前打开的工程文档；null 表示尚未打开（新建/打开后才有）。 */
  project: Project | null;
  /** 当前工程的保存路径；null 表示尚未保存过（保存时弹出另存为）。 */
  projectPath: string | null;
  /** 最近打开的工程（跨会话，来自应用数据目录）。 */
  recents: RecentProject[];
  /** 当前活跃研究（浇口 / 水路 / 工艺编辑的目标）。 */
  activeStudyId: string | null;
  /** 模具网络校验问题清单（校验按钮触发）。 */
  moldIssues: string[];
  /** 求解作业列表（调度器持有的快照）。 */
  jobs: Job[];
  /** 每作业的求解日志尾部（环形缓冲，key = 作业 id）。 */
  jobLogs: Record<string, string[]>;
  /** 运行时依赖状态（许可分级 + 就绪探测）。 */
  dependencies: DependencyStatus[];
  savedDownloads: Record<string, SavedDownload>;
  downloadedFiles: Record<string, DownloadedEntry>;
  /** 组件下载/编译流水线的阶段状态（key = 组件 id；成功后清除该条目）。 */
  componentStages: Record<string, ComponentStageState>;
  /** 虚拟机运行时状态（Multipass / WSL2 探测结果）。 */
  vmStatus: VmStatus | null;
  /** 应用内 Shell 输出（环形缓冲）。 */
  vmShellLogs: string[];
  /** 虚拟机面板进行中的动作（同一时刻至多一个）。 */
  vmBusy: VmAction | null;
  /** 虚拟机终端面板是否可见（状态栏右侧 Shell 按钮切换，默认隐藏）。 */
  vmPanelVisible: boolean;
  /** 探针列表（节点序号）。 */
  probes: Probe[];
  /** 材料库：内置示例材料 + 用户自定义材料。 */
  materials: MaterialLibrary;
  /** 已导入的几何（摘要列表，全量网格在 Rust 会话缓存）。 */
  geometries: GeometrySummary[];
  /** 每个几何的体积网格报告（key = geometryId）。 */
  meshReports: Record<string, MeshingReport>;
  /** 结果目录清单（扫描后填充）。 */
  resultCatalog: ResultCatalog | null;
  /** 最近加载的场（视口/图表展示用）。 */
  loadedField: ScalarField | null;
  /** 进行中的异步操作提示文案，标题栏展示；null 表示空闲。 */
  busy: string | null;
  /** 最近一次错误；info 为环境提示（浏览器预览，自动消失），否则是真实失败。 */
  error: { message: string; info: boolean } | null;
}

export const initialAppState: AppState = {
  info: null,
  project: null,
  projectPath: null,
  recents: [],
  materials: { builtin: [], custom: [] },
  geometries: [],
  meshReports: {},
  resultCatalog: null,
  loadedField: null,
  activeStudyId: null,
  moldIssues: [],
  jobs: [],
  jobLogs: {},
  dependencies: [],
  savedDownloads: {},
  downloadedFiles: {},
  componentStages: {},
  vmStatus: null,
  vmShellLogs: [],
  vmBusy: null,
  vmPanelVisible: false,
  probes: [],
  busy: null,
  error: null,
};

export const appStore = createStore<AppState>(initialAppState);

let errorTimer: ReturnType<typeof setTimeout> | undefined;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** 统一的错误入口：IPC 不可用是预期内的环境提示（自动消失），其余才是真实失败。 */
export function setError(error: unknown): void {
  const message = toMessage(error);
  const info = error instanceof IpcUnavailableError;
  if (errorTimer !== undefined) {
    clearTimeout(errorTimer);
  }
  appStore.set({ error: { message, info } });
  if (info) {
    errorTimer = setTimeout(() => {
      if (appStore.get().error?.message === message) {
        appStore.set({ error: null });
      }
    }, 8000);
  }
}
