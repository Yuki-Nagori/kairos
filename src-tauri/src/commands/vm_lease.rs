//! 虚拟机生命周期租约：作业需要时自动拉起虚拟机，作业跑完且空闲时自动关闭（省内存）。
//!
//! 两个标记都是**进程级单例**：自动拉起标记决定「空闲时能不能关」，Shell 占用标记
//! 决定「用户还在不在里面操作」。策略判定在 core（`vm::should_stop_when_idle`），
//! 这里只做标记记账。

use std::sync::atomic::{AtomicBool, Ordering};

static AUTO_STARTED: AtomicBool = AtomicBool::new(false);
static SHELL_OPEN: AtomicBool = AtomicBool::new(false);

/// 记录「虚拟机是作业自动拉起的」（用户手动启动的不记，空闲时不关）。
pub fn mark_auto_started() {
    AUTO_STARTED.store(true, Ordering::SeqCst);
}

/// 虚拟机是否由作业自动拉起。
pub fn auto_started() -> bool {
    AUTO_STARTED.load(Ordering::SeqCst)
}

/// 清除自动拉起标记（主动停止 / 应用退出联动）。
pub fn clear_auto_started() {
    AUTO_STARTED.store(false, Ordering::SeqCst);
}

/// 交互 Shell 会话开合（用户在里面操作时作业收尾不关虚拟机）。
pub fn set_shell_open(open: bool) {
    SHELL_OPEN.store(open, Ordering::SeqCst);
}

/// 当前是否有交互 Shell 会话。
pub fn shell_open() -> bool {
    SHELL_OPEN.load(Ordering::SeqCst)
}
