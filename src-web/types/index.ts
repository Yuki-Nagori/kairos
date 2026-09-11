/** 与 Rust 侧 serde 结构一一对应的共享类型。 */

/** 对应 `kairos-core::models::system::SystemInfo`（camelCase 序列化契约）。 */
export interface SystemInfo {
  name: string;
  version: string;
  os: string;
}

/** 对应 `kairos-core::models::project::Project`（.kairos 工程文件 schema v1）。 */
export interface Project {
  schemaVersion: number;
  id: string;
  name: string;
  createdMs: number;
  updatedMs: number;
  studies: Study[];
}

/** 对应 `kairos-core::models::project::Study`。 */
export interface Study {
  id: string;
  name: string;
  createdMs: number;
  runnerElements: RunnerElement[];
  coolingChannels: CoolingChannel[];
  process: ProcessSettings | null;
  materialId: string | null;
}

/** 对应 `kairos-core::models::process::ProcessSettings`。 */
export interface ProcessSettings {
  meltTempC: number;
  moldTempC: number;
  ejectionTempC: number;
  injectionTimeS: number;
  vpSwitchVolumePercent: number;
  packingPressureMpaCurve: [number, number][];
  packingTimeS: number;
  coolingTimeS: number;
  coolantTempC: number;
}

/** 对应 `kairos-core::models::project::RecentProject`（最近打开的工程）。 */
export interface RecentProject {
  path: string;
  name: string;
  lastOpenedMs: number;
}
/** 对应 `kairos-core::models::material::CrossWlf`。 */
export interface CrossWlf {
  n: number;
  tauStar: number;
  d1: number;
  d2: number;
  d3: number;
  a1: number;
  a2: number;
}

/** 对应 `kairos-core::models::material::Tait`。 */
export interface Tait {
  b1m: number;
  b1s: number;
  b2m: number;
  b2s: number;
  b3: number;
  b4m: number;
  b4s: number;
  b5: number;
}

/** 温度相关的性质表 [温度 K, 值]。 */
export type PropertyTable = [number, number][];

/** 对应 `kairos-core::models::material::Mechanics`。 */
export interface Mechanics {
  elasticModulus: number;
  poissonRatio: number;
}

/** 对应 `kairos-core::models::material::FillerGroup`（纤维 / 填料参数组）。 */
export interface FillerGroup {
  /** 填料类型，如「玻纤」「碳纤维」「滑石粉」。 */
  kind: string;
  /** 质量分数 (0, 1]。 */
  weightFraction: number;
  /** 平均长径比（非纤维填料填 1）。 */
  aspectRatio: number;
  /** 数据来源或工艺提示。 */
  note: string;
}

/** 对应 `kairos-core::models::material::Material`。 */
export interface Material {
  id: string;
  name: string;
  manufacturer: string;
  family: string;
  rheology: CrossWlf;
  pvt: Tait;
  specificHeat: PropertyTable;
  conductivity: PropertyTable;
  mechanics: Mechanics | null;
  /** 纤维 / 填料参数组（无填料牌号为 null）。 */
  filler: FillerGroup | null;
  dataNote: string;
}

/** 材料库：内置参考牌号 + 自定义材料。 */
export interface MaterialLibrary {
  builtin: Material[];
  custom: Material[];
}
/** 对应 `kairos-core::models::geometry::MeshIssues`。 */
export interface MeshIssues {
  degenerate: number;
  openEdges: number;
  nonManifoldEdges: number;
  normalInconsistentEdges: number;
}

/** 对应 `kairos-core::models::geometry::GeometrySummary`（全量网格留在 Rust 会话缓存）。 */
export interface GeometrySummary {
  geometryId: string;
  fileName: string;
  triangleCount: number;
  size: [number, number, number];
  surfaceArea: number;
  signedVolume: number;
  suggestedUnit: string;
  issues: MeshIssues;
}
/** 对应 `kairos-core::models::mesh::MeshQuality`。 */
export interface MeshQuality {
  minEdgeRatio: number;
  avgEdgeRatio: number;
  maxEdgeRatio: number;
  minVolume: number;
}

/** 对应 `kairos-core::models::mesh::MeshingReport`。 */
export interface MeshingReport {
  /** 生成引擎标识（voxel / gmsh）。 */
  engine: string;
  nodeCount: number;
  elementCount: number;
  surfaceFaceCount: number;
  totalVolume: number;
  quality: MeshQuality;
}

/** 对应 `kairos-core::models::mesh::DualDomainReport`。 */
export interface DualDomainReport {
  nodeCount: number;
  triangleCount: number;
  beamCount: number;
  /** 成功捕捉到表面节点的梁端点数。 */
  couplingCount: number;
  /** 未捕捉（自成为自由节点）的梁端点数。 */
  uncoupledEndpoints: number;
  /** 未配对到对面（厚度为 0）的表面三角形数。 */
  unpairedTriangles: number;
  thicknessMin: number;
  thicknessMax: number;
  thicknessAvg: number;
}

/** 对应 `kairos-core::models::mesh::RefineRegion`。 */
export interface RefineRegion {
  min: [number, number, number];
  max: [number, number, number];
}

/** 对应 `kairos-core::models::mesh::MeshRefinement`（体素引擎分级加密）。 */
export type MeshRefinement =
  | { mode: "boundaryLayers"; layers: number; ratio: number }
  | { mode: "region"; region: RefineRegion; levels: number };

