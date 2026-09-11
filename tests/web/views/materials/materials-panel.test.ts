import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type DOMWrapper } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import MaterialsPanel from "../../../../src-web/views/materials/MaterialsPanel.vue";
import { useMaterialsPanel } from "../../../../src-web/views/materials/useMaterialsPanel";
import LatexBlock from "../../../../src-web/components/latex-block/LatexBlock.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useMaterialsStore } from "../../../../src-web/stores/materials";
import { useProjectStore } from "../../../../src-web/stores/project";
import {
  deleteCustomMaterial,
  exportMaterialsToFile,
  importCustomMaterials,
  upsertCustomMaterial,
} from "../../../../src-web/api/materials";
import { pickExportJsonPath, pickOpenMaterialsPath } from "../../../../src-web/api/dialog";
import type { Material, Project, Study } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/materials", () => ({
  listBuiltinMaterials: vi.fn(),
  listCustomMaterials: vi.fn(),
  importCustomMaterials: vi.fn(),
  upsertCustomMaterial: vi.fn(),
  deleteCustomMaterial: vi.fn(),
  exportMaterialsToFile: vi.fn(),
}));
vi.mock("../../../../src-web/api/dialog", () => ({
  pickOpenProjectPath: vi.fn(),
  pickSaveProjectPath: vi.fn(),
  pickOpenMaterialsPath: vi.fn(),
  pickExportJsonPath: vi.fn(),
  pickStlPath: vi.fn(),
}));

