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
});
