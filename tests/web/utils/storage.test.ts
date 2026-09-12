import { beforeEach, describe, expect, it } from "vitest";
import {
  storageGet,
  storageIndex,
  storageKey,
  storageRemove,
  storageSet,
} from "../../../src-web/utils/storage";

describe("storage 网关", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("storageKey 生成全局唯一前缀形态", () => {
    expect(storageKey("layout", "left")).toBe("kairos:layout:left");
    expect(storageKey("panel", "流程引导")).toBe("kairos:panel:流程引导");
  });

  it("set/get 往返支持布尔、对象与数组（统一 JSON 形态）", () => {
    storageSet(storageKey("t", "b"), true);
    storageSet(storageKey("t", "o"), { a: 1 });
    storageSet(storageKey("t", "a"), [1, 2]);
    expect(storageGet(storageKey("t", "b"), false)).toBe(true);
    expect(storageGet(storageKey("t", "o"), { a: 0 })).toEqual({ a: 1 });
    expect(storageGet(storageKey("t", "a"), [] as number[])).toEqual([1, 2]);
  });

  it("缺 key 回退 fallback；损坏 JSON 不抛错按缺省处理", () => {
    expect(storageGet("kairos:t:missing", 42)).toBe(42);
    localStorage.setItem("kairos:t:broken", "{not json");
    expect(storageGet("kairos:t:broken", "fallback")).toBe("fallback");
  });

  it("storageRemove 删除条目", () => {
    storageSet(storageKey("t", "x"), 1);
    storageRemove(storageKey("t", "x"));
    expect(localStorage.getItem("kairos:t:x")).toBeNull();
  });

  it("storageIndex：登记幂等、列表有序、remove 清单与条目", () => {
    const index = storageIndex();
    const domain = "preset-test";
    index.add(domain, "B");
    index.add(domain, "A");
    index.add(domain, "B"); // 幂等
    expect(index.list(domain)).toEqual(["B", "A"]);
    storageSet(storageKey(domain, "B"), { v: 1 });
    index.remove(domain, "B");
    expect(index.list(domain)).toEqual(["A"]);
    expect(localStorage.getItem("kairos:preset-test:B")).toBeNull();
  });
});
