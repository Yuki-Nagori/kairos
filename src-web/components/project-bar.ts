import { button, hint, textInput } from "./ui";
import {
  appStore,
  addStudy,
  newProject,
  openProject,
  openProjectAtPath,
  removeStudy,
  saveProject,
  saveProjectAs,
} from "../state";

/** 项目栏：当前工程与研究的生命周期操作（新建/打开/保存/研究管理/最近项目）。 */
export function createProjectBar(): HTMLElement {
  const root = document.createElement("section");
  root.className =
    "flex flex-wrap items-center gap-3 rounded-2xl border border-zinc-800 bg-zinc-900 px-5 py-4 shadow-xl";

  const nameLabel = document.createElement("span");
  nameLabel.className = "text-sm font-semibold";
  const studiesBox = document.createElement("div");
  studiesBox.className = "flex flex-wrap items-center gap-2";
  const actions = document.createElement("div");
  actions.className = "ml-auto flex flex-wrap items-center gap-2";

  const newButton = button("新建", "primary");
  const openButton = button("打开");
  const saveButton = button("保存");
  const saveAsButton = button("另存为");
  const newProjectInput = textInput("项目名称");
  newProjectInput.className += " hidden";
  const newProjectConfirm = button("确定", "primary");
  newProjectConfirm.className += " hidden";
  const studyInput = textInput("新研究名称，如：填充分析");
  const addStudyButton = button("添加研究");
  const recentsSelect = document.createElement("select");
  recentsSelect.className =
    "rounded-lg border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs text-zinc-300";

  actions.append(newButton, openButton, saveButton, saveAsButton, recentsSelect);
  root.append(
    nameLabel,
    studiesBox,
    studyInput,
    addStudyButton,
    actions,
    newProjectInput,
    newProjectConfirm,
  );

  newButton.addEventListener("click", () => {
    // wry WebView 不支持 window.prompt，新建走内联输入。
    newProjectInput.classList.toggle("hidden");
    newProjectConfirm.classList.toggle("hidden");
    newProjectInput.focus();
  });
  const confirmNew = (): void => {
    const name = newProjectInput.value;
    void newProject(name || "未命名项目").then(() => {
      newProjectInput.classList.add("hidden");
      newProjectConfirm.classList.add("hidden");
      newProjectInput.value = "";
    });
  };
  newProjectConfirm.addEventListener("click", confirmNew);
  newProjectInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      confirmNew();
    }
  });
  openButton.addEventListener("click", () => void openProject());
  saveButton.addEventListener("click", () => void saveProject());
  saveAsButton.addEventListener("click", () => void saveProjectAs());
  addStudyButton.addEventListener("click", () => {
    addStudy(studyInput.value);
    studyInput.value = "";
  });
  studyInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      addStudy(studyInput.value);
      studyInput.value = "";
    }
  });
  recentsSelect.addEventListener("change", () => {
    const path = recentsSelect.value;
    recentsSelect.selectedIndex = 0;
    if (path) {
      void openProjectAtPath(path);
    }
  });

  function render(): void {
    const { project, projectPath, recents, busy } = appStore.get();
    const working = busy !== null;

    nameLabel.textContent = project ? project.name : "未打开项目";

    studiesBox.replaceChildren();
    if (project) {
      if (project.studies.length === 0) {
        studiesBox.append(hint("还没有研究，添加一个开始分析。"));
      }
      for (const study of project.studies) {
        const chip = document.createElement("span");
        chip.className =
          "flex items-center gap-1 rounded-full border border-emerald-500/60 bg-emerald-500/10 px-3 py-1 text-xs text-emerald-300";
        chip.textContent = study.name;
        const remove = document.createElement("button");
        remove.type = "button";
        remove.textContent = "✕";
        remove.title = `删除研究 ${study.name}`;
        remove.className = "text-emerald-400/70 hover:text-red-400";
        remove.addEventListener("click", () => removeStudy(study.id));
        chip.append(remove);
        studiesBox.append(chip);
      }
    }

    const noProject = project === null;
    studyInput.disabled = noProject || working;
    addStudyButton.disabled = noProject || working;
    saveButton.disabled = noProject || working;
    saveAsButton.disabled = noProject || working;
    newButton.disabled = working;
    openButton.disabled = working;

    recentsSelect.replaceChildren();
    const placeholder = document.createElement("option");
    placeholder.textContent = recents.length > 0 ? "最近打开…" : "暂无最近项目";
    placeholder.value = "";
    placeholder.disabled = true;
    placeholder.selected = true;
    recentsSelect.append(placeholder);
    for (const recent of recents) {
      const option = document.createElement("option");
      option.textContent = recent.name;
      option.value = recent.path;
      recentsSelect.append(option);
    }
    recentsSelect.disabled = recents.length === 0 || working;
    recentsSelect.title = projectPath ? `当前文件：${projectPath}` : "尚未保存到文件";
  }

  render();
  appStore.subscribe(render);
  return root;
}
