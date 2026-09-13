import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import type { Mock } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ResultsPanel from "../../../../src-web/views/results/ResultsPanel.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useResultsStore } from "../../../../src-web/stores/results";
import {
  deriveDifference,
  deriveField,
  listResultTimes,
  loadResultField,
} from "../../../../src-web/api/results";
import type { ResultCatalog, ScalarField } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  loadVectorField: vi.fn(),
  loadTensorField: vi.fn(),
  deriveField: vi.fn(),
  deriveDifference: vi.fn(),
}));

const catalog: ResultCatalog = {
  caseDir: "/tmp/case",
  times: [
    { dirName: "0.001", timeS: 0.001, fields: ["T", "U"] },
    { dirName: "0.002", timeS: 0.002, fields: ["T"] },
  ],
};

function makeField(overrides: Partial<ScalarField> = {}): ScalarField {
  return {
    field: "T",
    timeDir: "0.001",
    timeS: 0.001,
    values: [280, 300, 320],
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

describe("ResultsPanel 扫描与目录", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("未扫描时显示占位，派生场控件禁用", () => {
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("尚未扫描结果目录。");
    expect(wrapper.findAll("select")[1]?.attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "生成派生场").attributes("disabled")).toBeDefined();
  });

  it("扫描：busy 与空路径守卫，成功后渲染时间步表格", async () => {
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    const app = useAppStore();
    // busy 守卫：不触发扫描。
    app.busy = "正在求解…";
    await findButton(wrapper, "扫描结果").trigger("click");
    expect(listResultTimes).not.toHaveBeenCalled();
    // 空路径守卫。
    app.busy = null;
    await findButton(wrapper, "扫描结果").trigger("click");
    expect(listResultTimes).not.toHaveBeenCalled();

    vi.mocked(listResultTimes).mockResolvedValue(catalog);
    await wrapper.find("input").setValue("  /tmp/case  ");
    await findButton(wrapper, "扫描结果").trigger("click");
    await vi.waitFor(() => expect(wrapper.text()).toContain("0.001"));
    expect(listResultTimes).toHaveBeenCalledWith("/tmp/case");
    expect(wrapper.text()).toContain("0.002");
    expect(wrapper.text()).toContain("T, U");
  });

  it("点击时间步的加载 T 场按钮加载场数据", async () => {
    vi.mocked(listResultTimes).mockResolvedValue(catalog);
    vi.mocked(loadResultField).mockResolvedValue(makeField());
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.find("input").setValue("/tmp/case");
    await findButton(wrapper, "扫描结果").trigger("click");
    await vi.waitFor(() => expect(wrapper.text()).toContain("加载 T 场"));
    await findButton(wrapper, "加载 T 场").trigger("click");
    await vi.waitFor(() =>
      expect(loadResultField).toHaveBeenCalledWith("/tmp/case", "0.001", "T", "primary"),
    );
    expect(wrapper.text()).toContain("已加载 T @ 0.001：3 个值，min 280.000 / max 320.000");
  });

  it("扫描失败设置全局错误", async () => {
    vi.mocked(listResultTimes).mockRejectedValue(new Error("目录不存在"));
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.find("input").setValue("/tmp/case");
    await findButton(wrapper, "扫描结果").trigger("click");
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("目录不存在"));
  });

  it("目录中没有时间步时显示空目录占位", async () => {
    vi.mocked(listResultTimes).mockResolvedValue({ caseDir: "/tmp/case", times: [] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.find("input").setValue("/tmp/case");
    await findButton(wrapper, "扫描结果").trigger("click");
    await vi.waitFor(() => expect(wrapper.text()).toContain("结果目录中未发现时间步。"));
  });
});

describe("ResultsPanel 场统计", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("完整标量场：统计行不带模量后缀，完整态绿色", async () => {
    const results = useResultsStore();
    results.loadedField = makeField();
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("已加载 T @ 0.001：3 个值，min 280.000 / max 320.000");
    expect(wrapper.text()).toContain("结果完整");
    expect(wrapper.text()).not.toContain("（模量）");
    const completeness = wrapper.findAll("p").find((p) => p.text() === "结果完整");
    expect(completeness?.classes()).toContain("text-emerald-400");
  });

  it("模量场带后缀，不完整场显示琥珀警示", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({
      field: "U",
      isMagnitude: true,
      complete: false,
    });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("已加载 U @ 0.001（模量）：3 个值");
    expect(wrapper.text()).toContain("结果不完整（求解中途取消）");
    const warn = wrapper.findAll("p").find((p) => p.text() === "结果不完整（求解中途取消）");
    expect(warn?.classes()).toContain("text-amber-400");
  });

  it("空场以 NaN 占位仍按数值渲染", () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("0 个值，min NaN / max NaN");
  });
});

