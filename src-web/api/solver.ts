/** 求解器 IPC：生成 OpenFOAM case 与运行环境探测。 */
import { invokeCommand } from "../utils/ipc";
import type { AnalysisStage, EnvironmentCheck, Material, ProcessSettings } from "../types";

interface GenerateCaseInput {
  geometryId: string;
  caseDir: string;
  material: Material;
  process: ProcessSettings;
  stage: AnalysisStage;
  cores: number;
}

/** 生成 OpenFOAM case（polyMesh + 场 + 字典）。 */
export function generateOpenfoamCase(input: GenerateCaseInput): Promise<string> {
  return invokeCommand("generate_openfoam_case", {
    geometryId: input.geometryId,
    caseDir: input.caseDir,
    material: input.material,
    process: input.process,
    stage: input.stage,
    cores: input.cores,
  });
}

/** 探测 OpenFOAM 运行时是否就绪（工具链 + foamRun 模块化求解器）。 */
export function probeOpenfoam(): Promise<EnvironmentCheck> {
  return invokeCommand("probe_openfoam");
}
