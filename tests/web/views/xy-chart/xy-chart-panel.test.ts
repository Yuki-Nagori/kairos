import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { defineComponent, nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import XyChartPanel from "../../../../src-web/views/xy-chart/XyChartPanel.vue";
import { useXyChartPanel } from "../../../../src-web/views/xy-chart/useXyChartPanel";
import { useAppStore } from "../../../../src-web/stores/app";
import { useResultsStore } from "../../../../src-web/stores/results";
import { THEME_CHANGED_EVENT } from "../../../../src-web/composables/useTheme";
import { loadResultField } from "../../../../src-web/api/results";
import type { ScalarField } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));

/** 录制调用的假 2D 上下文：属性可写，方法名与实参记入 calls。 */
function fakeCtx(): CanvasRenderingContext2D & { calls: Array<{ name: string; args: unknown[] }> } {
  const calls: Array<{ name: string; args: unknown[] }> = [];
  return new Proxy(
    {},
    {
      get(_target, prop) {
        if (prop === "calls") {
          return calls;
        }
        return (...args: unknown[]) => {
          calls.push({ name: String(prop), args });
        };
      },
      set() {
        return true;
      },
    },
  ) as CanvasRenderingContext2D & { calls: Array<{ name: string; args: unknown[] }> };
}

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
  let ctx: CanvasRenderingContext2D & { calls: Array<{ name: string; args: unknown[] }> };
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
    ctx = fakeCtx();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(ctx);
  });

  afterEach(() => {
    while (wrappers.length > 0) {
      wrappers.pop()!.unmount();
    }
    vi.restoreAllMocks();
  });

  it("挂载即绘制空场背景并登记快照", () => {
    const wrapper = mountPanel();
    expect(wrapper.find("canvas").exists()).toBe(true);
    expect(ctx.calls.map((call) => call.name)).toContain("fillRect");
    expect(ctx.calls.map((call) => call.name)).not.toContain("arc");
    expect(wrapper.text()).toContain("暂无探针");
  });

  it("getContext 返回 null 时静默早退", () => {
    vi.mocked(HTMLCanvasElement.prototype.getContext).mockReturnValueOnce(null);
    const wrapper = mountPanel();
    expect(wrapper.find("canvas").exists()).toBe(true);
    expect(ctx.calls).toHaveLength(0);
  });

  it("加载场后经 watch 重绘曲线与坐标轴标签", async () => {
    mountPanel();
    const callsBefore = ctx.calls.length;
    useResultsStore().loadedField = makeField();
    await nextTick();
    const names = ctx.calls.map((call) => call.name);
    expect(names).toContain("moveTo");
    expect(names).toContain("lineTo");
    expect(names).toContain("stroke");
    expect(ctx.calls.length).toBeGreaterThan(callsBefore);
    const fillTextArgs = ctx.calls
      .filter((call) => call.name === "fillText")
      .flatMap((call) => call.args);
    expect(fillTextArgs).toContain("节点序号");
    expect(fillTextArgs).toContain("T");
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
    expect(ctx.calls.map((call) => call.name)).toContain("arc");
    const labels = ctx.calls.filter((call) => call.name === "fillText").map((call) => call.args[0]);
    expect(labels).toContain("#1: 20.00");
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
    const callsBefore = ctx.calls.length;
    await findButton(wrapper, "重绘").trigger("click");
    expect(ctx.calls.length).toBeGreaterThan(callsBefore);
  });

  it("主题切换事件触发整帧重绘", async () => {
    mountPanel();
    const callsBefore = ctx.calls.length;
    window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
    await nextTick();
    expect(ctx.calls.length).toBeGreaterThan(callsBefore);
  });

  it("卸载后移除主题监听不再重绘", async () => {
    const wrapper = mountPanel();
    wrapper.unmount();
    const callsAfterUnmount = ctx.calls.length;
    window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
    await nextTick();
    expect(ctx.calls.length).toBe(callsAfterUnmount);
  });

  it("空值场只铺背景，不进入曲线与探针绘制", async () => {
    mountPanel();
    const before = ctx.calls.length;
    useResultsStore().loadedField = makeField({ values: [] });
    await nextTick();
    const fresh = ctx.calls.slice(before);
    expect(fresh.map((call) => call.name)).toContain("fillRect");
    expect(fresh.map((call) => call.name)).not.toContain("lineTo");
    expect(fresh.map((call) => call.name)).not.toContain("arc");
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
    vi.mocked(loadResultField).mockImplementation((_caseDir: string, timeDir: string) => {
      const values = timeDir === "0" ? [10, 20] : [30, 40];
      return Promise.resolve(makeField({ timeDir, values }));
    });
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
    // 时间模式坐标轴：x 轴为时间步序，值域取自探针采样 min/max。
    const texts = ctx.calls.filter((call) => call.name === "fillText").map((call) => call.args[0]);
    expect(texts).toContain("时间步（序）");
    expect(texts).toContain("10.000");
    expect(texts).toContain("40.000");
  });

  it("时间曲线加载守卫：目录/探针/场未就绪时静默", async () => {
    const panel = useXyChartPanel();
    panel.loadTimeSeries();
    expect(loadResultField).not.toHaveBeenCalled();
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

describe("useXyChartPanel 防御分支（无 canvas 环境）", () => {
  let pinia: Pinia;
  let ctx: CanvasRenderingContext2D & { calls: Array<{ name: string; args: unknown[] }> };
  const wrappers: Array<ReturnType<typeof mount>> = [];

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    ctx = fakeCtx();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(ctx);
  });

  afterEach(() => {
    while (wrappers.length > 0) {
      wrappers.pop()!.unmount();
    }
    vi.restoreAllMocks();
  });

  /** 无 canvas 的挂载环境：驱动 composable 里 canvas === null 的防御分支。 */
  function mountCanvaslessHarness() {
    let panel!: ReturnType<typeof useXyChartPanel>;
    const wrapper = mount(
      defineComponent({
        setup() {
          panel = useXyChartPanel();
          return () => null;
        },
      }),
      { global: { plugins: [pinia] } },
    );
    wrappers.push(wrapper);
    return { wrapper, panel };
  }

  it("canvas 缺失时 draw 与主题重绘都安全早退", async () => {
    mountCanvaslessHarness();
    expect(ctx.calls).toHaveLength(0);
    window.dispatchEvent(new CustomEvent(THEME_CHANGED_EVENT));
    await nextTick();
    expect(ctx.calls).toHaveLength(0);
  });

  it("probeDots：无场回落 0，越界节点回落 0，在场内取实际值", () => {
    const results = useResultsStore();
    const { panel } = mountCanvaslessHarness();
    results.probes = [{ id: 1, nodeIndex: 3 }];
    expect(panel.probeDots.value).toEqual([{ probe: { id: 1, nodeIndex: 3 }, value: 0 }]);

    results.loadedField = makeField({ values: [7, 8] });
    expect(panel.probeDots.value).toEqual([{ probe: { id: 1, nodeIndex: 3 }, value: 0 }]);

    results.loadedField = makeField({ values: [7, 8, 9, 42] });
    expect(panel.probeDots.value).toEqual([{ probe: { id: 1, nodeIndex: 3 }, value: 42 }]);
  });
});
