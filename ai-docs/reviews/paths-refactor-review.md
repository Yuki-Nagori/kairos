# 路径处理收口评审（T21 清单 · 随修复执行）

背景：Linux/Windows CI 挂了两处——`commands/jobs.rs` 的原生分支引用了只在
`#[cfg(any(macos, windows))]` 下导入的 `vm_logic`（Linux 编译不过）；两个断言把
平台细节写死（`native_env_commands_quote_paths` 硬编码 `/`；`relative_paths_*` 里
`/etc/passwd` 在 Windows 上 `is_absolute()` 为 false）。根因是**路径处理散落各处、
部分手写字符串**。本轮把路径全部收口到 `std::path` + 单一工具模块。

| #   | 清单项         | 结论                                                                                                                                             |
| --- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | 架构一致性     | 通过。路径工具住 core（`services::paths`），命令层与 workspace 都只调它；前端不再解析路径，格式分派回 Rust                                       |
| 2   | 性能预算对照   | 通过。全是 O(组件数) 的库调用，无遍历热点；导入分派由一次 `Path::extension` 取代前端字符串切分                                                   |
| 3   | 代码与注释质量 | 通过。删掉 3 处手写实现（文件名清洗、报告名清洗、WSL 盘符字节判定）；分隔符归一收敛为**唯一**一处并写明理由                                      |
| 4   | 文档同步       | 通过。AGENTS.md 增「路径一律走库」红线，ARCHITECTURE §1.2 给入口表与两条硬性理由                                                                 |
| 5   | 测试覆盖缺口   | 通过。`paths` 7 个单测（拼装 / 存储归一 / 校验 / 清洗 / 主干 / 盘符映射 / 归一）；`format_from_path` + `parse_file` 分派有测试；core 行覆盖 100% |
| 6   | 依赖健康       | 无新增依赖（只用 std）                                                                                                                           |
| 7   | 安全           | 通过。相对路径校验覆盖绝对 / 根 / 盘符 / `..`，且盘符判定在 Unix 与 Windows 上语义一致（跨机器打开工程）                                         |
| 8   | 上轮遗留       | T21 清单里「平台 cfg 逐平台审计」的教训**扩写为三条**：① 新用法要在所有平台可见；② 断言不得依赖平台细节；③ 不可达分支要在无该平台语义时也能覆盖  |

## 收口内容

- 新增 `src-crates/kairos-core/src/services/paths.rs`：`join` / `to_storage` /
  `from_storage` / `validate_relative` / `sanitize_file_name` / `file_stem` /
  `normalize_separators` / `wsl_path`，全部基于 `std::path`（`join`、`file_name`、
  `file_stem`、`extension`、`components`、`as_os_str`、`with_extension`、
  `MAIN_SEPARATOR`）。
- `workspace.rs` 删掉手写的 `sanitize_file_name` / `file_stem`，相对路径校验与解析
  委托给 `paths`；工程文件扩展名用 `Path::with_extension`。
- `commands/project.rs` 报告文件名改用 `paths::file_stem` + `sanitize_file_name` +
  `Path::with_extension`（不再 `trim_end_matches(".html")` + `format!`）。
- `commands/jobs.rs` 的 WSL 路径映射下沉 core `paths::wsl_path`（盘符取「第一个组件的
  `as_os_str`」——Unix 解析成普通组件、Windows 解析成 `Prefix`，**同一代码路径**，
  两种平台都能测）；结构性组件用 `Path::file_name()==None` 丢弃，不写平台分叉。
- 导入分派：core 新增 `GeometryFormat` + `format_from_path`（`Path::extension`）+ `parse_file`；
  Tauri 三条导入命令合并为一条 `import_geometry`；前端删掉 `path.split(".")`，统一调
  `importGeometryFile`。
- 修复 CI 的两处：
  - `vm_logic` 导入去掉 `cfg`（原生分支在 Linux 也要用）；
  - `validate_relative` 增 `has_root()`（Windows 上 rooted 路径 `is_absolute()` 为 false）。

## 处理记录

- **立即修**：三处手写路径、一处前端扩展名解析、两处平台相关断言。
- **消融抽查**：把 `validate_relative` 的 `has_root()` 去掉 → Windows 形态的
  `/etc/passwd` 用例在 Unix 上仍绿（说明该分支只在 Windows 生效）→ 改为断言
  `\\etc\\passwd` 也被拒绝，Unix 上即刻变红，恢复后复绿。
- **回流任务**：无新增。
- **接受并记录**：`paths` 里保留 `normalize_separators`（把「平台不认识的另一种分隔符」
  换成 `MAIN_SEPARATOR`）——标准库没有跨平台分隔符归一函数，这是唯一需要一段字符串
  替换的地方，已在本模块与 §1.2 写明它是例外。
