import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ResultsPanel from "../../../../src-web/views/results/ResultsPanel.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useResultsStore } from "../../../../src-web/stores/results";
import { deriveField, listResultTimes, loadResultField } from "../../../../src-web/api/results";
import type { ResultCatalog, ScalarField } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/results", () => ({
  listResultTimes: vi.fn(),
  loadResultField: vi.fn(),
  deriveField: vi.fn(),
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
    expect(wrapper.find("select").attributes("disabled")).toBeDefined();
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
    await vi.waitFor(() => expect(loadResultField).toHaveBeenCalledWith("/tmp/case", "0.001", "T"));
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

    expect(deriveField).toHaveBeenCalledWith("normalize");
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
    await wrapper.find("select").setValue("threshold");
    vi.mocked(deriveField).mockResolvedValue(
      makeField({ field: "T · 阈值掩码", values: [0, 1, 1], isMagnitude: false }),
    );
    await findButton(wrapper, "生成派生场").trigger("click");
    await flushPromises();
    expect(results.loadedField?.values).toEqual([0, 1, 1]);
    expect(wrapper.text()).toContain("T · 阈值掩码");
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
