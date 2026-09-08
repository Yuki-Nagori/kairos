import { describe, expect, it, vi } from "vitest";
import { CommandError, invokeCommand } from "./ipc";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("./environment", () => ({ isTauriRuntime: () => true }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

describe("invokeCommand", () => {
  it("restores structured KairosError rejections into CommandError", async () => {
    invokeMock.mockRejectedValueOnce({ code: "solver", message: "迭代不收敛" });
    const error = await invokeCommand("solve_case", { caseId: "c-1" }).catch(
      (rejection: unknown) => rejection,
    );
    expect(error).toBeInstanceOf(CommandError);
    expect(error).toMatchObject({ code: "solver", message: "迭代不收敛" });
  });

  it("wraps unstructured rejections into plain errors", async () => {
    invokeMock.mockRejectedValueOnce("boom");
    const error = await invokeCommand("system_info").catch((rejection: unknown) => rejection);
    expect(error).toBeInstanceOf(Error);
    expect(error).not.toBeInstanceOf(CommandError);
    expect((error as Error).message).toBe("boom");
  });

  it("keeps the message of objects that carry no contract code", async () => {
    invokeMock.mockRejectedValueOnce({ message: "只有消息，没有 code" });
    const error = await invokeCommand("system_info").catch((rejection: unknown) => rejection);
    expect(error).not.toBeInstanceOf(CommandError);
    expect((error as Error).message).toBe("只有消息，没有 code");
  });

  it("passes native Error rejections through unchanged", async () => {
    const native = new Error("原生错误");
    invokeMock.mockRejectedValueOnce(native);
    const error = await invokeCommand("system_info").catch((rejection: unknown) => rejection);
    expect(error).toBe(native);
  });

  it("serializes objects without a message field", async () => {
    invokeMock.mockRejectedValueOnce({ reason: 42 });
    const error = await invokeCommand("system_info").catch((rejection: unknown) => rejection);
    expect(error).not.toBeInstanceOf(CommandError);
    expect((error as Error).message).toContain("42");
  });
});
