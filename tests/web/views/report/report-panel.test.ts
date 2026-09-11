import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import ReportPanel from "../../../../src-web/views/report/ReportPanel.vue";
import { registerSnapshot } from "../../../../src-web/render/snapshot";
import { useMaterialsStore } from "../../../../src-web/stores/materials";
import { useProjectStore } from "../../../../src-web/stores/project";
import { useGeometryStore } from "../../../../src-web/stores/geometry";
import { useResultsStore } from "../../../../src-web/stores/results";
import type { Material, Project, ScalarField, Study } from "../../../../src-web/types";

function materialFixture(): Material {
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
    mechanics: null,
    filler: null,
    dataNote: "示例数据",
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

function fieldFixture(overrides: Partial<ScalarField> = {}): ScalarField {
  return {
    field: "T",
    timeDir: "0.5",
    timeS: 0.5,
    values: [300, 310],
    isMagnitude: false,
    complete: true,
    ...overrides,
  };
}

/** 伪 canvas：快照注册表只依赖 width/height/toDataURL。 */
function fakeCanvas(dataUrl: string): HTMLCanvasElement {
  return {
    width: 4,
    height: 4,
    toDataURL: () => dataUrl,
  } as unknown as HTMLCanvasElement;
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

async function clickGenerate(wrapper: ReturnType<typeof mount>): Promise<string> {
  const createSpy = vi.mocked(URL.createObjectURL, true);
  const revokeSpy = vi.mocked(URL.revokeObjectURL, true);
  createSpy.mockClear();
  revokeSpy.mockClear();
  await findButton(wrapper, "生成 HTML 报告").trigger("click");
  await flushPromises();
  const blob = createSpy.mock.calls[0]?.[0] as Blob;
  return blob.text();
}

describe("ReportPanel", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.clearAllMocks();
    // 保留 happy-dom 真实实现，仅记录调用以便读取写出的 Blob。
    vi.spyOn(URL, "createObjectURL");
    vi.spyOn(URL, "revokeObjectURL");
  });

  it("初始引导语；无项目或无活跃研究时提示先创建", async () => {
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("生成自包含 HTML（浏览器打开后 Ctrl+P 打印为 PDF）。");

    await findButton(wrapper, "生成 HTML 报告").trigger("click");
    expect(wrapper.text()).toContain("请先创建项目与研究。");

    // 项目存在但未选活跃研究 → 活跃研究为 null（守卫的第二个操作数）。
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    await findButton(wrapper, "生成 HTML 报告").trigger("click");
    expect(wrapper.text()).toContain("请先创建项目与研究。");
  });

  it("生成报告：无材料 / 工艺时参数行给占位，无快照与场统计", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);

    expect(wrapper.text()).toContain("报告已生成并下载");
    expect(URL.createObjectURL).toHaveBeenCalledTimes(1);
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(1);
    const blob = vi.mocked(URL.createObjectURL).mock.calls[0]?.[0] as Blob;
    expect(blob.type).toBe("text/html;charset=utf-8");
    expect(html).toContain("<title>Kairos 仿真报告 · 演示项目 / 填充研究</title>");
    expect(html).toContain("<th>材料</th><td>未登记</td>");
    expect(html).toContain("<th>熔体温度</th><td>未设置</td>");
    expect(html).not.toContain("<figure>");
    expect(html).not.toContain('class="stats"');
  });

  it("材料与工艺齐全 + 视口快照 + 场统计（完整）", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture({ materialId: "mat-pp", process: null })]);
    project.activeStudyId = "study-1";
    const materials = useMaterialsStore();
    materials.materials = { builtin: [materialFixture()], custom: [] };
    const results = useResultsStore();
    results.loadedField = fieldFixture();
    // 已登记材料但未设置工艺：工艺行占位、材料行齐全。
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    let html = await clickGenerate(wrapper);
    expect(html).toContain("<th>材料</th><td>示例数据 · PP-示例</td>");
    expect(html).toContain("<th>熔体温度</th><td>未设置</td>");
    expect(html).not.toContain("<figure>");

    // 登记工艺参数 + 视口快照 + 场统计。
    project.project = projectFixture([
      studyFixture({
        materialId: "mat-pp",
        process: {
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
        },
      }),
    ]);
    project.activeStudyId = "study-1";
    registerSnapshot("viewport", fakeCanvas("data:image/png;base64,QUJD"));

    html = await clickGenerate(wrapper);
    expect(html).toContain("<th>熔体温度</th><td>250 °C</td>");
    expect(html).toContain("<th>模具温度</th><td>60 °C</td>");
    expect(html).toContain("<th>注射时间</th><td>2 s</td>");
    expect(html).toContain("<th>保压时间</th><td>8 s</td>");
    expect(html).toContain("<th>冷却时间</th><td>20 s</td>");
    expect(html).toContain('src="data:image/png;base64,QUJD"');
    expect(html).toContain("视口</figcaption>");
    expect(html).toContain("T @ 0.5s：2 个值，min 300.000 / max 310.000</p>");
    expect(html).not.toContain("（不完整）");
  });

  it("XY 曲线快照与不完整场统计一并进入报告；材料未登记走兜底", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture({ materialId: "mat-missing" })]);
    project.activeStudyId = "study-1";
    const results = useResultsStore();
    results.loadedField = fieldFixture({ complete: false, field: "p" });
    registerSnapshot("xy-chart", fakeCanvas("data:image/png;base64,REVG"));
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);

    // materialId 已登记但库中无此材料：回退「未登记」。
    expect(html).toContain("<th>材料</th><td>未登记</td>");
    expect(html).toContain("XY 曲线</figcaption>");
    expect(html).toContain('src="data:image/png;base64,REVG"');
    expect(html).toContain("p @ 0.5s：2 个值，min 300.000 / max 310.000（不完整）</p>");
  });

  it("空值场不生成统计行", async () => {
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const results = useResultsStore();
    results.loadedField = fieldFixture({ values: [] });
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);
    expect(html).not.toContain('class="stats"');
  });

  it("几何摘要 / 探针数值 / 时间序列进入报告；无数据时不渲染对应节", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      {
        geometryId: "geo-1",
        fileName: "box.stl",
        triangleCount: 12,
        size: [10, 10, 2],
        surfaceArea: 280,
        signedVolume: 200,
        suggestedUnit: "mm",
        issues: { degenerate: 0, openEdges: 0, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    geometry.meshReports["geo-1"] = {
      engine: "voxel",
      nodeCount: 27,
      elementCount: 40,
      surfaceFaceCount: 60,
      totalVolume: 200,
      quality: { minEdgeRatio: 1, avgEdgeRatio: 1.2, maxEdgeRatio: 2, minVolume: 0.5 },
    };
    const results = useResultsStore();
    results.loadedField = fieldFixture({ values: [10, 20, 30] });
    // 第二个探针序号越界：值回退为「越界」占位。
    results.probes = [
      { id: 1, nodeIndex: 2 },
      { id: 2, nodeIndex: 99 },
    ];
    results.probeTimeSeries = [
      {
        probeId: 1,
        nodeIndex: 2,
        samples: [
          { timeS: 0, value: 30 },
          { timeS: 1, value: 22 },
        ],
      },
    ];
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);

    expect(html).toContain("<h2>几何摘要</h2>");
    expect(html).toContain("体积网格");
    expect(html).toContain("<h2>探针数值</h2>");
    expect(html).toContain("#1 · 节点 2");
    expect(html).toContain("30.0000");
    expect(html).toContain("<h2>探针时间序列</h2>");
    expect(html).toContain("22.0000");
  });

  it("不健康几何给出问题计数；无场时探针节省略", async () => {
    const geometry = useGeometryStore();
    geometry.geometries = [
      {
        geometryId: "geo-bad",
        fileName: "bad.stl",
        triangleCount: 12,
        size: [10, 10, 2],
        surfaceArea: 280,
        signedVolume: 200,
        suggestedUnit: "mm",
        issues: { degenerate: 2, openEdges: 4, nonManifoldEdges: 0, normalInconsistentEdges: 0 },
      },
    ];
    const results = useResultsStore();
    results.probes = [{ id: 1, nodeIndex: 0 }];
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);

    expect(html).toContain("退化 2 / 开放边 4 / 非流形 0");
    expect(html).not.toContain("<h2>探针数值</h2>");
  });

  it("无几何 / 无探针时不渲染对应节", async () => {
    const results = useResultsStore();
    results.loadedField = fieldFixture({ values: [10, 20, 30] });
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    const html = await clickGenerate(wrapper);

    expect(html).not.toContain("<h2>几何摘要</h2>");
    expect(html).not.toContain("<h2>探针数值</h2>");
    expect(html).not.toContain("<h2>探针时间序列</h2>");
  });

  it("模板化：自定义标题与关闭分区透传到报告", async () => {
    const results = useResultsStore();
    results.loadedField = fieldFixture({ values: [10, 20, 30] });
    const project = useProjectStore();
    project.project = projectFixture([studyFixture()]);
    project.activeStudyId = "study-1";
    const wrapper = mount(ReportPanel, { global: { plugins: [pinia] } });

    await wrapper.find('input[placeholder="报告标题（空 = 默认）"]').setValue("季度评审报告");
    // 关闭结果统计分区。
    const checkboxes = wrapper.findAll('input[type="checkbox"]');
    await checkboxes[2]!.setValue(false);

    const html = await clickGenerate(wrapper);

    expect(html).toContain("<h1>季度评审报告</h1>");
    expect(html).not.toContain("<h2>结果</h2>");
    expect(html).not.toContain("统计行");
  });
});
