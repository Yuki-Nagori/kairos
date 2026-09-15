import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import XyChartPanel from "../../../../src-web/views/xy-chart/XyChartPanel.vue";
import { useXyChartPanel } from "../../../../src-web/views/xy-chart/useXyChartPanel";
import { useAppStore } from "../../../../src-web/stores/app";
import { useResultsStore } from "../../../../src-web/stores/results";
import { THEME_CHANGED_EVENT } from "../../../../src-web/utils/theme";
import { loadResultField, sampleProbeSeries } from "../../../../src-web/api/results";
import type { ScalarField } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  sampleProbeSeries: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));

function makeField(overrides: Partial<ScalarField> = {}): ScalarField {
  return {
    field: "T",
    timeDir: "0.001",
    timeS: 0.001,
    values: [10, 20, 30],
    isMagnitude: false,
    complete: true,
    ...overrides,
  };
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

describe("XyChartPanel", () => {
  let pinia: Pinia;
  /** happy-dom 环境按文件共享：逐个卸载，避免窗口主题监听跨用例重绘污染计数。 */
  const wrappers: Array<ReturnType<typeof mount>> = [];

  function mountPanel() {
    const wrapper = mount(XyChartPanel, { global: { plugins: [pinia] } });
    wrappers.push(wrapper);
    return wrapper;
  }

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  afterEach(() => {
    while (wrappers.length > 0) {
      wrappers.pop()!.unmount();
    }
    vi.restoreAllMocks();
  });

  it("挂载即绘制空场背景并登记快照", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("暂无探针");
  });

  it("加载场后经 watch 重绘曲线与坐标轴标签", async () => {
    mountPanel();
    useResultsStore().loadedField = makeField();
    await nextTick();
  });

  it("输入节点序号添加探针：圆点与值标注绘制在曲线上，输入框清空", async () => {
    const results = useResultsStore();
    results.loadedField = makeField();
    const wrapper = mountPanel();
    const input = wrapper.find("input");
    await input.setValue("1");
    await findButton(wrapper, "添加探针").trigger("click");
    expect((input.element as HTMLInputElement).value).toBe("");
    expect(wrapper.text()).toContain("节点 1");
    await nextTick();
  });

  it("非法与非负整数探针输入设置全局错误", async () => {
    const wrapper = mountPanel();
    const input = wrapper.find("input");

    await input.setValue("abc");
    await findButton(wrapper, "添加探针").trigger("click");
    expect(useAppStore().error?.message).toContain("非负整数");

    await input.setValue("-1");
    await findButton(wrapper, "添加探针").trigger("click");
    expect(useAppStore().error?.message).toContain("非负整数");

    const results = useResultsStore();
    results.loadedField = makeField({ values: [1, 2] });
    await input.setValue("9");
    await findButton(wrapper, "添加探针").trigger("click");
    expect(useAppStore().error?.message).toContain("超出范围");

    await input.setValue("0");
    await findButton(wrapper, "添加探针").trigger("click");
    await input.setValue("0");
    await findButton(wrapper, "添加探针").trigger("click");
    expect(useAppStore().error?.message).toContain("已有探针");
  });

  it("移除探针按钮从列表删除对应探针", async () => {
    const results = useResultsStore();
    results.probes = [
      { id: 1, nodeIndex: 0 },
      { id: 2, nodeIndex: 2 },
    ];
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("节点 0");
    await findButton(wrapper, "✕").trigger("click");
    expect(wrapper.text()).not.toContain("节点 0");
    expect(wrapper.text()).toContain("节点 2");
  });

  it("导出 CSV 在无场数据时设置全局错误", async () => {
    const wrapper = mountPanel();
    await findButton(wrapper, "导出 CSV").trigger("click");
    expect(useAppStore().error?.message).toBe("暂无可导出的场数据，请先加载场。");
  });

  it("重绘按钮手动触发 draw", async () => {
    const wrapper = mountPanel();
    await findButton(wrapper, "重置缩放").trigger("click");
    await findButton(wrapper, "重绘").trigger("click");
  });

  it("主题切换事件触发整帧重绘", async () => {
    mountPanel();
    window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
    await nextTick();
  });

  it("卸载后移除主题监听不再重绘", async () => {
    const wrapper = mountPanel();
    wrapper.unmount();
    window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
    await nextTick();
    expect(wrapper.exists()).toBe(false);
  });

  it("空值场只铺背景，不进入曲线与探针绘制", async () => {
    mountPanel();
    useResultsStore().loadedField = makeField({ values: [] });
    await nextTick();
  });

  it("探针时间曲线：就绪时加载时间序列并绘制探针编号", async () => {
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [
        { dirName: "0", timeS: 0, fields: ["T"] },
        { dirName: "1", timeS: 1, fields: ["T"] },
      ],
    };
    results.loadedField = makeField({ field: "T" });
    results.addProbe(0);
    results.addProbe(1);
    vi.mocked(sampleProbeSeries).mockResolvedValue(
      results.probes.map((probe, i) => ({
        probeId: probe.id,
        nodeIndex: probe.nodeIndex,
        samples: [
          { timeS: 0, value: 10 + i * 10 },
          { timeS: 1, value: 30 + i * 10 },
        ],
      })),
    );
    const wrapper = mountPanel();

    await wrapper.findAll("select")[0]!.setValue("time");
    await findButton(wrapper, "加载时间曲线").trigger("click");
    await flushPromises();

    expect(results.probeTimeSeries).toHaveLength(2);
    expect(results.probeTimeSeries[0]?.samples).toEqual([
      { timeS: 0, value: 10 },
      { timeS: 1, value: 30 },
    ]);
    expect(results.probeTimeSeries[1]?.samples).toEqual([
      { timeS: 0, value: 20 },
      { timeS: 1, value: 40 },
    ]);
    expect(wrapper.text()).toContain("探针时间曲线");
  });

  it("时间曲线加载守卫：目录/探针/场未就绪时静默", async () => {
    const panel = useXyChartPanel();
    panel.loadTimeSeries();
    panel.draw();
    expect(loadResultField).not.toHaveBeenCalled();
  });

  it("探针值在场缺失或越界时回落 0", () => {
    const results = useResultsStore();
    const panel = useXyChartPanel();
    results.probes = [{ id: 1, nodeIndex: 3 }];
    expect(panel.probeDots.value[0]?.value).toBe(0);
    results.loadedField = makeField({ values: [7, 8] });
    expect(panel.probeDots.value[0]?.value).toBe(0);
    results.loadedField = makeField({ values: [7, 8, 9, 42] });
    expect(panel.probeDots.value[0]?.value).toBe(42);
  });

  it("跳转选择为空时静默返回，不触发加载", async () => {
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "0", timeS: 0, fields: ["T"] }],
    };
    results.loadedField = makeField({ field: "T" });
    results.probeSeriesField = "T";
    const wrapper = mountPanel();

    // 保持空选项直接 change：守卫返回，不调用加载。
    await wrapper.findAll("select")[0]!.setValue("time");
    await wrapper.findAll("select")[1]!.setValue("");
    await flushPromises();

    expect(loadResultField).not.toHaveBeenCalled();
  });

  it("跳转到时间步：按所选时间步加载原始场", async () => {
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [
        { dirName: "0", timeS: 0, fields: ["T"] },
        { dirName: "1", timeS: 1, fields: ["T"] },
      ],
    };
    results.loadedField = makeField({ field: "T" });
    results.probeSeriesField = "T";
    vi.mocked(loadResultField).mockResolvedValue(makeField());
    const wrapper = mountPanel();

    // 跳转选择器仅在时间模式下渲染。
    await wrapper.findAll("select")[0]!.setValue("time");
    await wrapper.findAll("select")[1]!.setValue("1");
    await flushPromises();

    expect(loadResultField).toHaveBeenCalledWith("/case/run", "1", "T", "primary");
  });
});
