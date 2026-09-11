import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAppStore } from "../../../src-web/stores/app";
import { useProcessStore } from "../../../src-web/stores/process";
import { checkProcess } from "../../../src-web/api/process";

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
