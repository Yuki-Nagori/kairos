//! 统一错误契约：领域层所有错误归一为 [`KairosError`]，经 IPC 序列化为
//! `{ code, message }` 结构；前端 `src-web/lib/ipc.ts` 将其还原为 `CommandError`。

use std::fmt;

use serde::Serialize;

/// 错误类别：跨 IPC 的稳定代码。前端按 `code` 分支处理，禁止对 message 做文本匹配。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 输入不合法（参数缺失、超出量程等）。
    Validation,
    /// 目标资源不存在。
    NotFound,
    /// 文件 / 系统交互失败。
    Io,
    /// 领域计算失败（如求解不收敛）。
    Solver,
    /// 未归类的内部错误。
    Internal,
}

impl ErrorKind {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::Io => "io",
            Self::Solver => "solver",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KairosError {
    kind: ErrorKind,
    message: String,
}

impl KairosError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, message)
    }

    pub fn solver(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Solver, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for KairosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind.as_code(), self.message)
    }
}

impl std::error::Error for KairosError {}

/// IPC 错误契约：Tauri 命令的 `Err` 必须实现 Serialize（官方要求），
/// 这里稳定为 `{ code, message }` 两字段结构。
impl Serialize for KairosError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut structured = serializer.serialize_struct("KairosError", 2)?;
        structured.serialize_field("code", self.kind.as_code())?;
        structured.serialize_field("message", &self.message)?;
        structured.end()
    }
}

/// 领域层统一 Result 别名：所有可能失败的领域函数都返回它。
pub type Result<T> = std::result::Result<T, KairosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_kind() {
        assert_eq!(KairosError::validation("x").kind(), ErrorKind::Validation);
        assert_eq!(KairosError::not_found("x").kind(), ErrorKind::NotFound);
        assert_eq!(KairosError::io("x").kind(), ErrorKind::Io);
        assert_eq!(KairosError::solver("x").kind(), ErrorKind::Solver);
        assert_eq!(KairosError::internal("x").kind(), ErrorKind::Internal);
    }

    #[test]
    fn display_contains_message() {
        let error = KairosError::solver("迭代不收敛");
        assert!(error.to_string().contains("迭代不收敛"));
    }
}
