import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ProcessPanel from "../../../../src-web/views/process/ProcessPanel.vue";
import { useProcessPanel } from "../../../../src-web/views/process/useProcessPanel";
import { useAppStore } from "../../../../src-web/stores/app";
import { useProjectStore } from "../../../../src-web/stores/project";
import { checkProcess } from "../../../../src-web/api/process";
import type { ProcessSettings, Project, Study } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/process", () => ({
  checkProcess: vi.fn(),
}));

function processP1(): ProcessSettings {
  return {
    meltTempC: 230,
    moldTempC: 40,
    ejectionTempC: 90,
    injectionTimeS: 1.5,
    vpSwitchVolumePercent: 96,
    packingPressureMpaCurve: [
      [0, 60],
      [8, 48],
    ],
    packingTimeS: 8,
    coolingTimeS: 15,
    coolantTempC: 25,
  };
}

function processP2(): ProcessSettings {
  return {
    meltTempC: 255,
    moldTempC: 45,
    ejectionTempC: 95,
    injectionTimeS: 2,
    vpSwitchVolumePercent: 97,
    packingPressureMpaCurve: [
      [0, 50],
      [5, 30],
    ],
    packingTimeS: 5,
    coolingTimeS: 12,
    coolantTempC: 20,
  };
}

function studyFixture(id: string, process: ProcessSettings | null): Study {
  return {
    id,
    name: `研究-${id}`,
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process,
    materialId: null,
  };
}

