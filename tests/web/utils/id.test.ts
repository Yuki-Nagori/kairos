import { afterEach, describe, expect, it, vi } from "vitest";
import { newId } from "../../../src-web/utils/id";

describe("newId", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("uses the platform UUID generator", () => {
    vi.stubGlobal("crypto", { randomUUID: vi.fn(() => "01234567-89ab-cdef-0123-456789abcdef") });
    expect(newId("study")).toBe("study-01234567-89ab-cdef-0123-456789abcdef");
  });

  it("falls back to a unique monotonic suffix when UUID is unavailable", () => {
    vi.stubGlobal("crypto", {});
    const first = newId("study");
    const second = newId("study");
    expect(first).toMatch(/^study-[a-z0-9]+-[a-z0-9]+$/);
    expect(second).toMatch(/^study-[a-z0-9]+-[a-z0-9]+$/);
    expect(first).not.toBe(second);
  });
});
