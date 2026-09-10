/**
 * 前端状态层基准（tinybench）：度量 Vue reactive 热路径——写入通知、切片追踪、
 * 属性读与订阅生命周期。预算见 ai-docs/perf-budget.md。
 * 全部用例同步执行——异步 flush 会把通知推迟到微任务，测不出热路径成本。
 */
import { Bench } from "tinybench";
import { computed, reactive, watch, watchEffect } from "vue";

const bench = new Bench({ warmupIterations: 200, iterations: 20_000 });

// —— 用例 1：一次写入触发 10 个同步 watcher 的通知成本 ——
const subscribed = reactive({ count: 0, label: "kairos" });
let sink = 0;
for (let index = 0; index < 10; index += 1) {
  watch(
    subscribed,
    (state) => {
      sink += state.count;
    },
    { flush: "sync" },
  );
}
bench.add("reactive: write × 10 sync watchers", () => {
  subscribed.count += 1;
});

// —— 用例 2/3：一次写入对 5 个 computed 切片的成本（被追踪 vs 无关切片）——
const tracked = reactive({ count: 0, label: "kairos" });
for (let offset = 0; offset < 5; offset += 1) {
  const slice = computed(() => tracked.count + offset);
  watchEffect(
    () => {
      sink += slice.value;
    },
    { flush: "sync" },
  );
}
bench.add("reactive: write tracked slice × 5 computeds", () => {
  tracked.count += 1;
});
bench.add("reactive: write unrelated slice × 5 computeds", () => {
  tracked.label = `${sink}`;
});

// —— 用例 4：响应式属性读的开销 ——
const plain = reactive({ count: 0 });
bench.add("reactive: property read", () => {
  sink += plain.count;
});

// —— 用例 5：建立 watch 并立即停止的订阅生命周期成本 ——
const transient = reactive({ count: 0 });
bench.add("reactive: watch + stop", () => {
  const stop = watch(
    transient,
    (state) => {
      sink += state.count;
    },
    { flush: "sync" },
  );
  stop();
});

const tasks = await bench.run();

for (const task of tasks) {
  if (task.result.state !== "completed") {
    throw new Error(`基准未完成：${task.name}（${task.result.state}）`);
  }
  const nsPerOp = task.result.latency.mean * 1_000_000;
  process.stdout.write(`${task.name.padEnd(48)} ${nsPerOp.toFixed(1)} ns/op\n`);
}
if (sink === Number.POSITIVE_INFINITY) {
  throw new Error("unreachable");
}
