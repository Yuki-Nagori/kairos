/**
 * 新建项目对话框：模块级单例开合态与表单——原生菜单、自绘标题栏菜单、工具栏
 * 与命令面板都指向同一份状态。
 *
 * 工程落在「文档目录/kairos/<项目名>/<文件名>.kairos」：项目名决定目录，文件名
 * 决定工程文件，两者都可改；文件名默认跟随项目名，用户手动改过之后不再自动跟随。
 */
import { ref, watch } from "vue";
import { useProjectStore } from "../../stores/project";

const open = ref(false);
const name = ref("");
const fileName = ref("");
const error = ref<string | null>(null);
/** 文件名是否被手动编辑过：没编辑过时跟随项目名（每次打开对话框重置）。 */
let fileNameEdited = false;

watch(name, (value) => {
  if (!fileNameEdited) {
    fileName.value = value;
  }
});

export function useNewProjectDialog() {
  function show(): void {
    name.value = "";
    fileName.value = "";
    error.value = null;
    fileNameEdited = false;
    open.value = true;
  }

  function hide(): void {
    open.value = false;
  }

  /** 文件名输入被编辑：此后不再跟随项目名。 */
  function markFileNameEdited(): void {
    fileNameEdited = true;
  }

  /** 提交：项目名必填，文件名为空时用项目名；创建成功才收起对话框。 */
  async function submit(): Promise<void> {
    const trimmedName = name.value.trim();
    if (trimmedName === "") {
      error.value = "项目名不能为空。";
      return;
    }
    const trimmedFile = fileName.value.trim();
    error.value = null;
    const created = await useProjectStore().newProject(
      trimmedName,
      trimmedFile === "" ? trimmedName : trimmedFile,
    );
    if (created) {
      hide();
    }
    // 创建失败的原因由全局错误通道呈现（工作区路径校验等），对话框保持打开供修改。
  }

  return { open, name, fileName, error, show, hide, markFileNameEdited, submit };
}