function projectFixture(studies: Study[]): Project {
  return {
    schemaVersion: 1,
    id: "p-1",
    name: "演示项目",
    createdMs: 1,
    updatedMs: 1,
    studies,
  };
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

/** 字段输入框：0-8 为工艺字段（顺序同 FIELDS），9 为预设名称。 */
function fieldInputs(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll("input");
}

describe("ProcessPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    localStorage.clear();
  });

  it("挂载即回填活跃研究的工艺；切换研究时重新回填，无工艺则保持", async () => {
    const project = useProjectStore();
    project.project = projectFixture([
      studyFixture("study-1", processP1()),
      studyFixture("study-2", processP2()),
      studyFixture("study-3", null),
    ]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });

    let inputs = fieldInputs(wrapper);
    expect(inputs[0]?.element.value).toBe("230");
    expect(inputs[5]?.element.value).toBe("48"); // 保压压力取曲线末点

    // 切到研究 2：整表重新回填。
    project.activeStudyId = "study-2";
    await nextTick();
    inputs = fieldInputs(wrapper);
    expect(inputs[0]?.element.value).toBe("255");
    expect(inputs[5]?.element.value).toBe("30");

    // 切到无工艺的研究：不回填，保留上一份表单。
    project.activeStudyId = "study-3";
    await nextTick();
    inputs = fieldInputs(wrapper);
    expect(inputs[0]?.element.value).toBe("255");

    // 切到不存在的 id：同样不回填。
    project.activeStudyId = "ghost";
    await nextTick();
    inputs = fieldInputs(wrapper);
    expect(inputs[0]?.element.value).toBe("255");
  });

  it("回填时保压曲线为空则落 60 兜底", () => {
    const project = useProjectStore();
    project.project = projectFixture([
      studyFixture("study-4", { ...processP1(), packingPressureMpaCurve: [] }),
    ]);
    project.activeStudyId = "study-4";
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });

    expect(fieldInputs(wrapper)[5]?.element.value).toBe("60");
  });

  it("校验通过后应用到活跃研究：曲线末点取 80% 保压压力", async () => {
    vi.mocked(checkProcess).mockResolvedValue([]);
    const project = useProjectStore();
    project.project = projectFixture([
      studyFixture("study-1", null),
      studyFixture("study-2", processP2()),
    ]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });

    const inputs = fieldInputs(wrapper);
    await inputs[0]?.setValue("250"); // meltTempC
    await inputs[1]?.setValue("60"); // moldTempC
    await inputs[2]?.setValue("100"); // ejectionTempC
    await inputs[3]?.setValue("2"); // injectionTimeS
    await inputs[4]?.setValue("95"); // vpSwitchVolumePercent
    await inputs[5]?.setValue("55"); // packingPressureMpa
    await inputs[6]?.setValue("8"); // packingTimeS
    await inputs[7]?.setValue("20"); // coolingTimeS
    await inputs[8]?.setValue("30"); // coolantTempC
    await findButton(wrapper, "校验并应用到研究").trigger("click");
    await flushPromises();

    const expected: ProcessSettings = {
      meltTempC: 250,
      moldTempC: 60,
      ejectionTempC: 100,
      injectionTimeS: 2,
      vpSwitchVolumePercent: 95,
      packingPressureMpaCurve: [
        [0, 55],
        [8, 44],
      ],
      packingTimeS: 8,
      coolingTimeS: 20,
      coolantTempC: 30,
    };
    expect(checkProcess).toHaveBeenCalledWith(expected);
    expect(project.project?.studies[0]?.process).toEqual(expected);
    // 非活跃研究不受影响。
    expect(project.project?.studies[1]?.process).toEqual(processP2());
    expect(wrapper.text()).toContain("已应用到当前研究");
    expect(useAppStore().error).toBeNull();
  });

  it("校验发现问题：渲染问题清单且不写入研究", async () => {
    vi.mocked(checkProcess).mockResolvedValue(["熔体温度超过上限"]);
    const project = useProjectStore();
    project.project = projectFixture([studyFixture("study-1", null)]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });

    await fieldInputs(wrapper)[0]?.setValue("999");
    await findButton(wrapper, "校验并应用到研究").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("• 熔体温度超过上限");
    expect(wrapper.text()).not.toContain("已应用到当前研究");
    expect(project.project?.studies[0]?.process).toBeNull();
  });

  it("无项目或无活跃研究：提示先选研究（按钮禁用态外的守卫分支）", async () => {
    vi.mocked(checkProcess).mockResolvedValue([]);
    const project = useProjectStore();
    const panel = useProcessPanel();

    panel.applyProcess();
    await flushPromises();
    expect(panel.notice.value).toBe("请先选择一个研究。");

    project.project = projectFixture([studyFixture("study-1", null)]);
    project.activeStudyId = null;
    panel.applyProcess();
    await flushPromises();
    expect(panel.notice.value).toBe("请先选择一个研究。");
    expect(project.project?.studies[0]?.process).toBeNull();
  });

  it("应用按钮在无活跃研究或忙碌时禁用", async () => {
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });
    expect(findButton(wrapper, "校验并应用到研究").attributes("disabled")).toBeDefined();

    const project = useProjectStore();
    project.project = projectFixture([studyFixture("study-1", null)]);
    project.activeStudyId = "study-1";
    await nextTick();
    expect(findButton(wrapper, "校验并应用到研究").attributes("disabled")).toBeUndefined();

    const app = useAppStore();
    app.beginBusy("正在保存项目…");
    await nextTick();
    expect(findButton(wrapper, "校验并应用到研究").attributes("disabled")).toBeDefined();
  });

  it("预设：空名不保存；保存后进入下拉并可载回表单", async () => {
    // 混入一个非预设键：refreshPresetSelect 的前缀过滤需要走到 false 分支。
    localStorage.setItem("unrelated", "x");
    const wrapper = mount(ProcessPanel, { global: { plugins: [pinia] } });

    // 空名（含纯空白）不写入 localStorage。
    await findButton(wrapper, "保存预设").trigger("click");
    await fieldInputs(wrapper)[9]?.setValue("   ");
    await findButton(wrapper, "保存预设").trigger("click");
    expect(wrapper.findAll("option")).toHaveLength(1); // 仅「选择预设…」占位

    await fieldInputs(wrapper)[0]?.setValue("250");
    await fieldInputs(wrapper)[9]?.setValue("快速启动");
    await findButton(wrapper, "保存预设").trigger("click");

    const raw = localStorage.getItem("kairos-process-presets:快速启动");
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw ?? "{}")).toMatchObject({ meltTempC: 250 });
    // 下拉出现新选项并自动选中。
    const options = wrapper.findAll("option");
    expect(options.map((option) => option.text())).toContain("快速启动");
    expect((wrapper.find("select").element as HTMLSelectElement).value).toBe("快速启动");

    // 载入预设：表单恢复保存值。
    await fieldInputs(wrapper)[0]?.setValue("999");
    await findButton(wrapper, "载入预设").trigger("click");
    expect(fieldInputs(wrapper)[0]?.element.value).toBe("250");

    // 未选择预设时载入：表单保持不变。
    await wrapper.find("select").setValue("");
    await fieldInputs(wrapper)[0]?.setValue("777");
    await findButton(wrapper, "载入预设").trigger("click");
    expect(fieldInputs(wrapper)[0]?.element.value).toBe("777");
  });
});
