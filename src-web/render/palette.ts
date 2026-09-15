/** 结果云图的统一蓝橙色标：WebGL、WebGPU 与图例共用。 */
export const FIELD_COLD_RGB = [0.05, 0.33, 0.66] as const;
export const FIELD_HOT_RGB = [0.94, 0.33, 0.13] as const;
const FIELD_COLD_HEX = "#0d54a8";
const FIELD_HOT_HEX = "#f05221";
export const FIELD_LEGEND_STYLE = `linear-gradient(180deg, ${FIELD_HOT_HEX}, ${FIELD_COLD_HEX})`;

export function wgslFieldColor(value: readonly [number, number, number]): string {
  return `vec3<f32>(${value.join(", ")})`;
}
