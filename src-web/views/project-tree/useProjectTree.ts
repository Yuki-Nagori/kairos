/** 项目树面板：方案（可点击切换活跃研究）/ 几何 / 求解作业 / 运行时依赖。 */
import { computed } from "vue";
import { useDependenciesStore } from "../../stores/dependencies";
import { useGeometryStore } from "../../stores/geometry";
import { useJobsStore } from "../../stores/jobs";
import { useProjectStore } from "../../stores/project";

interface StudyLeaf {
  id: string;
  name: string;
  active: boolean;
}

export function useProjectTree() {
  const project = useProjectStore();
  const geometry = useGeometryStore();
  const jobsStore = useJobsStore();
  const deps = useDependenciesStore();

  /** 方案层（工程视图：工程 → 方案）。 */
  const studies = computed<StudyLeaf[]>(() =>
    (project.project?.studies ?? []).map((study) => ({
      id: study.id,
      name: study.name,
      active: study.id === project.activeStudyId,
    })),
  );

  /** 次级分组：空分组不渲染，保持树的紧凑。 */
  const groups = computed(() => {
    const sections: { title: string; leaves: string[] }[] = [
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

  /** 头部徽标：当前项目名（未打开时给占位）。 */
  const projectName = computed(() => project.project?.name ?? "未打开项目");

  /** 是否已打开工程：方案层与新建入口只在有工程时渲染。 */
  const hasProject = computed(() => project.project !== null);

  /** 点击方案 → 切换活跃研究（工程视图的方案选择）。 */
  function selectStudy(id: string): void {
    project.selectStudy(id);
  }

  /** 新建方案：默认名「方案 N」（N = 已有方案数 + 1），建好即为活跃方案。 */
  function createStudy(): void {
    project.addStudy(`方案 ${studies.value.length + 1}`);
  }

  return { studies, groups, projectName, hasProject, selectStudy, createStudy };
}
