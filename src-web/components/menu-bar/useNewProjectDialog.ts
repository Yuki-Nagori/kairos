/**
 * 新建项目对话框：模块级单例开合态与表单——原生菜单、自绘标题栏菜单、工具栏
 * 与命令面板都指向同一份状态。
 *
 * 工程落在「<工作区根>/<项目名>/<项目名>.kairos」：工作区默认 `<文档目录>/kairos`，
 * 可改成任意目录（含外接盘）；文件名与项目名一致，不单独设置。
 */
import { computed, ref } from "vue";
import { useProjectStore } from "../../stores/project";

const open = ref(false);
const workspace = ref("");
const name = ref("");
const error = ref<string | null>(null);

/** 落点预览：工作区 + 项目名 → 工程目录与工程文件（未填部分用占位符）。
 *  纯展示串，末尾分隔符只是排版问题，不参与落盘路径计算（那是 Rust 的事）。 */
const placement = computed(() => {
  const base = workspace.value.trim().replace(/[/\\]+$/, "") || "<工作区>";
  const project = name.value.trim() || "<项目名>";
  return `${base}/${project}/${project}.kairos`;
});

export function useNewProjectDialog() {
  /** 打开：工作区初值取平台默认根（`<文档目录>/kairos`，目录名随语言变化）。 */
  async function show(): Promise<void> {
    name.value = "";
    error.value = null;
    open.value = true;
    const fallback = workspace.value;
    const resolved = await useProjectStore().defaultWorkspace();
    // 取不到默认根（IPC 不可用等）：保留上次值，用户可手填。
    workspace.value = resolved ?? fallback;
  }

  function hide(): void {
    open.value = false;
  }

  /** 用系统目录选择器改工作区；取消保持原值。 */
  async function browseWorkspace(): Promise<void> {
    const picked = await useProjectStore().chooseWorkspaceDir(workspace.value);
    if (picked !== null) {
      workspace.value = picked;
    }
  }

  /** 提交：项目名必填；创建成功才收起对话框。 */
  async function submit(): Promise<void> {
    const trimmedName = name.value.trim();
    if (trimmedName === "") {
      error.value = "项目名不能为空。";
      return;
    }
    error.value = null;
    const created = await useProjectStore().newProject(trimmedName, workspace.value);
    if (created) {
      hide();
    }
    // 创建失败的原因由全局错误通道呈现（工作区不可写等），对话框保持打开供修改。
  }

  return { open, workspace, name, error, placement, show, hide, browseWorkspace, submit };
}
