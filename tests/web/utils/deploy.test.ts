import { describe, expect, it } from "vitest";
import { isPendingDeploy } from "../../../src-web/utils/deploy";

describe("isPendingDeploy（更新未部署比对）", () => {
  it("下载侧无版本标签（静态直链）→ 不提示", () => {
    expect(isPendingDeploy(null, null)).toBe(false);
    expect(isPendingDeploy(null, "v0.1.1")).toBe(false);
  });

  it("VM 侧未知（未部署 / VM 未启动）→ 有下载即提示", () => {
    expect(isPendingDeploy("v0.2.0", null)).toBe(true);
  });

  it("标签相等 → 已是最新；不等 → 提示", () => {
    expect(isPendingDeploy("v0.2.0", "v0.2.0")).toBe(false);
    expect(isPendingDeploy("v0.2.0", "v0.1.1")).toBe(true);
  });
});
