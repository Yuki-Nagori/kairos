/** 快照注册表：面板把自己的 canvas 登记进来，报告导出时截图使用。 */

const snapshots = new Map<string, HTMLCanvasElement>();

export function registerSnapshot(id: string, canvas: HTMLCanvasElement): void {
  snapshots.set(id, canvas);
}

export function unregisterSnapshot(id: string): void {
  snapshots.delete(id);
}