describe("ResultsPanel 派生场", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("归一化派生场：派生请求携带 kind，返回场写回加载态", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [1, 2, 3] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    const derived = makeField({
      field: "T · 归一化",
      values: [0, 0.5, 1],
      isMagnitude: false,
    });
    vi.mocked(deriveField).mockResolvedValue(derived);

    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();

    expect(deriveField).toHaveBeenCalledWith({ kind: "normalize" });
    expect(wrapper.text()).toContain("已加载 T · 归一化 @ 0.001：3 个值，min 0.000 / max 1.000");
    expect(results.loadedField?.values).toEqual([0, 0.5, 1]);
    expect(results.loadedField?.isMagnitude).toBe(false);
  });

  it("平坦场归一化得到全 0", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [5, 5, 5] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    vi.mocked(deriveField).mockResolvedValue(
      makeField({ field: "T · 归一化", values: [0, 0, 0], isMagnitude: false }),
    );
    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();
    expect(results.loadedField?.values).toEqual([0, 0, 0]);
    expect(wrapper.text()).toContain("T · 归一化");
  });

  it("阈值掩码：中点以上 1、以下 0", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [1, 3, 2] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("select")[1]!.setValue("threshold");
    vi.mocked(deriveField).mockResolvedValue(
      makeField({ field: "T · 阈值掩码", values: [0, 1, 1], isMagnitude: false }),
    );
    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();
    expect(results.loadedField?.values).toEqual([0, 1, 1]);
    expect(wrapper.text()).toContain("T · 阈值掩码");
  });

  it("线性映射：请求携带 scale 与 offset 参数", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [1, 2] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("select")[1]!.setValue("linear");
    await wrapper.find('input[placeholder="scale"]').setValue("2");
    await wrapper.find('input[placeholder="offset"]').setValue("-1");
    vi.mocked(deriveField).mockResolvedValue(
      makeField({ field: "T · 线性映射 ×2 -1", values: [1, 3], isMagnitude: false }),
    );

    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();

    expect(deriveField).toHaveBeenCalledWith({ kind: "linear", scale: 2, offset: -1 });
    expect(wrapper.text()).toContain("T · 线性映射 ×2 -1");
  });

  it("线性参数留空时按 scale=1、offset=0 回退", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [1, 2] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("select")[1]!.setValue("linear");
    // 显式清空输入框：空串经 Number() 得 NaN，走 || 回退分支。
    await wrapper.find('input[placeholder="scale"]').setValue("");
    await wrapper.find('input[placeholder="offset"]').setValue("");
    vi.mocked(deriveField).mockResolvedValue(makeField());

    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();

    expect(deriveField).toHaveBeenCalledWith({ kind: "linear", scale: 1, offset: 0 });
  });

  it("两场差值：对比场加载后可用，结果写回主场展示", async () => {
    vi.mocked(loadResultField).mockResolvedValue(makeField());
    const results = useResultsStore();
    const catalog = {
      caseDir: "/tmp/case",
      times: [{ dirName: "0.001", timeS: 0.001, fields: ["T"] }],
    };
    results.resultCatalog = catalog;
    results.loadedField = makeField();
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });

    // 未加载对比场时按钮禁用。
    expect(findButton(wrapper, "两场差值").attributes("disabled")).toBeDefined();

    await wrapper.find("select").setValue("compare");
    await wrapper
      .findAll("button")
      .find((b) => b.text() === "加载 T 场")!
      .trigger("click");
    await flushPromises();
    expect(loadResultField).toHaveBeenCalledWith("/tmp/case", "0.001", "T", "compare");
    expect(results.compareField).not.toBeNull();
    expect(wrapper.text()).toContain("对比场：T @ 0.001");

    vi.mocked(deriveDifference).mockResolvedValue(
      makeField({ field: "T - T", values: [0, 0, 0], isMagnitude: false }),
    );
    await findButton(wrapper, "两场差值").trigger("click");
    await flushPromises();
    expect(deriveDifference).toHaveBeenCalled();
    expect(results.loadedField?.field).toBe("T - T");
  });

  it("空场派生早退不改原名", async () => {
    const results = useResultsStore();
    results.loadedField = makeField({ values: [] });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await findButton(wrapper, "生成派生场").trigger("click");
    expect(wrapper.text()).toContain("已加载 T @ 0.001");
    expect(results.loadedField?.field).toBe("T");
  });
});

