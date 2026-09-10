/** 层管理面板：制品网格与浇注系统 / 冷却水路图层的可见性切换（数据来自项目研究）。 */
import { computed } from "vue";
import { useGeometryStore } from "../../stores/geometry";
import { useProjectStore } from "../../stores/project";
import { useViewportStore, type LayerState } from "../../stores/viewport";

interface LayerRow {
  id: keyof LayerState;
  label: string;
  /** 数量注记（四面体数 / 单元数），null 不显示。 */
  count: number | null;
  /** 数据存在才可切换；无数据的层固定为熄灭。 */
  available: boolean;
  visible: boolean;
}

export function useLayersPanel() {
  const project = useProjectStore();
  const geometry = useGeometryStore();
  const viewport = useViewportStore();

  const rows = computed<LayerRow[]>(() => {
    const study = project.activeStudy;
    const gates = study?.runnerElements.filter((element) => element.kind === "gate") ?? [];
    const runners = study?.runnerElements.filter((element) => element.kind === "runner") ?? [];
    const channels = study?.coolingChannels ?? [];
    const report = Object.values(geometry.meshReports).at(-1);

    const definitions: {
      id: keyof LayerState;
      label: string;
      count: number | null;
      available: boolean;
    }[] = [
      {
        id: "mesh",
        label: "制品网格",
        count: report?.elementCount ?? null,
        available: report !== undefined,
      },
      { id: "gates", label: "浇口", count: gates.length, available: gates.length > 0 },
      { id: "runners", label: "流道", count: runners.length, available: runners.length > 0 },
      {
        id: "cooling",
        label: "冷却水路",
        count: channels.length,
        available: channels.length > 0,
      },
    ];
    return definitions.map((definition) => ({
      ...definition,
      visible: viewport.layers[definition.id],
    }));
  });

  /** 切换图层；无数据层不可切换（点击无效）。 */
  function toggle(row: LayerRow): void {
    if (!row.available) {
      return;
    }
    viewport.setLayerVisible(row.id, !row.visible);
  }

  return { rows, toggle };
}
