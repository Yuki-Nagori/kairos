/**
 * 前端状态层基准（tinybench）：预算见 ai-docs/perf-budget.md。
 *
 * 状态层已迁移 Vue reactive，用例与迁移前一一对应，保证前后基线可对照：
 * 订阅者通知 → flush:"sync" 的 watch（同步触发，与旧 store 的同步通知同语义）；
 * select 切片订阅 → computed + watchEffect 消费；快照读 → reactive 读。
 * 全部用例同步执行——异步 flush 会把通知推迟到微任务，测不出热路径成本。
 */
import { Bench } from "tinybench";
import { computed, reactive, watch, watchEffect } from "vue";

const bench = new Bench({ warmupIterations: 200, iterations: 20_000 });

// —— 用例 1：一次写入触发 10 个订阅者（对标旧「set × 10 订阅者」）——
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

// —— 用例 2/3：5 个切片追踪消费（对标旧「set × 5 select」）——
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

// —— 用例 4：响应式读（对标旧「快照读」）——
const plain = reactive({ count: 0 });
bench.add("reactive: property read", () => {
  sink += plain.count;
});

// —— 用例 5：订阅 + 退订（对标旧「subscribe + unsubscribe」）——
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
