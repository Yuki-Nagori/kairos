import { describe, expect, it, vi } from "vitest";

const { isTauriRuntimeMock } = vi.hoisted(() => ({ isTauriRuntimeMock: vi.fn() }));

vi.mock("./environment", () => ({ isTauriRuntime: isTauriRuntimeMock }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { IpcUnavailableError, invokeCommand } from "./ipc";

describe("invokeCommand 在浏览器预览环境", () => {
  it("抛出 IpcUnavailableError（预期内的环境提示）", async () => {
    isTauriRuntimeMock.mockReturnValue(false);
    await expect(invokeCommand("system_info")).rejects.toBeInstanceOf(IpcUnavailableError);
  });
});
