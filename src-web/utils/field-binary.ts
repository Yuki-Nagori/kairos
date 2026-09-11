/** 二进制场解码：解析 Rust 侧 field_binary 编码的字节
 * [magic "KF1\\0" 4B][meta_len u32 LE][meta JSON][f64 值 × n LE]。 */

const MAGIC = [0x4b, 0x46, 0x31, 0x00];

interface DecodedField {
  field: string;
  timeDir: string;
  timeS: number;
  isMagnitude: boolean;
  complete: boolean;
  values: number[];
}

/** 解码二进制场；格式不符抛 Error（按 CommandError 同样进入全局错误态）。 */
export function decodeFieldBinary(buffer: ArrayBuffer): DecodedField {
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
  const meta = JSON.parse(new TextDecoder().decode(new Uint8Array(buffer, 8, metaLen))) as {
    field: string;
    timeDir: string;
    timeS: number;
    isMagnitude: boolean;
    complete: boolean;
    count: number;
  };
  const valuesEnd = metaEnd + meta.count * 8;
  if (valuesEnd > buffer.byteLength) {
    throw new Error("二进制场数据无效：值区被截断。");
  }
  const values: number[] = [];
  for (let index = 0; index < meta.count; index += 1) {
    values.push(view.getFloat64(metaEnd + index * 8, true));
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
