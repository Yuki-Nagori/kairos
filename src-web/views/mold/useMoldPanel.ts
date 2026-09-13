/**
 * 模具网络面板：流道 / 浇口 + 冷却水路的编辑、列表与连通性校验，
 * 作用于活跃方案；未选方案或有操作进行中时整个表单禁用。
 * 坐标输入按「起点 xyz → 终点 xyz」成组，数值统一 mm（入口温度 °C）。
 */
import { computed, reactive, ref, watch } from "vue";
import { useAppStore } from "../../stores/app";
import { useProjectStore } from "../../stores/project";
import { useViewportStore } from "../../stores/viewport";
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
  const viewport = useViewportStore();

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

  // 空输入按 0 处理（Number("") === 0），坐标允许留空。
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

  // —— 视口拾取放置 ——
  // 拾取点写入浇口**终点**（型腔端，与 case 生成的 GatePortal 取端一致），
  // 起点保持表单值：用户可先填起点，或拾取后再微调。
  const placementActive = computed(() => viewport.placement.active);
  const placementContinuous = computed(() => viewport.placement.continuous);
  /** 连续放置开关（按钮式切换，拾取后不自动退出）。 */
  function toggleContinuous(): void {
    viewport.placement = { ...viewport.placement, continuous: !viewport.placement.continuous };
  }
  const placementDisabledReason = computed(() => {
    if (study.value === null) {
      return "请先创建或选择一个方案。";
    }
    if (!viewport.meshLoaded) {
      return "请在视口载入网格后再拾取放置。";
    }
    return "";
  });
  const placementDisabled = computed(() => placementDisabledReason.value !== "" || working.value);
  /** 拾取结果提示：未拾取时给操作指引。 */
  const placementHint = computed(() => {
    if (placementDisabledReason.value !== "") {
      return placementDisabledReason.value;
    }
    if (!placementActive.value) {
      return viewport.placement.picks > 0
        ? `已拾取 ${viewport.placement.picks} 个点，最后一点已填入终点。`
        : "点击「视口拾取放置」后单击模型表面，坐标吸附到最近网格节点。";
    }
    return "放置模式：请在视口中单击模型表面，或点「取消拾取」退出。";
  });

  function startPlacement(): void {
    viewport.beginPlacement(viewport.placement.continuous);
  }

  function cancelPlacement(): void {
    viewport.cancelPlacement();
  }

  /** 拾取点回填终点坐标（面板显示 mm，与网格同单位）。 */
  watch(
    () => viewport.placement.picks,
    () => {
      const point = viewport.placement.point;
      if (point === null) {
        return;
      }
      runnerEnd.x = String(point[0]);
      runnerEnd.y = String(point[1]);
      runnerEnd.z = String(point[2]);
    },
  );

  function runnerLabel(element: RunnerElement): string {
    return `${element.kind === "gate" ? "浇口" : "流道"} ${element.id} · Ø${element.diameterMm} mm`;
  }

  function channelLabel(channel: CoolingChannel): string {
    return `水路 ${channel.id} · Ø${channel.diameterMm} mm · ${channel.inletTempC}°C`;
  }

  return {
    AXES,
    project,
    working,
    formDisabled,
    placementActive,
    placementContinuous,
    toggleContinuous,
    placementDisabled,
    placementHint,
    startPlacement,
    cancelPlacement,
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
