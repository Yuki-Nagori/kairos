import { invokeCommand } from "../lib/ipc";
import type { CoolingChannel, RunnerElement } from "../types";

/** 校验流道 / 浇口 / 冷却水路网络，返回问题清单（空 = 通过）。 */
export function checkMoldNetwork(
  runnerElements: RunnerElement[],
  coolingChannels: CoolingChannel[],
): Promise<string[]> {
  return invokeCommand("check_mold_network", {
    runnerElements,
    coolingChannels,
  });
}
