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

## 第二轮（Windows 复跑后的修正）

Windows CI 复跑又暴露两处，都是**把用户输入的「名字」当路径解析**导致的：

| 现象                                                                                                      | 根因                                                             | 修法                                                                                                                             |
| --------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `file_stem("x:y")` 在 Windows 得到 `y`、Unix 得到 `x_y`                                                   | Windows 把 `x:` 当盘符前缀，`Path::file_stem` 只取盘符后的部分   | 名字按**单个路径段**处理：`paths::path_segment` 先把保留字符（含分隔符与冒号）换成下划线，**不解析路径**——名字本来就没有目录语义 |
| `validate_relative("C:/Users/x/part.stl")` 报错文案在 Windows 是「必须是相对路径」、Unix 是「不得含盘符」 | 两个平台由不同判定先拦下（Windows `has_root()` / Unix 冒号检查） | 测试对该用例只断言「报错」，把文案断言留给平台无关的 `geometry/a:b.stl` 用例                                                     |

同时清掉三处「一边平台永不执行」的写法（都是覆盖率门禁当场照出来的）：

- `normalized` 的 `if MAIN_SEPARATOR == '/' { '\\' } else { '/' }` → 两种分隔符都替换成主分隔符
  （native 换成自己幂等），无平台分支；
- `drive_letter` 的 `chars.next()?`（空串分支不可达）→ 单 `match (char, char, char)`，
  所有分支都由测试触达；
- `file_stem` 的 `unwrap_or_else` 闭包（`path_segment` 已保证段有效 → 永不执行）→ 去闭包。

**原则沉淀**（已写进 `paths` 模块文档与 ARCHITECTURE §1.2）：
**名字 ≠ 路径**——用户输入的工程名 / 文件名 / 报告名用 `path_segment`（纯字符清洗，
不解析），真实文件路径取末段用 `file_name_of`（`Path::file_name`）。

## 第三轮（Windows 再次复跑）

只剩一条：`workspace.rs` 的 `relative_paths_reject_escape_and_absolute` 里
`validate_relative("C:/Users/x/part.stl")` 断言了「不得含盘符」文案——Windows 上该输入
先被 `has_root()` 拦下，文案是「必须是相对路径」。这一条在上一轮已经意识到（同文件里
改过一处），但漏改了 workspace 侧的副本；本轮改成只断言报错，文案断言统一留在
平台无关的 `geometry/a:b.stl` 用例上。

顺带把全仓同类断言扫了一遍（`grep '不得含盘符|必须是相对路径'`）：现在只剩 `paths.rs`
与 `workspace.rs` 各一条，且都挂在 `geometry/a:b.stl`（相对路径 + 冒号，两个平台都由
冒号规则命中，文案一致）。

## 处理记录

- **立即修**：三处手写路径、一处前端扩展名解析、两处平台相关断言。
- **消融抽查**：把 `validate_relative` 的 `has_root()` 去掉 → Windows 形态的
  `/etc/passwd` 用例在 Unix 上仍绿（说明该分支只在 Windows 生效）→ 改为断言
  `\\etc\\passwd` 也被拒绝，Unix 上即刻变红，恢复后复绿。
- **回流任务**：无新增。
- **接受并记录**：`paths` 里保留 `normalize_separators`（把「平台不认识的另一种分隔符」
  换成 `MAIN_SEPARATOR`）——标准库没有跨平台分隔符归一函数，这是唯一需要一段字符串
  替换的地方，已在本模块与 §1.2 写明它是例外。
