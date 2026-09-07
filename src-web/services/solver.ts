import { invokeCommand } from "../lib/ipc";
import type { AnalysisStage, Material, ProcessSettings } from "../types";

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

interface EnvironmentCheck {
  openfoam: boolean;
  solver: boolean;
  hint: string;
}

/** 探测 OpenFOAM / openInjMoldSim 运行时是否就绪。 */
export function probeOpenfoam(): Promise<EnvironmentCheck> {
  return invokeCommand("probe_openfoam");
}
