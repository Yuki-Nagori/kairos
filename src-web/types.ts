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
  dataNote: string;
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
  nodeCount: number;
  elementCount: number;
  surfaceFaceCount: number;
  totalVolume: number;
  quality: MeshQuality;
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
