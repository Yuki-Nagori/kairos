import { describe, expect, it } from "vitest";
import { fixed, lengthFromMm, NOT_AVAILABLE, significant } from "../../../src-web/utils/format";

describe("fixed", () => {
  it("按定点位数输出", () => {
    expect(fixed(1.2345, 2)).toBe("1.23");
    expect(fixed(0, 3)).toBe("0.000");
  });

  it("非有限值给占位符（不把 NaN 漏到界面）", () => {
    expect(fixed(Number.NaN, 2)).toBe(NOT_AVAILABLE);
    expect(fixed(Number.POSITIVE_INFINITY, 2)).toBe(NOT_AVAILABLE);
    expect(fixed(Number.NEGATIVE_INFINITY, 2, "-")).toBe("-");
  });
});

describe("significant", () => {
  it("常用量级用定点", () => {
    expect(significant(353.279, 2)).toBe("353.28");
    expect(significant(0.5, 3)).toBe("0.500");
  });

  it("过大或过小转科学计数", () => {
    expect(significant(1234567, 2)).toBe("1.23e+6");
    expect(significant(1.2e-9, 2)).toBe("1.20e-9");
  });

  it("零与负值按定点处理，非有限值给占位符", () => {
    expect(significant(0, 2)).toBe("0.00");
    expect(significant(-2.5, 1)).toBe("-2.5");
    expect(significant(Number.NaN, 2)).toBe(NOT_AVAILABLE);
  });
});

describe("lengthFromMm", () => {
  it("按量级换档到 µm / mm / m", () => {
    expect(lengthFromMm(0.25, 2)).toBe("250.00 µm");
    expect(lengthFromMm(12.5, 2)).toBe("12.50 mm");
    expect(lengthFromMm(1500, 2)).toBe("1.50 m");
  });

  it("边界值落在 mm 档，非有限值给占位符", () => {
    expect(lengthFromMm(1, 2)).toBe("1.00 mm");
    expect(lengthFromMm(1000, 2)).toBe("1.00 m");
    expect(lengthFromMm(Number.NaN, 2)).toBe(NOT_AVAILABLE);
  });
});
