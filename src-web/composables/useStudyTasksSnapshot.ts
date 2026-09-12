/**
 * 方案任务快照（跨面板共享）：从各领域 store 组装评估输入，派生方案任务
 * 任务序列。方案任务窗格（列表 / 右键菜单）与阶段选项卡（状态角标）共用
 * 同一份派生结果，避免两处各写一份快照组装。
 */
import { computed } from "vue";
import { useGeometryStore } from "../stores/geometry";
import { useJobsStore } from "../stores/jobs";
import { useMaterialsStore } from "../stores/materials";
import { useProjectStore } from "../stores/project";
import { useResultsStore } from "../stores/results";
import { evaluateStudyTasks, type StudyTask } from "../utils/study-tasks";

export function useStudyTasksSnapshot() {
  const geometry = useGeometryStore();
  const jobsStore = useJobsStore();
  const materials = useMaterialsStore();
  const project = useProjectStore();
  const results = useResultsStore();

  const tasks = computed<StudyTask[]>(() =>
    evaluateStudyTasks({
      geometries: geometry.geometries,
      meshReports: geometry.meshReports,
      project: project.project,
      activeStudyId: project.activeStudyId,
      materials: materials.materials,
      jobs: jobsStore.jobs,
      resultCatalog: results.resultCatalog,
    }),
  );

  return { tasks };
}
