/** 仿真报告 HTML 生成：模板 + 转义 + 快照/参数表。 */

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
  /** 几何摘要（三角形数 / 尺寸 / 网格健康）；缺省或空数组时不渲染该节。 */
  geometryRows?: Array<[string, string]>;
  /** 探针数值（节点序号 → 当前场值）；缺省或空数组时不渲染该节。 */
  probeRows?: Array<[string, string]>;
  /** 探针时间序列（每探针一张 采样表）；缺省或空数组时不渲染该节。 */
  timeSeriesTables?: Array<{ probeLabel: string; samples: Array<[string, string]> }>;
}

/** 转义 HTML 特殊字符：报告内容含用户输入（项目/研究名等），防止破坏标记结构。 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** 生成自包含 HTML 报告（可直接浏览器打开并打印为 PDF）。 */
/** 键值对 → 表格行（键值均转义）。 */
function keyValueRows(rows: Array<[string, string]>): string {
  return rows
    .map(([key, value]) => `<tr><th>${escapeHtml(key)}</th><td>${escapeHtml(value)}</td></tr>`)
    .join("\n");
}

/** 时间序列采样 → 表格行。 */
function sampleRows(samples: Array<[string, string]>): string {
  return samples
    .map(([time, value]) => `<tr><td>${escapeHtml(time)}</td><td>${escapeHtml(value)}</td></tr>`)
    .join("\n");
}

export function buildReportHtml(input: ReportInput): string {
  const parameterRows = keyValueRows(input.parameterRows);
  const geometryRows = keyValueRows(input.geometryRows ?? []);
  const probeRows = keyValueRows(input.probeRows ?? []);
  const timeSeriesTables = (input.timeSeriesTables ?? [])
    .map(
      (table) =>
        `<h3>探针 ${escapeHtml(table.probeLabel)} 时间序列</h3>` +
        `<table><thead><tr><th>时间 (s)</th><th>值</th></tr></thead><tbody>${sampleRows(table.samples)}</tbody></table>`,
    )
    .join("\n");
  const geometrySection =
    (input.geometryRows?.length ?? 0) > 0
      ? `<h2>几何摘要</h2>\n<table>\n${geometryRows}\n</table>`
      : "";
  const probeSection =
    (input.probeRows?.length ?? 0) > 0 ? `<h2>探针数值</h2>\n<table>\n${probeRows}\n</table>` : "";
  const timeSeriesSection =
    (input.timeSeriesTables?.length ?? 0) > 0 ? `<h2>探针时间序列</h2>\n${timeSeriesTables}` : "";
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
${geometrySection}
<h2>结果</h2>
${fieldStats}
${probeSection}
${timeSeriesSection}
${snapshots}
<p class="meta">由 Kairos 生成 · 数值结果请结合材料数据来源与网格质量评估。</p>
</body>
</html>
`;
}
