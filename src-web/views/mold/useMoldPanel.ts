/**
 * 模具网络面板：流道 / 浇口 + 冷却水路的编辑、列表与连通性校验，
 * 作用于活跃研究；未选研究或有操作进行中时整个表单禁用。
 * 坐标输入按「起点 xyz → 终点 xyz」成组，数值统一 mm（入口温度 °C）。
 */
import { computed, reactive, ref } from "vue";
import { useAppStore } from "../../stores/app";
import { useProjectStore } from "../../stores/project";
import type { CoolingChannel, RunnerElement, RunnerKind } from "../../types";

export function useMoldPanel() {
  // 坐标按 XYZ 键值存储（键为字面量联合，索引访问不引入 undefined）。
  const AXES = [
    { key: "x", label: "X" },
    { key: "y", label: "Y" },
    { key: "z", label: "Z" },
  ] as const;

  type AxisKey = (typeof AXES)[number]["key"];

  function zeroCoords(): Record<AxisKey, string> {
    return { x: "0", y: "0", z: "0" };
  }

  const app = useAppStore();
  const project = useProjectStore();

  const working = computed(() => app.busy !== null);

  const study = computed(() => project.activeStudy);

  const formDisabled = computed(() => study.value === null || working.value);

  // —— 流道 / 浇口表单 ——
  const runnerKind = ref("runner");
  const runnerDiameter = ref("6");
  const runnerStart = reactive(zeroCoords());
  const runnerEnd = reactive(zeroCoords());

  // —— 冷却水路表单 ——
  const channelDiameter = ref("8");
  const channelStart = reactive(zeroCoords());
  const channelEnd = reactive(zeroCoords());
  const inletTemp = ref("25");

  // 空串按 0 处理（与 Number("") === 0 的原生行为一致）。
  function xyz(values: Record<AxisKey, string>): [number, number, number] {
    return [Number(values.x ?? 0), Number(values.y ?? 0), Number(values.z ?? 0)];
  }

  function addRunner(): void {
    project.addRunnerElement(
      runnerKind.value as RunnerKind,
      Number(runnerDiameter.value),
      xyz(runnerStart),
      xyz(runnerEnd),
    );
  }

  function addChannel(): void {
    project.addCoolingChannel(
      Number(channelDiameter.value),
      xyz(channelStart),
      xyz(channelEnd),
      Number(inletTemp.value),
    );
  }

  function check(): void {
    void project.checkNetwork();
  }

  function runnerLabel(element: RunnerElement): string {
    return `${element.kind === "gate" ? "浇口" : "流道"} ${element.id} · Ø${element.diameterMm} mm`;
  }

  function channelLabel(channel: CoolingChannel): string {
    return `水路 ${channel.id} · Ø${channel.diameterMm} mm · ${channel.inletTempC}°C`;
  }

  return {
    AXES,
    project,
    formDisabled,
    study,
    runnerKind,
    runnerDiameter,
    runnerStart,
    runnerEnd,
    addRunner,
    runnerLabel,
    channelDiameter,
    channelStart,
    channelEnd,
    inletTemp,
    addChannel,
    channelLabel,
    check,
  };
}