/** 对应 `kairos-core::models::mesh::MidplaneReport`。 */
export interface MidplaneReport {
  nodeCount: number;
  elementCount: number;
  beamCount: number;
  /** 成功捕捉到中面节点的梁端点数。 */
  couplingCount: number;
  /** 未捕捉（自成为自由节点）的梁端点数。 */
  uncoupledEndpoints: number;
  /** 未配对到对面的表面顶点数。 */
  unpairedVertices: number;
  /** 因顶点未配对而被丢弃的单元数。 */
  droppedElements: number;
  thicknessMin: number;
  thicknessMax: number;
  thicknessAvg: number;
}

/** 对应 `kairos-core::models::runners::RunnerKind`。 */
export type RunnerKind = "gate" | "runner";

/** 对应 `kairos-core::models::runners::RunnerElement`。 */
export interface RunnerElement {
  id: string;
  kind: RunnerKind;
  diameterMm: number;
  start: [number, number, number];
  end: [number, number, number];
}

/** 对应 `kairos-core::models::runners::CoolingChannel`。 */
export interface CoolingChannel {
  id: string;
  diameterMm: number;
  start: [number, number, number];
  end: [number, number, number];
  inletTempC: number;
}
/** 对应 `kairos-core::models::jobs::JobStatus`。 */
export type JobStatus = "queued" | "running" | "done" | "failed" | "cancelled";

/** 对应 `kairos-core::models::jobs::Job`。 */
export interface Job {
  id: string;
  studyId: string | null;
  caseDir: string;
  cores: number;
  status: JobStatus;
  createdMs: number;
  startedMs: number | null;
  finishedMs: number | null;
  lastTimeS: number | null;
  message: string | null;
}
/** 对应 `kairos-core::models::solver::AnalysisStage`。 */
export type AnalysisStage = "fill" | "fill_pack" | "fill_pack_cool";
/** 对应 `kairos-core::models::results::TimeStepMeta`。 */
export interface TimeStepMeta {
  dirName: string;
  timeS: number;
  fields: string[];
}

/** 对应 `kairos-core::models::results::ResultCatalog`。 */
export interface ResultCatalog {
  caseDir: string;
  times: TimeStepMeta[];
}

/** 对应 `kairos-core::models::results::ScalarField`。 */
export interface ScalarField {
  field: string;
  timeDir: string;
  timeS: number;
  values: number[];
  isMagnitude: boolean;
  complete: boolean;
}

/** 探针：节点序号的命名标记（results store 维护）。 */
export interface Probe {
  id: number;
  nodeIndex: number;
}
/** 对应 `kairos-core::models::dependencies::LicenseKind`。 */
export type LicenseKind = "mit" | "gpl";

/** 对应 `kairos-core::models::dependencies::InstallStrategy`。 */
export type InstallStrategy = "direct_download" | "guided_install";

/** 对应 `kairos-core::models::dependencies::DownloadSpec`（按平台的下载地址）。 */
export interface DownloadSpec {
  macos: string;
  windows: string;
  linux: string;
}

/** 对应 `kairos-core::models::dependencies::UpdateCheck`（在线更新检查结果）。 */
export interface UpdateCheck {
  componentId: string;
  installedTag: string | null;
  latestTag: string | null;
  updateAvailable: boolean;
}

/** 组件下载流水线的阶段状态（仅进行中或失败时存在；成功后清除）。 */
export type ComponentStageState =
  { stage: "downloading"; percent: number } | { stage: "failed"; error: string };

/** 对应 `kairos-core::models::vm`：虚拟机 provider（平台固定；native = Linux
 * 原生环境，无虚拟机，Shell 即本地 bash）与实例状态。 */
export type VmProvider = "multipass" | "wsl" | "native";
export type VmState = "missing" | "stopped" | "starting" | "running" | "unknown";
export interface VmStatus {
  provider: VmProvider;
  toolInstalled: boolean;
  instanceName: string;
  instanceState: VmState;
  hint: string;
}

/** 虚拟机面板一次只允许一个进行中动作。 */
export type VmAction = "install" | "start" | "shell" | "stop";

/** 下载完成后的落盘信息（压缩包自动解压后 extractDir 指向组件目录）。 */
export interface SavedDownload {
  path: string;
  fileName: string;
  sizeBytes: number;
  extractDir: string | null;
  /** release 流组件的来源版本标签；静态直链组件为 null。 */
  releaseTag: string | null;
}

/** 跨会话的下载清单条目（manifest.json，key = 组件 id）。 */
export interface DownloadedEntry {
  fileName: string;
  sizeBytes: number;
  downloadedAtMs: number;
  extractDir: string | null;
}

/** 依赖状态视图（目录项 + 就绪探测）。 */
export interface DependencyStatus {
  id: string;
  name: string;
  license: string;
  licenseKind: LicenseKind;
  strategy: InstallStrategy;
  pageUrl: string;
  required: boolean;
  checkCommand: string;
  hint: string;
  download: DownloadSpec | null;
  ready: boolean;
  /** 应用内受管目录中检测到可执行副本（下载 + 解压后即可用）。 */
  managedReady: boolean;
  /** 是否来自可在线检查更新的 release 流。 */
  updatable: boolean;
}

/** 分析阶段选项卡（工作流导航；home = 总览显示全部面板）。 */
export type Stage = "home" | "geometry" | "mesh" | "process" | "solve" | "results" | "report";
