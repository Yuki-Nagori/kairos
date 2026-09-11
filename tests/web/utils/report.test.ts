import { describe, expect, it } from "vitest";
import { buildReportHtml, escapeHtml } from "../../../src-web/utils/report";

describe("escapeHtml", () => {
  it("escapes html special characters", () => {
    expect(escapeHtml('<script>&"')).toBe("&lt;script&gt;&amp;&quot;");
  });
});

describe("buildReportHtml", () => {
  const input = {
    projectName: "外壳注塑",
    studyName: "填充分析",
    materialName: "PP-示例-001",
    generatedAt: "2026-09-08 10:00",
    parameterRows: [
      ["熔体温度", "230 °C"],
      ["保压时间", "8 s"],
    ] as Array<[string, string]>,
    snapshots: [{ title: "压力云图", dataUrl: "data:image/png;base64,AAAA" }],
    fieldStats: "500 个值，min 300 / max 350",
  };

  it("builds a self-contained html document", () => {
    const html = buildReportHtml(input);
    expect(html.startsWith("<!doctype html>")).toBe(true);
    expect(html).toContain("外壳注塑");
    expect(html).toContain("230 °C");
    expect(html).toContain('src="data:image/png;base64,AAAA"');
    expect(html).toContain("压力云图");
    expect(html).toContain("min 300 / max 350");
  });

  it("escapes user-provided names", () => {
    const html = buildReportHtml({ ...input, projectName: "<img src=x onerror=alert(1)>" });
    expect(html).not.toContain("<img src=x onerror");
    expect(html).toContain("&lt;img");
  });

  it("omits stats section when null", () => {
    const html = buildReportHtml({ ...input, fieldStats: null });
    expect(html).not.toContain('class="stats"');
  });

  it("几何摘要 / 探针数值 / 时间序列节按数据渲染且转义", () => {
    const html = buildReportHtml({
      projectName: "项目 <A>",
      studyName: "研究",
      materialName: "PP",
      generatedAt: "now",
      parameterRows: [["材料", "PP"]],
      geometryRows: [
        ["三角形数", "12"],
        ["网格健康", "<b>健康</b>"],
      ],
      probeRows: [["#1 · 节点 3", "0.5000"]],
      timeSeriesTables: [
        {
          probeLabel: "#1 · 节点 <3>",
          samples: [
            ["0.000", "1.0000"],
            ["0.100", "2.0000"],
          ],
        },
      ],
      snapshots: [],
      fieldStats: null,
    });
    expect(html).toContain("<h2>几何摘要</h2>");
    expect(html).toContain("<h2>探针数值</h2>");
    expect(html).toContain("<h2>探针时间序列</h2>");
    expect(html).toContain("&lt;b&gt;健康&lt;/b&gt;");
    expect(html).toContain("#1 · 节点 &lt;3&gt;");
    expect(html).not.toContain("<b>健康</b>");
  });

  it("新节缺省时不渲染对应标题", () => {
    const html = buildReportHtml({
      projectName: "p",
      studyName: "s",
      materialName: "m",
      generatedAt: "now",
      parameterRows: [["材料", "PP"]],
      snapshots: [],
      fieldStats: null,
    });
    expect(html).not.toContain("几何摘要");
    expect(html).not.toContain("探针数值");
    expect(html).not.toContain("探针时间序列");
  });
});
