/** XY 图表数据与 CSV 工具；绘制由 uPlot 渲染层负责。 */
export { chartRange, downsampleSeries } from "./chart-data";

/** 导出 CSV 内容（含表头）。 */
function csvCell(value: string | number): string {
  const text = String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

/** 导出 RFC 4180 兼容 CSV：BOM 让 Excel 正确识别 UTF-8，CRLF 兼容表格软件。 */
export function toCsv(headers: string[], rows: Array<Array<string | number>>): string {
  const lines = [headers.map(csvCell).join(",")];
  for (const row of rows) {
    lines.push(row.map(csvCell).join(","));
  }
  return `\uFEFF${lines.join("\r\n")}\r\n`;
}
