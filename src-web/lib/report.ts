/** 仿真报告 HTML 生成（T17）：模板 + 转义 + 快照/参数表。 */

interface ReportSnapshot {
  title: string;
  dataUrl: string;
}

interface ReportInput {
  projectName: string;
  studyName: string;
  materialName: string;
  generatedAt: string;
  /** 参数表：键值对逐行展示（材料与工艺参数）。 */
  parameterRows: Array<[string, string]>;
  snapshots: ReportSnapshot[];
  fieldStats: string | null;
}

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** 生成自包含 HTML 报告（可直接浏览器打开并打印为 PDF）。 */
export function buildReportHtml(input: ReportInput): string {
  const parameterRows = input.parameterRows
    .map(([key, value]) => `<tr><th>${escapeHtml(key)}</th><td>${escapeHtml(value)}</td></tr>`)
    .join("\n");
  const snapshots = input.snapshots
    .map(
      (snapshot) =>
        `<figure><img src="${snapshot.dataUrl}" alt="${escapeHtml(snapshot.title)}" /><figcaption>${escapeHtml(snapshot.title)}</figcaption></figure>`,
    )
    .join("\n");
  const fieldStats =
    input.fieldStats === null ? "" : `<p class="stats">${escapeHtml(input.fieldStats)}</p>`;
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<title>Kairos 仿真报告 · ${escapeHtml(input.projectName)} / ${escapeHtml(input.studyName)}</title>
<style>
  body { font-family: sans-serif; color: #18181b; margin: 40px auto; max-width: 800px; }
  h1 { border-bottom: 2px solid #10b981; padding-bottom: 8px; }
  table { border-collapse: collapse; width: 100%; margin: 16px 0; }
  th, td { border: 1px solid #d4d4d8; padding: 6px 10px; text-align: left; font-size: 14px; }
  th { background: #f4f4f5; width: 40%; }
  figure { margin: 16px 0; text-align: center; }
  figure img { max-width: 100%; border: 1px solid #d4d4d8; }
  figcaption { font-size: 12px; color: #71717a; margin-top: 4px; }
  .meta { color: #71717a; font-size: 12px; }
  .stats { background: #fafafa; padding: 8px; font-size: 13px; }
</style>
</head>
<body>
<h1>Kairos 仿真报告</h1>
<p class="meta">项目：${escapeHtml(input.projectName)} · 研究：${escapeHtml(input.studyName)} · 生成时间：${escapeHtml(input.generatedAt)}</p>
<h2>材料与工艺</h2>
<table>
${parameterRows}
</table>
<h2>结果</h2>
${fieldStats}
${snapshots}
<p class="meta">由 Kairos 生成 · 数值结果请结合材料数据来源与网格质量评估。</p>
</body>
</html>
`;
}
