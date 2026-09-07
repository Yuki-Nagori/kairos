/** 与 Rust 侧 serde 结构一一对应的共享类型。 */

/** 对应 `kairos-core::models::system::SystemInfo`（camelCase 序列化契约）。 */
export interface SystemInfo {
  name: string;
  version: string;
  os: string;
}

/** 对应 `kairos-core::models::project::Project`（.kairos 工程文件 schema v1）。 */
export interface Project {
  schemaVersion: number;
  id: string;
  name: string;
  createdMs: number;
  updatedMs: number;
  studies: Study[];
}

/** 对应 `kairos-core::models::project::Study`。 */
export interface Study {
  id: string;
  name: string;
  createdMs: number;
}

/** 对应 `kairos-core::models::project::RecentProject`（最近打开的工程）。 */
export interface RecentProject {
  path: string;
  name: string;
  lastOpenedMs: number;
}
