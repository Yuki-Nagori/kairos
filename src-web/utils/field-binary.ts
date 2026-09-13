/** 二进制场解码：解析 Rust 侧 field_binary 编码的字节
 * [magic "KF1\0" 4B][meta_len u32 LE][meta JSON][值区]。
 * 值区格式由元数据的 format 决定：f32（默认，相对误差 ≤ 2^-24）/ f64（旧载荷），
 * 矢量载荷每单元 3 个值（x/y/z）。 */

const MAGIC = [0x4b, 0x46, 0x31, 0x00];

interface Envelope {
  meta: {
    field: string;
    timeDir: string;
    timeS: number;
    isMagnitude: boolean;
    complete: boolean;
    count: number;
    format?: string;
    vector?: boolean;
  };
  /** 值区起始字节与单值宽度。 */
  valuesStart: number;
  width: 4 | 8;
}

interface DecodedField {
  field: string;
  timeDir: string;
  timeS: number;
  isMagnitude: boolean;
  complete: boolean;
  values: number[];
}

interface DecodedVectorField {
  field: string;
  timeDir: string;
  timeS: number;
  complete: boolean;
  components: [number, number, number][];
}

function envelope(buffer: ArrayBuffer): Envelope {
  if (buffer.byteLength < 8) {
    throw new Error("二进制场数据无效：长度不足。");
  }
  const view = new DataView(buffer);
  for (let i = 0; i < MAGIC.length; i += 1) {
    if (view.getUint8(i) !== MAGIC[i]) {
      throw new Error("二进制场数据无效：魔数不匹配。");
    }
  }
  const metaLen = view.getUint32(4, true);
  const metaEnd = 8 + metaLen;
  if (metaEnd > buffer.byteLength) {
    throw new Error("二进制场数据无效：元数据被截断。");
  }
  const meta = JSON.parse(
    new TextDecoder().decode(new Uint8Array(buffer, 8, metaLen)),
  ) as Envelope["meta"];
  const format = meta.format ?? "f64";
  if (format !== "f32" && format !== "f64") {
    throw new Error(`二进制场数据无效：未知的值格式 ${format}。`);
  }
  const width = format === "f32" ? 4 : 8;
  const groups = meta.vector === true ? 3 : 1;
  if (metaEnd + meta.count * width * groups > buffer.byteLength) {
    throw new Error("二进制场数据无效：值区被截断。");
  }
  return { meta, valuesStart: metaEnd, width };
}

/** 解码标量场；格式不符抛 Error（按 CommandError 同样进入全局错误态）。 */
export function decodeFieldBinary(buffer: ArrayBuffer): DecodedField {
  const { meta, valuesStart, width } = envelope(buffer);
  if (meta.vector === true) {
    throw new Error("二进制场数据无效：该载荷是矢量场（请用矢量解码）。");
  }
  const view = new DataView(buffer);
  const values: number[] = [];
  for (let index = 0; index < meta.count; index += 1) {
    const offset = valuesStart + index * width;
    values.push(width === 4 ? view.getFloat32(offset, true) : view.getFloat64(offset, true));
  }
  return {
    field: meta.field,
    timeDir: meta.timeDir,
    timeS: meta.timeS,
    isMagnitude: meta.isMagnitude,
    complete: meta.complete,
    values,
  };
}

/** 解码矢量场三分量。 */
export function decodeVectorFieldBinary(buffer: ArrayBuffer): DecodedVectorField {
  const { meta, valuesStart, width } = envelope(buffer);
  if (meta.vector !== true) {
    throw new Error("二进制场数据无效：该载荷是标量场（请用标量解码）。");
  }
  const view = new DataView(buffer);
  const read = (offset: number): number =>
    width === 4 ? view.getFloat32(offset, true) : view.getFloat64(offset, true);
  const components: [number, number, number][] = [];
  for (let index = 0; index < meta.count; index += 1) {
    const base = valuesStart + index * 3 * width;
    components.push([read(base), read(base + width), read(base + 2 * width)]);
  }
  return {
    field: meta.field,
    timeDir: meta.timeDir,
    timeS: meta.timeS,
    complete: meta.complete,
    components,
  };
}
