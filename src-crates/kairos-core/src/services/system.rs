//! 领域服务：系统信息。

use crate::models::system::SystemInfo;

/// 汇总应用基本信息。`os` 来自运行环境；应用名与版本由适配层注入，
/// 使本 crate 不依赖具体的打包配置。
pub fn system_info(name: &str, version: &str) -> SystemInfo {
    SystemInfo {
        name: name.to_string(),
        version: version.to_string(),
        os: std::env::consts::OS.to_string(),
    }
}
