//! 通用工具：无领域语义、可被任意层复用（`error ← utils ← models / services`）。
//!
//! 准入条件：函数签名与注释里不出现网格 / 场 / 工况 / 方案等领域词汇，出现即属
//! `services/`——否则工具层会退化成第二个领域层。接口约定、`unwrap` / `expect`
//! 边界与各模块的用法见仓库根 `ai-docs/rust-conventions.md`。

pub mod float;
pub mod fs;
pub mod regex;
pub mod shell;
pub mod time;
pub mod utf8;
