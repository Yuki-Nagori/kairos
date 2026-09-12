/**
 * 时间步动画节拍器：固定间隔逐帧推进场加载。背压——上一步加载未完成时
 * 跳过本拍，避免大场加载慢于节拍导致请求叠加排队；索引对时间步序列循环
 * 回绕。与 DOM / 渲染器解耦，可在假计时器下单测（消融 A12 缺口收编）。
 */

interface FieldAnimation {
  /** 开始循环加载 times；重复 start 先停掉旧节奏再重开。 */
  start(times: readonly string[], load: (dirName: string) => Promise<void>): void;
  /** 停止节奏；在途加载不取消（结果仍落地，只是不再推进新步）。 */
  stop(): void;
  /** 是否有加载在途（背压判据，测试观察）。 */
  readonly inFlight: boolean;
  /** 已触发加载的次数（测试观察）。 */
  readonly advanced: number;
}

export function createFieldAnimation(intervalMs: number): FieldAnimation {
  let timer: ReturnType<typeof setInterval> | null = null;
  let index = 0;
  let inFlightCount = 0;
  let advancedCount = 0;

  const tick = (times: readonly string[], load: (dirName: string) => Promise<void>): void => {
    // 背压：上一步未完成则跳过本拍（请求叠加会让队列越积越长）。
    if (inFlightCount > 0) {
      return;
    }
    inFlightCount += 1;
    advancedCount += 1;
    const dirName = times[index % times.length]!;
    index += 1;
    void load(dirName).finally(() => {
      inFlightCount -= 1;
    });
  };

  return {
    start(times, load) {
      this.stop();
      index = 0; // 重新开始总是从头循环（与原 startPlay 的 playIndex=0 语义一致）
      timer = setInterval(() => {
        tick(times, load);
      }, intervalMs);
    },
    stop() {
      if (timer !== null) {
        clearInterval(timer);
        timer = null;
      }
    },
    get inFlight(): boolean {
      return inFlightCount > 0;
    },
    get advanced(): number {
      return advancedCount;
    },
  };
}
