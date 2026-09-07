import { describe, expect, it, vi } from "vitest";
import { createStore, select } from "./store";

describe("createStore", () => {
  it("merges patches and notifies subscribers", () => {
    const store = createStore({ count: 0, label: "a" });
    const seen: Array<{ count: number; label: string }> = [];
    store.subscribe((state) => seen.push({ ...state }));

    store.set({ count: 1 });
    store.set({ label: "b" });

    expect(store.get()).toEqual({ count: 1, label: "b" });
    expect(seen).toEqual([
      { count: 1, label: "a" },
      { count: 1, label: "b" },
    ]);
  });

  it("supports unsubscribing", () => {
    const store = createStore({ hits: 0 });
    let calls = 0;
    const unsubscribe = store.subscribe(() => {
      calls += 1;
    });
    store.set({ hits: 1 });
    unsubscribe();
    store.set({ hits: 2 });
    expect(calls).toBe(1);
  });
});

describe("select", () => {
  it("calls the listener immediately and on relevant changes only", () => {
    const store = createStore({ count: 0, label: "a" });
    const listener = vi.fn();
    select(store, (state) => state.count, listener);

    expect(listener).toHaveBeenNthCalledWith(1, 0);
    store.set({ label: "b" });
    expect(listener).toHaveBeenCalledTimes(1);
    store.set({ count: 1 });
    expect(listener).toHaveBeenNthCalledWith(2, 1);
  });

  it("supports unsubscribing", () => {
    const store = createStore({ count: 0 });
    const listener = vi.fn();
    const unsubscribe = select(store, (state) => state.count, listener);
    store.set({ count: 1 });
    unsubscribe();
    store.set({ count: 2 });
    expect(listener).toHaveBeenCalledTimes(2);
  });
});