describe("ResultsPanel：矢量场三分量", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("无目录时禁用；加载后展示单元数、首单元分量与模量范围", async () => {
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    const button = wrapper.findAll("button").find((node) => node.text() === "加载矢量场")!;
    expect(button.attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain("未加载矢量场");

    // 扫描出目录后可用：时间步跟随当前已加载场（无则取最后一个）
    const { listResultTimes, loadVectorField } =
      (await import("../../../../src-web/api/results")) as unknown as {
        listResultTimes: Mock;
        loadVectorField: Mock;
      };
    listResultTimes.mockResolvedValue({
      caseDir: "/case/run",
      times: [
        { dirName: "1", timeS: 1, fields: ["D"] },
        { dirName: "2", timeS: 2, fields: ["D"] },
      ],
    });
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: [] }],
    };
    results.loadedField = {
      field: "T",
      timeDir: "1",
      timeS: 1,
      values: [1],
      isMagnitude: false,
      complete: true,
    };
    loadVectorField.mockResolvedValue({
      field: "D",
      timeDir: "1",
      timeS: 1,
      components: [
        [0.001, -0.002, 0],
        [0.0002, 0.0001, -0.0003],
      ],
      complete: true,
    });
    await nextTick();

    const enabled = wrapper.findAll("button").find((node) => node.text() === "加载矢量场")!;
    expect(enabled.attributes("disabled")).toBeUndefined();
    await enabled.trigger("click");
    await flushPromises();

    // 时间步取当前已加载场的 1（而不是最后一个 2）
    expect(loadVectorField).toHaveBeenCalledWith("/case/run", "1", "D");
    expect(wrapper.text()).toContain("矢量 D @ 1：2 个单元");
    expect(wrapper.text()).toContain("首单元 (1.00e-3, -2.00e-3, 0.00e+0)");
    expect(wrapper.text()).toContain("|v|");
  });

  it("无目录 / 空时间步时不请求；场名为空回退 D（防御分支）", async () => {
    const { loadVectorField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadVectorField: Mock;
    };
    const results = useResultsStore();
    const { useResultsPanel } = await import("../../../../src-web/views/results/useResultsPanel");
    const panel = useResultsPanel();

    // 目录未扫描
    panel.loadVector();
    expect(loadVectorField).not.toHaveBeenCalled();

    // 扫描了但没有时间步
    results.resultCatalog = { caseDir: "/case/run", times: [] };
    panel.loadVector();
    expect(loadVectorField).not.toHaveBeenCalled();

    // 场名清空 → 用默认 D
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: [] }],
    };
    panel.vectorFieldName.value = "   ";
    loadVectorField.mockResolvedValue({
      field: "D",
      timeDir: "1",
      timeS: 1,
      components: [],
      complete: true,
    });
    panel.loadVector();
    await flushPromises();
    expect(loadVectorField).toHaveBeenCalledWith("/case/run", "1", "D");
    // 空分量：统计行为 null（不渲染空行）
    expect(panel.vectorStats.value).toBeNull();
  });

  it("未加载场时取最后一个时间步；不完整矢量给出告警行", async () => {
    const { loadVectorField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadVectorField: Mock;
    };
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [
        { dirName: "1", timeS: 1, fields: ["D"] },
        { dirName: "2", timeS: 2, fields: ["D"] },
      ],
    };
    loadVectorField.mockResolvedValue({
      field: "U",
      timeDir: "2",
      timeS: 2,
      components: [[1, 0, 0]],
      complete: false,
    });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await nextTick();

    await wrapper.find('input[placeholder="D"]').setValue("U");
    await wrapper
      .findAll("button")
      .find((node) => node.text() === "加载矢量场")!
      .trigger("click");
    await flushPromises();

    expect(loadVectorField).toHaveBeenCalledWith("/case/run", "2", "U");
    expect(wrapper.text()).toContain("矢量 U @ 2：1 个单元");
    const incomplete = wrapper.findAll("p").find((node) => node.text().includes("（不完整）"));
    expect(incomplete?.classes()).toContain("text-amber-400");
  });
});

