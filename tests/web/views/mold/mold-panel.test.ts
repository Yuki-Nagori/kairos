import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import MoldPanel from "../../../../src-web/views/mold/MoldPanel.vue";
import { useMoldPanel } from "../../../../src-web/views/mold/useMoldPanel";
import { useAppStore } from "../../../../src-web/stores/app";
import { useProjectStore } from "../../../../src-web/stores/project";
import { checkMoldNetwork } from "../../../../src-web/api/mold";
import type { CoolingChannel, Project, RunnerElement, Study } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/mold", () => ({
  checkMoldNetwork: vi.fn(),
}));

function runnerElementFixture(overrides: Partial<RunnerElement> = {}): RunnerElement {
  return {
    id: "re-1",
    kind: "gate",
    diameterMm: 8,
    start: [1, 2, 3],
    end: [4, 5, 6],
    ...overrides,
  };
}

function channelFixture(overrides: Partial<CoolingChannel> = {}): CoolingChannel {
  return {
    id: "cc-1",
    diameterMm: 10,
    start: [0, 0, 0],
    end: [9, 9, 9],
    inletTempC: 25,
    ...overrides,
  };
}

function studyFixture(overrides: Partial<Study> = {}): Study {
  return {
    id: "study-1",
    name: "填充研究",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process: null,
    materialId: null,
    ...overrides,
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

describe("MoldPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("未选研究：表单整体禁用并给出引导文案", () => {
    const wrapper = mount(MoldPanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(wrapper.text().match(/请先在上方项目栏选择或创建一个研究。/g)).toHaveLength(2);
    expect(text).not.toContain("尚无单元。");
    // 15 个数值输入（流道 7 + 水路 8）与类型下拉全部禁用。
    for (const input of wrapper.findAll("input")) {
      expect(input.attributes("disabled")).toBeDefined();
    }
    expect(wrapper.find("select").attributes("disabled")).toBeDefined();
    for (const label of ["添加单元", "添加水路", "校验连通性"]) {
      expect(findButton(wrapper, label).attributes("disabled")).toBeDefined();
    }
  });

  it("添加流道 / 浇口单元：默认流道、可切浇口，坐标成组透传", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(MoldPanel, { global: { plugins: [pinia] } });

    // 默认：流道 / 直径 6 / 坐标全 0。
    await findButton(wrapper, "添加单元").trigger("click");
    let element = project.activeStudy?.runnerElements[0];
    expect(element).toMatchObject({
      kind: "runner",
      diameterMm: 6,
      start: [0, 0, 0],
      end: [0, 0, 0],
    });

    // 切浇口 + 填直径与起终点坐标。
    const inputs = wrapper.findAll("input");
    await wrapper.find("select").setValue("gate");
    await inputs[0]?.setValue("8");
    await inputs[1]?.setValue("1");
    await inputs[2]?.setValue("2");
    await inputs[3]?.setValue("3");
    await inputs[4]?.setValue("4");
    await inputs[5]?.setValue("5");
    await inputs[6]?.setValue("6");
    await findButton(wrapper, "添加单元").trigger("click");

    const elements = project.activeStudy?.runnerElements ?? [];
    expect(elements).toHaveLength(2);
    element = elements[1];
    expect(element).toMatchObject({
      kind: "gate",
      diameterMm: 8,
      start: [1, 2, 3],
      end: [4, 5, 6],
    });
    expect(wrapper.text()).toContain(`浇口 ${element?.id} · Ø8 mm`);
    expect(wrapper.text()).toContain("流道");
  });

  it("添加冷却水路并删除：标签带直径与入口温度", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(MoldPanel, { global: { plugins: [pinia] } });

    const inputs = wrapper.findAll("input");
    await inputs[7]?.setValue("8"); // 水路直径
    await inputs[8]?.setValue("1"); // 起点 x
    await inputs[9]?.setValue("2");
    await inputs[10]?.setValue("3");
    await inputs[11]?.setValue("4"); // 终点 x
    await inputs[12]?.setValue("5");
    await inputs[13]?.setValue("6");
    await inputs[14]?.setValue("30"); // 入口温度
    await findButton(wrapper, "添加水路").trigger("click");

    const channels = project.activeStudy?.coolingChannels ?? [];
    expect(channels[0]).toMatchObject({
      diameterMm: 8,
      start: [1, 2, 3],
      end: [4, 5, 6],
      inletTempC: 30,
    });
    expect(wrapper.text()).toContain(`水路 ${channels[0]?.id} · Ø8 mm · 30°C`);

    // 行内 ✕ 删除。
    const removeButtons = wrapper.findAll("button").filter((b) => b.text() === "✕");
    await removeButtons[0]?.trigger("click");
    await nextTick();
    expect(project.activeStudy?.coolingChannels).toHaveLength(0);
    expect(wrapper.text()).toContain("尚无水路。");
  });

  it("校验连通性：问题清单着红色展示，通过后复位；失败进全局错误", async () => {
    const project = useProjectStore();
    project.project = projectFixture([
      studyFixture({
        runnerElements: [runnerElementFixture()],
        coolingChannels: [channelFixture()],
      }),
    ]);
    project.activeStudyId = "study-1";
    const wrapper = mount(MoldPanel, { global: { plugins: [pinia] } });

    // 初始无问题：不渲染红色容器；标签渲染（浇口 + 流道两种文案）。
    expect(wrapper.text()).toContain("浇口 re-1 · Ø8 mm");
    expect(wrapper.find(".border-red-900").exists()).toBe(false);

    vi.mocked(checkMoldNetwork).mockResolvedValue(["浇口未连接到流道", "水路未连通"]);
    await findButton(wrapper, "校验连通性").trigger("click");
    await flushPromises();

    expect(checkMoldNetwork).toHaveBeenCalledWith(
      project.activeStudy?.runnerElements,
      project.activeStudy?.coolingChannels,
    );
    expect(project.moldIssues).toHaveLength(2);
    expect(wrapper.find(".border-red-900").exists()).toBe(true);
    expect(wrapper.text()).toContain("• 浇口未连接到流道");

    vi.mocked(checkMoldNetwork).mockResolvedValue([]);
    await findButton(wrapper, "校验连通性").trigger("click");
    await flushPromises();
    expect(project.moldIssues).toHaveLength(0);
    expect(wrapper.find(".border-red-900").exists()).toBe(false);

    vi.mocked(checkMoldNetwork).mockRejectedValue(new Error("校验服务不可用"));
    await findButton(wrapper, "校验连通性").trigger("click");
    await flushPromises();
    expect(useAppStore().error?.message).toBe("校验服务不可用");
  });

  it("忙碌中：表单整体禁用", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const app = useAppStore();
    const wrapper = mount(MoldPanel, { global: { plugins: [pinia] } });

    app.beginBusy("正在保存项目…");
    await nextTick();
    for (const input of wrapper.findAll("input")) {
      expect(input.attributes("disabled")).toBeDefined();
    }
    for (const label of ["添加单元", "添加水路", "校验连通性"]) {
      expect(findButton(wrapper, label).attributes("disabled")).toBeDefined();
    }
  });

  it("坐标键缺失时按 0 兜底（xyz 的空值防御分支）", () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const panel = useMoldPanel();

    // 正常交互下坐标对象恒有 x/y/z 三键；删键是唯一能执行 `?? 0`
    // 兜底臂的方式（该分支为防御性代码，专门覆盖）。
    const start = panel.runnerStart as Record<string, string | undefined>;
    const end = panel.runnerEnd as Record<string, string | undefined>;
    for (const axis of ["x", "y", "z"]) {
      delete start[axis];
      delete end[axis];
    }
    panel.addRunner();

    const channelStart = panel.channelStart as Record<string, string | undefined>;
    const channelEnd = panel.channelEnd as Record<string, string | undefined>;
    for (const axis of ["x", "y", "z"]) {
      delete channelStart[axis];
      delete channelEnd[axis];
    }
    panel.addChannel();

    expect(project.activeStudy?.runnerElements[0]?.start).toEqual([0, 0, 0]);
    expect(project.activeStudy?.runnerElements[0]?.end).toEqual([0, 0, 0]);
    expect(project.activeStudy?.coolingChannels[0]?.start).toEqual([0, 0, 0]);
    expect(project.activeStudy?.coolingChannels[0]?.end).toEqual([0, 0, 0]);
  });
});
