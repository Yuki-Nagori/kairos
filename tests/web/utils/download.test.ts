import { afterEach, describe, expect, it, vi } from "vitest";
import { downloadTextFile } from "../../../src-web/utils/download";

describe("downloadTextFile", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("creates a Blob URL, triggers the anchor, and revokes the URL", () => {
    const revoke = vi.fn();
    const createdUrl = "blob:mock";
    vi.stubGlobal("URL", {
      createObjectURL: vi.fn(() => createdUrl),
      revokeObjectURL: revoke,
    });
    const click = vi.fn();
    const anchor = { href: "", download: "", click } as {
      href: string;
      download: string;
      click: () => void;
    } as unknown as HTMLAnchorElement;
    vi.spyOn(document, "createElement").mockImplementation(() => anchor);

    downloadTextFile("field-T-100.csv", "node,p\n0,1\n");

    expect(click).toHaveBeenCalledTimes(1);
    expect(anchor.href).toBe(createdUrl);
    expect(anchor.download).toBe("field-T-100.csv");
    expect(revoke).toHaveBeenCalledWith(createdUrl);
  });

  it("passes the mime type to the Blob", () => {
    const createdUrl = "blob:mock";
    vi.stubGlobal("URL", { createObjectURL: vi.fn(() => createdUrl), revokeObjectURL: vi.fn() });
    const click = vi.fn();
    const anchor = { href: "", download: "", click } as unknown as HTMLAnchorElement;
    vi.spyOn(document, "createElement").mockImplementation(() => anchor);
    const blobSpy = vi.spyOn(globalThis, "Blob").mockImplementation(function FakeBlob(
      this: unknown,
      parts: BlobPart[],
    ) {
      (this as { parts: BlobPart[] }).parts = parts;
    } as unknown as typeof Blob);

    downloadTextFile("a.json", "{}", "application/json");

    expect(blobSpy).toHaveBeenCalledWith(["{}"], { type: "application/json" });
    expect(click).toHaveBeenCalledTimes(1);
  });
});
