/** 二进制场解码单测：手写字节流验证格式解析与各截断 / 魔数错误路径。 */
import { describe, expect, it } from "vitest";
import { decodeFieldBinary } from "../../../src-web/utils/field-binary";

/** 按 Rust 侧 field_binary::encode 的布局构造有效字节流。 */
function buildBuffer(options: {
  field: string;
  timeDir: string;
  timeS: number;
  isMagnitude: boolean;
  complete: boolean;
  values: number[];
  magic?: Uint8Array;
  truncate?: number;
}): ArrayBuffer {
  const meta = JSON.stringify({
    field: options.field,
    timeDir: options.timeDir,
    timeS: options.timeS,
    isMagnitude: options.isMagnitude,
    complete: options.complete,
    count: options.values.length,
  });
  const full = 8 + meta.length + options.values.length * 8;
  const total = full - (options.truncate ?? 0);
  // 先写入完整内容，再按 truncate 截断（模拟传输中途被切断的字节流）。
  const source = new ArrayBuffer(full);
  const sourceView = new DataView(source);
  const magic = options.magic ?? new Uint8Array([0x4b, 0x46, 0x31, 0x00]);
  for (let i = 0; i < 4; i += 1) {
    sourceView.setUint8(i, magic[i] ?? 0);
  }
  sourceView.setUint32(4, meta.length, true);
  for (let i = 0; i < meta.length; i += 1) {
    sourceView.setUint8(8 + i, meta.charCodeAt(i));
  }
  const valuesStart = 8 + meta.length;
  options.values.forEach((value, index) => {
    sourceView.setFloat64(valuesStart + index * 8, value, true);
  });
  return source.slice(0, total);
}

describe("decodeFieldBinary", () => {
  it("解析元数据与 LE 值区", () => {
    const field = decodeFieldBinary(
      buildBuffer({
        field: "T",
        timeDir: "0.100",
        timeS: 0.1,
        isMagnitude: false,
        complete: true,
        values: [1.5, -2.25, 0],
      }),
    );
    expect(field.field).toBe("T");
    expect(field.timeDir).toBe("0.100");
    expect(field.timeS).toBeCloseTo(0.1, 12);
    expect(field.values).toEqual([1.5, -2.25, 0]);
    expect(field.complete).toBe(true);
  });

  it("魔数不匹配抛错", () => {
    expect(() =>
      decodeFieldBinary(
        buildBuffer({
          field: "T",
          timeDir: "0",
          timeS: 0,
          isMagnitude: false,
          complete: true,
          values: [1],
          magic: new Uint8Array([1, 2, 3, 4]),
        }),
      ),
    ).toThrow("魔数不匹配");
  });

  it("元数据截断抛错", () => {
    const buffer = buildBuffer({
      field: "T",
      timeDir: "0",
      timeS: 0,
      isMagnitude: false,
      complete: true,
      values: [1],
      truncate: 16,
    });
    expect(() => decodeFieldBinary(buffer)).toThrow("元数据被截断");
  });

  it("值区截断抛错", () => {
    const buffer = buildBuffer({
      field: "T",
      timeDir: "0",
      timeS: 0,
      isMagnitude: false,
      complete: true,
      values: [1, 2],
      truncate: 10,
    });
    expect(() => decodeFieldBinary(buffer)).toThrow("值区被截断");
  });

  it("长度不足 8 字节抛错", () => {
    expect(() => decodeFieldBinary(new ArrayBuffer(4))).toThrow("长度不足");
  });
});
