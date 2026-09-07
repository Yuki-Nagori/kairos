/** 快照注册表：面板把自己的 canvas 登记进来，报告导出时截图使用。 */

const snapshots = new Map<string, HTMLCanvasElement>();

export function registerSnapshot(id: string, canvas: HTMLCanvasElement): void {
  snapshots.set(id, canvas);
}

/** 导出 PNG data URL；画布不存在或为空返回 null。 */
export function getSnapshotDataUrl(id: string): string | null {
  const canvas = snapshots.get(id);
  if (canvas === undefined || canvas.width === 0 || canvas.height === 0) {
    return null;
  }
  try {
    return canvas.toDataURL("image/png");
  } catch {
    return null;
  }
}