describe("ResultsPanel：对称张量场", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("无目录时禁用；加载后展示单元数、模量范围与首单元主轴", async () => {
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("未加载张量场");

    const { loadTensorField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadTensorField: Mock;
    };
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "2", timeS: 2, fields: [] }],
    };
    loadTensorField.mockResolvedValue({
      field: "sigma",
      timeDir: "2",
      timeS: 2,
      components: [[100, 0, 0, 0, 0, 0]],
      magnitudes: [100, 50],
      principalAxes: [[0.577, 0.577, 0.577]],
      complete: true,
    });
    await nextTick();

    await findButton(wrapper, "加载张量场").trigger("click");
    await flushPromises();
    expect(loadTensorField).toHaveBeenCalledWith("/case/run", "2", "sigma");
    expect(wrapper.text()).toContain("张量 sigma @ 2：2 个单元");
    expect(wrapper.text()).toContain("首单元主轴 (0.577, 0.577, 0.577)");
  });

  it("无目录 / 空时间步时不请求张量场；名字清空回退 sigma（防御分支）", async () => {
    const { loadTensorField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadTensorField: Mock;
    };
    const results = useResultsStore();
    const { useResultsPanel } = await import("../../../../src-web/views/results/useResultsPanel");
    const panel = useResultsPanel();

    panel.loadTensor();
    expect(loadTensorField).not.toHaveBeenCalled();

    results.resultCatalog = { caseDir: "/case/run", times: [] };
    panel.loadTensor();
    expect(loadTensorField).not.toHaveBeenCalled();

    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: [] }],
    };
    panel.tensorFieldName.value = "  ";
    loadTensorField.mockResolvedValue({
      field: "sigma",
      timeDir: "1",
      timeS: 1,
      components: [],
      magnitudes: [],
      principalAxes: [],
      complete: true,
    });
    panel.loadTensor();
    await flushPromises();
    expect(loadTensorField).toHaveBeenCalledWith("/case/run", "1", "sigma");
    expect(panel.tensorStats.value).toBeNull();
  });

  it("不完整张量给出告警行", async () => {
    const { loadTensorField } = (await import("../../../../src-web/api/results")) as unknown as {
      loadTensorField: Mock;
    };
    const results = useResultsStore();
    results.resultCatalog = {
      caseDir: "/case/run",
      times: [{ dirName: "1", timeS: 1, fields: [] }],
    };
    loadTensorField.mockResolvedValue({
      field: "sigmaEq",
      timeDir: "1",
      timeS: 1,
      components: [],
      magnitudes: [1],
      principalAxes: [[0, 0, 1]],
      complete: false,
    });
    const wrapper = mount(ResultsPanel, { global: { plugins: [pinia] } });
    await nextTick();
    await wrapper.find('input[placeholder="sigma"]').setValue("sigmaEq");
    await findButton(wrapper, "加载张量场").trigger("click");
    await flushPromises();

    const line = wrapper.findAll("p").find((node) => node.text().includes("（不完整）"));
    expect(line?.classes()).toContain("text-amber-400");
  });
});
