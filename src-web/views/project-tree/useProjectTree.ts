import { computed } from "vue";
import { useDependenciesStore } from "../../stores/dependencies";
import { useGeometryStore } from "../../stores/geometry";
import { useJobsStore } from "../../stores/jobs";
import { useProjectStore } from "../../stores/project";

/** 项目树面板逻辑：层级展示工程 / 几何 / 求解作业 / 运行时依赖。 */
export function useProjectTree() {
  const project = useProjectStore();
  const geometry = useGeometryStore();
  const jobsStore = useJobsStore();
  const deps = useDependenciesStore();

  // 空分组不渲染，保持树的紧凑；项目组恒在（未打开时给占位叶）。
  const groups = computed(() => {
    const sections: { title: string; leaves: string[] }[] = [
      { title: "项目", leaves: [project.project?.name ?? "未打开项目"] },
      {
        title: "几何",
        leaves: geometry.geometries.map((geo) => `${geo.fileName} (${geo.triangleCount} 面)`),
      },
      { title: "求解作业", leaves: jobsStore.jobs.map((job) => `${job.id}: ${job.status}`) },
      {
        title: "运行时依赖",
        leaves: deps.dependencies.map((dep) => `${dep.name}: ${dep.ready ? "就绪" : "未就绪"}`),
      },
    ];
    return sections.filter((section) => section.leaves.length > 0);
  });

  return { groups };
}
