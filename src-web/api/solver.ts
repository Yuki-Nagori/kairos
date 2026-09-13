/** 求解器 IPC：生成 moldingFoam case 与运行环境探测。 */
import { invokeCommand } from "../utils/ipc";
import type {
  AnalysisStage,
  CaseOutcome,
  CoolingChannel,
  EnvironmentCheck,
  Material,
  ProcessSettings,
  RunnerElement,
} from "../types";

interface GenerateCaseInput {
  geometryId: string;
  caseDir: string;
  material: Material;
  process: ProcessSettings;
  stage: AnalysisStage;
  cores: number;
  /** 方案的模具网络：浇口单元决定 case 的 inlet patch。 */
  runnerElements: RunnerElement[];
  /** 方案的冷却水路：决定模壁 T 边界（模壁 1D 通道 BC）；空则恒定模温。 */
  coolingChannels: CoolingChannel[];
}

/** 生成 moldingFoam case（polyMesh + 场 + 字典），返回浇口入口口径回显与告警。 */
export function generateMoldingfoamCase(input: GenerateCaseInput): Promise<CaseOutcome> {
  return invokeCommand("generate_moldingfoam_case", {
    geometryId: input.geometryId,
    caseDir: input.caseDir,
    material: input.material,
    process: input.process,
    stage: input.stage,
    cores: input.cores,
    runnerElements: input.runnerElements,
    coolingChannels: input.coolingChannels,
  });
}

/** 探测 OpenFOAM 运行时是否就绪（工具链 + foamRun 模块化求解器）。 */
export function probeMoldingfoam(): Promise<EnvironmentCheck> {
  return invokeCommand("probe_moldingfoam");
}
