import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useProcessStore } from "../../../src-web/stores/process";
import { checkProcess } from "../../../src-web/api/process";
import type { CaseOutcome } from "../../../src-web/types";

vi.mock("../../../src-web/api/process", () => ({
  checkProcess: vi.fn(),
}));

describe("process store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("校验通过：无问题清单并返回 true", async () => {
    vi.mocked(checkProcess).mockResolvedValue([]);
    const process = useProcessStore();
    await expect(process.checkProcess({} as never)).resolves.toBe(true);
    expect(process.issues).toEqual([]);
    expect(useAppStore().error).toBeNull();
  });

  it("校验有问题：问题清单入状态并返回 false", async () => {
    vi.mocked(checkProcess).mockResolvedValue(["熔体温度超出量程。"]);
    const process = useProcessStore();
    await expect(process.checkProcess({} as never)).resolves.toBe(false);
    expect(process.issues).toEqual(["熔体温度超出量程。"]);
  });

  it("命令失败进全局错误并按未通过处理", async () => {
    vi.mocked(checkProcess).mockRejectedValue(new Error("校验命令不可用"));
    const app = useAppStore();
    const process = useProcessStore();
    await expect(process.checkProcess({} as never)).resolves.toBe(false);
    expect(app.error?.message).toBe("校验命令不可用");
  });
});

function caseOutcome(inletAreaM2 = 5e-5): CaseOutcome {
  return {
    caseDir: "/case/s-1",
    inletAreaM2,
    inletEquivalentDiameterMm: 7.98,
    gates: [
      {
        index: 1,
        requestedRadiusMm: 4,
        requestedAreaMm2: 50.27,
        actualAreaMm2: 55.1,
        faceCount: 9,
        equivalentDiameterMm: 8.38,
        areaRatio: 1.1,
        expressible: true,
        minFaceAreaMm2: 6.2,
      },
    ],
    warnings: [],
  };
}

describe("case 入口口径回显", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("记录后按方案取有效面积；跨方案或未记录时为空", () => {
    const process = useProcessStore();
    expect(process.effectiveInletAreaM2("s-1")).toBeUndefined();
    process.recordCaseInlet("s-1", caseOutcome());
    expect(process.effectiveInletAreaM2("s-1")).toBe(5e-5);
    expect(process.effectiveInletAreaM2("s-2")).toBeUndefined();
    expect(process.effectiveInletAreaM2(null)).toBeUndefined();
  });

  it("入口面积为 0（无面）时视为无有效面积", () => {
    const process = useProcessStore();
    process.recordCaseInlet("s-1", caseOutcome(0));
    expect(process.effectiveInletAreaM2("s-1")).toBeUndefined();
  });
});
