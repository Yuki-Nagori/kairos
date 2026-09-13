/** 二进制场解码单测：手写字节流验证格式解析与各截断 / 魔数错误路径。 */
import { describe, expect, it } from "vitest";
import { decodeFieldBinary, decodeVectorFieldBinary } from "../../../src-web/utils/field-binary";

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
  /** 值区格式：默认 f64（旧载荷）；f32 为压缩后的默认形态。 */
  format?: "f32" | "f64";
  /** 三分量矢量载荷（每单元 x/y/z 平铺）。 */
  vector?: boolean;
}): ArrayBuffer {
  const format = options.format ?? "f64";
  const width = format === "f32" ? 4 : 8;
  const groups = options.vector === true ? 3 : 1;
  const meta = JSON.stringify({
    field: options.field,
    timeDir: options.timeDir,
    timeS: options.timeS,
    isMagnitude: options.isMagnitude,
    complete: options.complete,
    count: options.values.length / groups,
    format,
    vector: options.vector === true,
  });
  const full = 8 + meta.length + options.values.length * width;
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
    if (width === 4) {
      sourceView.setFloat32(valuesStart + index * 4, value, true);
    } else {
      sourceView.setFloat64(valuesStart + index * 8, value, true);
    }
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

describe("压缩与矢量载荷", () => {
  it("f32 值区按半宽解析且保持量级", () => {
    const field = decodeFieldBinary(
      buildBuffer({
        field: "T",
        timeDir: "0.5",
        timeS: 0.5,
        isMagnitude: false,
        complete: true,
        values: [300.25, -1e-4, 0],
        format: "f32",
      }),
    );
    expect(field.values[0]).toBeCloseTo(300.25, 3);
    expect(field.values[1]).toBeCloseTo(-1e-4, 10);
    expect(field.values[2]).toBe(0);
    // f32 载荷比 f64 短：同样的值区宽度减半
    const f64Buffer = buildBuffer({
      field: "T",
      timeDir: "0.5",
      timeS: 0.5,
      isMagnitude: false,
      complete: true,
      values: [300.25, -1e-4, 0],
    });
    const f32Buffer = buildBuffer({
      field: "T",
      timeDir: "0.5",
      timeS: 0.5,
      isMagnitude: false,
      complete: true,
      values: [300.25, -1e-4, 0],
      format: "f32",
    });
    // 元数据里多出的 format/vector 字段不计，只看值区：f32 少 12 字节（3 × 4）
    expect(f32Buffer.byteLength).toBeLessThan(f64Buffer.byteLength);
  });

  it("未知值格式与矢量载荷在标量入口报错", () => {
    // 手工构造：编码端不会产出未知格式，这里直接拼字节流
    const meta = JSON.stringify({
      field: "T",
      timeDir: "0",
      timeS: 0,
      isMagnitude: false,
      complete: true,
      count: 1,
      format: "f16",
      vector: false,
    });
    const crafted = new ArrayBuffer(8 + meta.length + 8);
    const view = new DataView(crafted);
    [0x4b, 0x46, 0x31, 0x00].forEach((byte, index) => view.setUint8(index, byte));
    view.setUint32(4, meta.length, true);
    for (let i = 0; i < meta.length; i += 1) {
      view.setUint8(8 + i, meta.charCodeAt(i));
    }
    view.setFloat64(8 + meta.length, 1, true);
    expect(() => decodeFieldBinary(crafted)).toThrow("未知的值格式");

    const vectorPayload = buildBuffer({
      field: "D",
      timeDir: "2",
      timeS: 2,
      isMagnitude: false,
      complete: true,
      values: [1, 2, 3],
      vector: true,
    });
    expect(() => decodeFieldBinary(vectorPayload)).toThrow("该载荷是矢量场");
  });

  it("矢量载荷解析三分量；标量载荷在矢量入口报错", () => {
    const vector = decodeVectorFieldBinary(
      buildBuffer({
        field: "D",
        timeDir: "2",
        timeS: 2,
        isMagnitude: false,
        complete: true,
        values: [0.001, -0.002, 0, 1.5e-4, 2.5e-5, -3.5e-5],
        vector: true,
        format: "f32",
      }),
    );
    expect(vector.field).toBe("D");
    expect(vector.timeDir).toBe("2");
    expect(vector.complete).toBe(true);
    expect(vector.components).toHaveLength(2);
    expect(vector.components[0]![0]).toBeCloseTo(0.001, 7);
    expect(vector.components[0]![1]).toBeCloseTo(-0.002, 7);
    expect(vector.components[1]![2]).toBeCloseTo(-3.5e-5, 10);

    const scalarPayload = buildBuffer({
      field: "T",
      timeDir: "0",
      timeS: 0,
      isMagnitude: false,
      complete: true,
      values: [1],
    });
    expect(() => decodeVectorFieldBinary(scalarPayload)).toThrow("该载荷是标量场");
  });

  it("旧载荷缺少 format 字段时按 f64 解析；矢量支持 f64 值区", () => {
    // 缺 format / vector 字段的历史载荷：按 f64 标量读
    const legacyMeta = JSON.stringify({
      field: "T",
      timeDir: "0",
      timeS: 0,
      isMagnitude: false,
      complete: true,
      count: 1,
    });
    const legacy = new ArrayBuffer(8 + legacyMeta.length + 8);
    const legacyView = new DataView(legacy);
    [0x4b, 0x46, 0x31, 0x00].forEach((byte, index) => legacyView.setUint8(index, byte));
    legacyView.setUint32(4, legacyMeta.length, true);
    for (let i = 0; i < legacyMeta.length; i += 1) {
      legacyView.setUint8(8 + i, legacyMeta.charCodeAt(i));
    }
    legacyView.setFloat64(8 + legacyMeta.length, 2.5, true);
    expect(decodeFieldBinary(legacy).values).toEqual([2.5]);

    // 矢量载荷的 f64 值区（编码端当前用 f32，解码端兼容 f64）
    const vectorF64 = decodeVectorFieldBinary(
      buildBuffer({
        field: "U",
        timeDir: "1",
        timeS: 1,
        isMagnitude: false,
        complete: true,
        values: [1, 2, 3],
        vector: true,
        format: "f64",
      }),
    );
    expect(vectorF64.components).toEqual([[1, 2, 3]]);
  });

  it("矢量载荷值区截断时报错", () => {
    const truncated = buildBuffer({
      field: "D",
      timeDir: "2",
      timeS: 2,
      isMagnitude: false,
      complete: true,
      values: [1, 2, 3],
      vector: true,
      format: "f32",
      truncate: 4,
    });
    expect(() => decodeVectorFieldBinary(truncated)).toThrow("值区被截断");
  });
});
