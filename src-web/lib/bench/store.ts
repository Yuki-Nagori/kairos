import { Bench } from "tinybench";
import { createStore, select } from "../store";

/** 状态容器基准（tinybench）：预算见 ai-docs/perf-budget.md，数字随 T21 评审对照更新。 */
const bench = new Bench({ warmupIterations: 200, iterations: 20_000 });

const subscribed = createStore({ count: 0, label: "kairos" });
let sink = 0;
const unsubscribers = Array.from({ length: 10 }, () =>
  subscribed.subscribe((state) => {
    sink += state.count;
  }),
);

bench.add("set with 10 subscribers", () => {
  subscribed.set({ count: (subscribed.get().count + 1) % 1000 });
});

const observed = createStore({ count: 0, label: "a" });
const unsubscribedSlices = Array.from({ length: 5 }, () =>
  select(
    observed,
    (state) => state.count,
    () => undefined,
  ),
);

bench.add("set observed by 5 selects (slice unchanged)", () => {
  observed.set({ label: `${observed.get().label.length % 7}` });
});

bench.add("set observed by 5 selects (slice changed)", () => {
  observed.set({ count: (observed.get().count + 1) % 1000 });
});

bench.add("snapshot read", () => {
  sink += observed.get().count;
});

bench.add("subscribe + unsubscribe", () => {
  const unsubscribe = subscribed.subscribe((state) => {
    sink += state.count;
  });
  unsubscribe();
});

await bench.run();
console.table(bench.table());

// 消费累计值，避免基准体被死代码消除。
if (sink < 0) {
  console.error(sink);
}
for (const unsubscribe of unsubscribers) {
  unsubscribe();
}
for (const unsubscribe of unsubscribedSlices) {
  unsubscribe();
}
