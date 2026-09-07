/** 与 Rust 侧 serde 结构一一对应的共享类型。 */

/** 对应 `kairos-core::models::system::SystemInfo`（camelCase 序列化契约）。 */
export interface SystemInfo {
  name: string;
  version: string;
  os: string;
}
