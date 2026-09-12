/**
 * 浏览器下载助手：把文本内容作为文件保存（Blob URL + 隐藏锚点）。
 * DOM 操作集中在 util 层——store 只负责数据与编排，不直接触碰 document。
 */

export function downloadTextFile(
  fileName: string,
  content: string,
  mime = "text/csv;charset=utf-8",
): void {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}
