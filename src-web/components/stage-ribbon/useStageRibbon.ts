/**
 * 阶段功能工具条逻辑：按分析阶段渲染命令分组（同一命令注册表的动作集）。
 * 无工具的阶段返回空分组，由模板渲染阶段引导文案保持工具条高度稳定。
 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import { useGeometryStore } from "../../stores/geometry";
import { useJobsStore } from "../../stores/jobs";
import { useResultsStore } from "../../stores/results";
import { runMenuAction } from "../../menu-actions";
import type { Stage } from "../../types";

/** 工具条按钮：icon 为设计稿指定的字符字形，disabled 动态求值。 */
interface RibbonButton {
  id: string;
  label: string;
  icon: string;
  disabled?: () => boolean;
  run: () => void;
}

interface RibbonGroup {
  buttons: RibbonButton[];
}

/** 各阶段的阶段引导文案（无工具时显示）。 */
export const STAGE_HINTS: Record<Stage, string> = {
  home: "从左侧流程面板开始：导入几何 → 生成网格 → 选择材料 → 设置工艺 → 提交求解。",
  geometry: "导入 STL 或样例几何后，在网格阶段生成体积网格。",
  mesh: "在几何面板导入几何后，选择网格引擎与目标尺寸生成体积网格；网格日志见中列下方。",
  process: "在材料与工艺面板完成参数设置并应用到研究；浇注系统在模具网络面板校验。",
  solve: "在求解作业面板提交作业；进度流式显示在方案任务与状态栏，日志见中列下方。",
  results: "求解完成后在结果面板扫描 case 目录，加载时间步场数据查看云图与曲线。",
  report: "在报告面板汇总项目 / 材料 / 工艺 / 结果快照，生成自包含 HTML 报告。",
};

export function useStageRibbon() {
  const app = useAppStore();
  const geometry = useGeometryStore();
  const jobs = useJobsStore();
  const results = useResultsStore();

  const groups = computed<RibbonGroup[]>(() => {
    const busy = (): boolean => app.busy !== null;
    switch (app.stage) {
      case "home":
        return [
          {
            buttons: [
              {
                id: "home.new",
                label: "新建项目",
                icon: "＋",
                run: () => runMenuAction("file.new"),
              },
              {
                id: "home.open",
                label: "打开项目",
                icon: "⌂",
                run: () => runMenuAction("file.open"),
              },
              {
                id: "home.save",
                label: "保存",
                icon: "⇩",
                run: () => runMenuAction("file.save"),
              },
            ],
          },
          {
            buttons: [
              {
                id: "home.network",
                label: "校验模具网络",
                icon: "∩",
                run: () => runMenuAction("analysis.checkNetwork"),
              },
              {
                id: "home.deps",
                label: "探测依赖",
                icon: "⟳",
                run: () => runMenuAction("tools.refreshDeps"),
              },
            ],
          },
        ];
      case "geometry":
        return [
          {
            buttons: [
              {
                id: "geo.import",
                label: "导入 STL",
                icon: "⇧",
                disabled: busy,
                run: () => void geometry.importGeometry(),
              },
              {
                id: "geo.sample",
                label: "导入样例",
                icon: "▣",
                disabled: busy,
                run: () => void geometry.importSampleGeometry(),
              },
            ],
          },
        ];
      case "process":
        return [
          {
            buttons: [
              {
                id: "process.network",
                label: "校验模具网络",
                icon: "∩",
                run: () => runMenuAction("analysis.checkNetwork"),
              },
            ],
          },
        ];
      case "solve":
        return [
          {
            buttons: [
              {
                id: "solve.refresh",
                label: "刷新作业",
                icon: "⟳",
                disabled: busy,
                run: () => void jobs.refreshJobs(),
              },
            ],
          },
          {
            buttons: [
              {
                id: "solve.deps",
                label: "探测依赖",
                icon: "◉",
                run: () => runMenuAction("tools.refreshDeps"),
              },
            ],
          },
        ];
      case "results":
        return [
          {
            buttons: [
              {
                id: "results.export",
                label: "导出 CSV",
                icon: "⇩",
                disabled: () => results.loadedField === null,
                run: () => runMenuAction("results.exportCsv"),
              },
              {
                id: "results.rescan",
                label: "重新扫描",
                icon: "⟳",
                disabled: busy,
                run: () => void results.rescanCatalog(),
              },
            ],
          },
        ];
      default:
        return [];
    }
  });

  const hint = computed(() => STAGE_HINTS[app.stage]);

  return { groups, hint };
}