function materialFixture(overrides: Partial<Material> = {}): Material {
  return {
    id: "mat-pp",
    name: "PP-示例",
    manufacturer: "示例数据",
    family: "PP",
    rheology: { n: 0.32, tauStar: 2e4, d1: 1.1e13, d2: 263, d3: 0, a1: 31, a2: 51.6 },
    pvt: {
      b1m: 1.28e-3,
      b1s: 1.22e-3,
      b2m: 7.5e-7,
      b2s: 3e-7,
      b3: 1.4e8,
      b4m: 3e-3,
      b4s: 1.5e-3,
      b5: 418,
    },
    specificHeat: [[300, 1900]],
    conductivity: [[300, 0.22]],
    mechanics: { elasticModulus: 1.5e9, poissonRatio: 0.35 },
    filler: null,
    dataNote: "示例数据，仅用于演示。",
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

function studyFixture(): Study {
  return {
    id: "study-1",
    name: "填充研究",
    createdMs: 1,
    runnerElements: [],
    coolingChannels: [],
    process: null,
    materialId: null,
  };
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

/** 材料清单按钮（truncate 类只出现在列表项上），内置在前、自定义在后。 */
function listButtons(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll("button.truncate");
}

function isSelected(button: DOMWrapper<Element> | undefined): boolean {
  return button?.classes().includes("bg-emerald-500/10") ?? false;
}

describe("MaterialsPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
  });

  it("空库：占位文案与按钮禁用态，空选中时的动作守卫不触发调用", () => {
    const panel = useMaterialsPanel();
    expect(panel.selected.value).toBeUndefined();
    // 空选中时各性质表早退为空（模板在无选中时不渲染这些表，直接驱动）。
    expect(panel.wlfRows.value).toEqual([]);
    expect(panel.pvtRows.value).toEqual([]);
    expect(panel.cpRows.value).toEqual([]);
    expect(panel.lambdaRows.value).toEqual([]);
    // 选中 id 为空时复制 / 登记 / 删除动作直接短路。
    panel.doCopy();
    panel.doUse();
    panel.doDelete();
    expect(upsertCustomMaterial).not.toHaveBeenCalled();
    expect(useAppStore().error).toBeNull();

    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("加载中…");
    expect(text).toContain("无自定义材料。");
    expect(text).toContain("选择左侧材料查看参数。");
    expect(findButton(wrapper, "导入 JSON").attributes("disabled")).toBeUndefined();
    expect(findButton(wrapper, "导出自定义").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "复制为自定义").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "删除").attributes("disabled")).toBeDefined();
  });

  it("初始回退展示首个内置材料：详情、公式常量与性质表", () => {
    const materials = useMaterialsStore();
    materials.materials = {
      builtin: [materialFixture()],
      custom: [materialFixture({ id: "custom-1", name: "PP-副本" })],
    };

    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });
    const text = wrapper.text();
    expect(text).toContain("PP-示例（示例数据）");
    // Cross-WLF / Tait 公式常量原样传给 KaTeX 渲染块。
    const latex = wrapper.findAllComponents(LatexBlock);
    expect(latex).toHaveLength(2);
    expect(latex[0]?.props("tex")).toContain("\\dfrac{\\eta_0}");
    expect(latex[1]?.props("tex")).toContain("T_t = b_5 + b_6 p");
    // 性质表行（键值 + 单位）。
    expect(text).toContain("τ*");
    expect(text).toContain("20000 Pa");
    expect(text).toContain("0.00128 m³/kg");
    expect(text).toContain("418 K");
    expect(text).toContain("300 K");
    expect(text).toContain("1900 J/(kg·K)");
    expect(text).toContain("0.22 W/(m·K)");
    expect(text).toContain("1500000000 Pa");
    expect(text).toContain("0.35");
    expect(text).toContain("示例数据，仅用于演示。");
    // watch 非 immediate：初始选中 id 仍为空，无高亮，复制 / 删除禁用。
    expect(listButtons(wrapper).every((button) => !isSelected(button))).toBe(true);
    expect(findButton(wrapper, "复制为自定义").attributes("disabled")).toBeDefined();
    expect(findButton(wrapper, "删除").attributes("disabled")).toBeDefined();
  });

  it("点击材料切换选中：高亮迁移且复制动作启用", async () => {
    const materials = useMaterialsStore();
    materials.materials = {
      builtin: [materialFixture()],
      custom: [materialFixture({ id: "custom-1", name: "PP-副本" })],
    };
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    await listButtons(wrapper)[0]?.trigger("click");
    await nextTick();
    expect(isSelected(listButtons(wrapper)[0])).toBe(true);
    expect(isSelected(listButtons(wrapper)[1])).toBe(false);
    expect(findButton(wrapper, "复制为自定义").attributes("disabled")).toBeUndefined();

    await listButtons(wrapper)[1]?.trigger("click");
    await nextTick();
    expect(isSelected(listButtons(wrapper)[0])).toBe(false);
    expect(isSelected(listButtons(wrapper)[1])).toBe(true);
    expect(wrapper.text()).toContain("PP-副本（示例数据）");
  });

  it("选中含填料牌号：渲染纤维 / 填料参数表", async () => {
    const fillerFixture = materialFixture({
      id: "mat-gf",
      name: "PA66-GF30",
      filler: {
        kind: "玻纤",
        weightFraction: 0.3,
        aspectRatio: 20,
        note: "短切玻纤",
      },
    });
    const materials = useMaterialsStore();
    materials.materials = { builtin: [fillerFixture], custom: [] };
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });
    await wrapper.vm.$nextTick();

    const fillerRows = useMaterialsPanel().fillerRows;
    expect(fillerRows.value).toEqual([
      ["类型", "玻纤"],
      ["质量分数", "30.0 %"],
      ["长径比", "20"],
      ["备注", "短切玻纤"],
    ]);
    expect(wrapper.text()).toContain("纤维 / 填料");
  });

  it("复制为自定义：新 id 与「副本」后缀经 upsert 落库", async () => {
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture()], custom: [] };
    vi.mocked(upsertCustomMaterial).mockResolvedValue([
      materialFixture({ id: "custom-2", name: "PP-示例-副本" }),
    ]);
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    await listButtons(wrapper)[0]?.trigger("click");
    await findButton(wrapper, "复制为自定义").trigger("click");
    await flushPromises();

    expect(upsertCustomMaterial).toHaveBeenCalledTimes(1);
    const copy = vi.mocked(upsertCustomMaterial).mock.calls[0]?.[0];
    expect(copy?.id).toMatch(/^custom-/);
    expect(copy?.name).toBe("PP-示例-副本");
    expect(copy?.dataNote.startsWith("自定义副本。")).toBe(true);
    expect(materials.materials.custom).toHaveLength(1);

    vi.mocked(upsertCustomMaterial).mockRejectedValue(new Error("写入失败"));
    await findButton(wrapper, "复制为自定义").trigger("click");
    await flushPromises();
    expect(useAppStore().error?.message).toBe("写入失败");
  });

  it("导入 JSON：取消无动作；成功更新清单且同 id 选中不被改写；失败进全局错误", async () => {
    const materials = useMaterialsStore();
    materials.materials = {
      builtin: [materialFixture()],
      custom: [materialFixture({ id: "custom-1", name: "PP-副本" })],
    };
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    // 取消：不触发 IPC。
    vi.mocked(pickOpenMaterialsPath).mockResolvedValue(null);
    await findButton(wrapper, "导入 JSON").trigger("click");
    await flushPromises();
    expect(importCustomMaterials).not.toHaveBeenCalled();

    // 选中自定义材料后再导入：同 id 的新对象替换清单，回退 watch 不改写选中 id。
    await listButtons(wrapper)[1]?.trigger("click");
    vi.mocked(pickOpenMaterialsPath).mockResolvedValue("/库/材料.json");
    vi.mocked(importCustomMaterials).mockResolvedValue([
      materialFixture({ id: "custom-1", name: "PP-副本-新" }),
    ]);
    await findButton(wrapper, "导入 JSON").trigger("click");
    await flushPromises();

    expect(importCustomMaterials).toHaveBeenCalledWith("/库/材料.json");
    expect(materials.materials.custom[0]?.name).toBe("PP-副本-新");
    // 详情跟随新对象显示，高亮仍停留在同 id 的自定义项上。
    expect(wrapper.text()).toContain("PP-副本-新（示例数据）");
    expect(isSelected(listButtons(wrapper)[1])).toBe(true);

    // 失败路径。
    vi.mocked(importCustomMaterials).mockRejectedValue(new Error("JSON 不合法"));
    await findButton(wrapper, "导入 JSON").trigger("click");
    await flushPromises();
    expect(useAppStore().error?.message).toBe("JSON 不合法");
  });

  it("导出自定义：有自定义材料时经对话框导出，失败进入全局错误", async () => {
    const materials = useMaterialsStore();
    const custom = [materialFixture({ id: "custom-1", name: "PP-副本" })];
    materials.materials = { builtin: [materialFixture()], custom };
    vi.mocked(pickExportJsonPath).mockResolvedValue("/导出/materials.json");
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    expect(findButton(wrapper, "导出自定义").attributes("disabled")).toBeUndefined();
    await findButton(wrapper, "导出自定义").trigger("click");
    await flushPromises();

    expect(pickExportJsonPath).toHaveBeenCalledWith("kairos-custom-materials");
    expect(exportMaterialsToFile).toHaveBeenCalledWith("/导出/materials.json", custom);

    vi.mocked(exportMaterialsToFile).mockRejectedValue(new Error("磁盘只读"));
    await findButton(wrapper, "导出自定义").trigger("click");
    await flushPromises();
    expect(useAppStore().error?.message).toBe("磁盘只读");
  });

  it("用于当前研究：无活跃研究时报错，有则登记到研究", async () => {
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture()], custom: [] };
    const project = useProjectStore();
    const app = useAppStore();
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    await listButtons(wrapper)[0]?.trigger("click");
    await findButton(wrapper, "用于当前研究").trigger("click");
    expect(app.error?.message).toBe("请先创建或选择一个研究。");

    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    app.error = null; // 上一步的报错不会被成功路径清除，这里显式复位。
    await findButton(wrapper, "用于当前研究").trigger("click");
    expect(app.error).toBeNull();
    expect(project.project?.studies[0]?.materialId).toBe("mat-pp");
  });

  it("删除自定义材料：失败保留选中便于重试，成功后回落首个内置材料", async () => {
    const materials = useMaterialsStore();
    materials.materials = {
      builtin: [materialFixture()],
      custom: [materialFixture({ id: "custom-1", name: "PP-副本" })],
    };
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    await listButtons(wrapper)[1]?.trigger("click");
    expect(findButton(wrapper, "删除").attributes("disabled")).toBeUndefined();

    vi.mocked(deleteCustomMaterial).mockRejectedValueOnce(new Error("删除失败"));
    await findButton(wrapper, "删除").trigger("click");
    await flushPromises();
    expect(useAppStore().error?.message).toBe("删除失败");
    expect(deleteCustomMaterial).toHaveBeenCalledWith("custom-1");
    // 删除失败材料仍在清单：选中保留，便于用户直接重试。
    expect(isSelected(listButtons(wrapper)[1])).toBe(true);
    expect(findButton(wrapper, "删除").attributes("disabled")).toBeUndefined();

    // 重试成功：custom 清空，选中回落首个内置材料。
    vi.mocked(deleteCustomMaterial).mockResolvedValue([]);
    await findButton(wrapper, "删除").trigger("click");
    await flushPromises();

    expect(isSelected(listButtons(wrapper)[0])).toBe(true);
    expect(wrapper.text()).toContain("PP-示例（示例数据）");
    expect(wrapper.text()).toContain("无自定义材料。");
    expect(findButton(wrapper, "删除").attributes("disabled")).toBeDefined();
  });

  it("忙碌中：IPC 类动作按钮全部禁用", async () => {
    const materials = useMaterialsStore();
    materials.materials = {
      builtin: [materialFixture()],
      custom: [materialFixture({ id: "custom-1" })],
    };
    const app = useAppStore();
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });

    await listButtons(wrapper)[1]?.trigger("click");
    expect(findButton(wrapper, "复制为自定义").attributes("disabled")).toBeUndefined();

    app.beginBusy("正在导入材料…");
    await nextTick();
    for (const label of ["导入 JSON", "导出自定义", "复制为自定义", "用于当前研究", "删除"]) {
      expect(findButton(wrapper, label).attributes("disabled")).toBeDefined();
    }
  });

  it("无力学参数的材料：力学表隐藏且 computed 早退为空", () => {
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture({ mechanics: null })], custom: [] };
    const wrapper = mount(MaterialsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).not.toContain("力学（预留）");
    // 模板不渲染力学表时该 computed 不被求值，直接驱动其早退分支。
    const panel = useMaterialsPanel();
    expect(panel.mechanicsRows.value).toEqual([]);
  });
});
