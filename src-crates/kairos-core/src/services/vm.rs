//! 虚拟机适配的纯逻辑：provider 判定、命令行参数构造与输出解析。
//! 进程副作用（安装 / 启动 / Shell 子进程）全部住在 src-tauri 适配层；
//! 这里只产出参数与解析结果，保证可测（覆盖率门槛适用）。

use std::path::{Path, PathBuf};

use crate::models::vm::{VmProviderKind, VmState, VmStatus};

/// 受管实例名：Kairos 自己管理的 multipass 虚拟机，与 moldingFoam README
/// 的 of14 开发虚拟机相互独立（规格与镜像仍对齐其验证配方）。
pub const INSTANCE_NAME: &str = "kairos";
/// WSL 受管发行版（`wsl --install -d` 的目标；v1 固定，自装发行版不受管）。
pub const WSL_DISTRO: &str = "Ubuntu-24.04";

/// 平台 → provider 的唯一映射；Linux 原生即目标环境，无需虚拟机。
pub fn provider_for(os: &str) -> Option<VmProviderKind> {
    match os {
        "macos" => Some(VmProviderKind::Multipass),
        "windows" => Some(VmProviderKind::Wsl),
        "linux" => Some(VmProviderKind::Native),
        _ => None,
    }
}

/// 工具版本探测命令（成功退出即视为已安装）。
pub fn version_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "--version".into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--status".into()],
        // 原生环境恒就绪：探测命令用 /usr/bin/true（恒成功），无需安装。
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 一键安装命令。Windows 走 UAC 提权（输出不可回传）；macOS 依赖 Homebrew。
/// 原生环境没有「安装虚拟机」这一步，用 `None` 表达（同 `start_args` / `ready_probe_args`）：
/// 返回空表会让调用点拿到一个 `args[0]` 就越界的参数表。
pub fn install_args(provider: VmProviderKind) -> Option<Vec<String>> {
    match provider {
        VmProviderKind::Multipass => Some(vec![
            "brew".into(),
            "install".into(),
            "--cask".into(),
            "multipass".into(),
        ]),
        VmProviderKind::Wsl => Some(vec![
            "powershell".into(),
            "-Command".into(),
            "Start-Process wsl -ArgumentList '--install' -Verb RunAs".into(),
        ]),
        VmProviderKind::Native => None,
    }
}

/// 实例清单 / 状态查询命令。
pub fn instance_info_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "info".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "-l".into(), "-v".into()],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 虚拟机资源规格（multipass 用；wsl/原生环境由系统自管，不消费此结构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmResources {
    pub cpus: u32,
    pub memory_gib: u32,
    pub disk_gib: u32,
}

/// 由宿主机规格推导虚拟机默认规格。
/// - CPU：宿主核数的一半（给宿主留余量），收敛到 [2, 8]；
/// - 内存：宿主内存的一半，收敛到 [4, 16] GiB；
/// - 磁盘：multipass 磁盘是稀疏分配、按需增长，固定 80G 足够 openfoam14 + bundle。
pub fn plan_resources(host_cpus: u32, host_memory_gib: u32) -> VmResources {
    VmResources {
        cpus: (host_cpus / 2).clamp(2, 8),
        memory_gib: (host_memory_gib / 2).clamp(4, 16),
        disk_gib: 80,
    }
}

/// 视为中国大陆时区的 IANA / Windows 时区名（本机判断，不发起网络请求）。
pub const CN_TIMEZONES: &[&str] = &[
    "Asia/Shanghai",
    "Asia/Urumqi",
    "Asia/Chongqing",
    "Asia/Harbin",
    "Asia/Kashgar",
    "China Standard Time",
];

/// 中国大陆时区 → 云镜像走国内镜像源（multipassd 直连上游下载慢）。
pub fn is_china_timezone(tz: &str) -> bool {
    CN_TIMEZONES.contains(&tz)
}

/// 平台架构 → Ubuntu 云镜像文件名（noble / 24.04）。
pub fn image_file_name(arch: &str) -> Option<&'static str> {
    match arch {
        "aarch64" => Some("noble-server-cloudimg-arm64.img"),
        "x86_64" => Some("noble-server-cloudimg-amd64.img"),
        _ => None,
    }
}

/// 国内镜像源主机（云镜像与 apt 源共用；只做加速，不改上游来源）。
pub const CN_MIRROR_HOST: &str = "mirrors.tuna.tsinghua.edu.cn";

/// 国内镜像源候选（清华 TUNA，实测可达；按序尝试）。
pub fn image_mirror_urls(arch: &str) -> Vec<String> {
    let Some(name) = image_file_name(arch) else {
        return Vec::new();
    };
    vec![format!(
        "https://{CN_MIRROR_HOST}/ubuntu-cloud-images/noble/current/{name}"
    )]
}

/// 首次创建实例（multipass 按宿主规格拉起 Ubuntu；WSL 安装发行版）。
/// `image` 为本地云镜像路径（国内镜像源预下载后 file:// 导入）；
/// None 时由 multipass 直接拉取 `24.04`。
pub fn launch_args(
    provider: VmProviderKind,
    resources: &VmResources,
    image: Option<&str>,
) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => {
            let source = match image {
                Some(path) => format!("file://{path}"),
                None => "24.04".into(),
            };
            vec![
                "multipass".into(),
                "launch".into(),
                "--name".into(),
                INSTANCE_NAME.into(),
                "--cpus".into(),
                resources.cpus.to_string(),
                // --mem 已被 multipass 弃用，新脚本一律用 --memory。
                "--memory".into(),
                format!("{}G", resources.memory_gib),
                "--disk".into(),
                format!("{}G", resources.disk_gib),
                source,
            ]
        }
        VmProviderKind::Wsl => vec![
            "wsl".into(),
            "--install".into(),
            "-d".into(),
            WSL_DISTRO.into(),
        ],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 把已存在但停止的实例拉起来（WSL 无此步骤，实例随首次执行自启）。
pub fn start_args(provider: VmProviderKind) -> Option<Vec<String>> {
    match provider {
        VmProviderKind::Multipass => Some(vec![
            "multipass".into(),
            "start".into(),
            INSTANCE_NAME.into(),
        ]),
        VmProviderKind::Wsl | VmProviderKind::Native => None,
    }
}

/// 虚拟机可执行命令的探测：能跑通一条 `true` 即视为就绪。
///
/// 启动（`multipass start`）返回时 guest 往往还没起完 sshd，直接投作业会失败；
/// 作业前先按本探测轮询，直到可执行。WSL 无独立启动步骤（首次执行自启）。
pub fn ready_probe_args(provider: VmProviderKind) -> Option<Vec<String>> {
    match provider {
        VmProviderKind::Multipass => Some(vec![
            "multipass".into(),
            "exec".into(),
            INSTANCE_NAME.into(),
            "--".into(),
            "true".into(),
        ]),
        VmProviderKind::Wsl => Some(vec![
            "wsl".into(),
            "-d".into(),
            WSL_DISTRO.into(),
            "--".into(),
            "true".into(),
        ]),
        VmProviderKind::Native => None,
    }
}

/// 作业跑完、队列空闲时是否关闭虚拟机（省内存）。纯策略，便于穷举测试。
///
/// 只在「虚拟机是为作业自动拉起的」且「没有排队 / 运行中的作业」且「没有交互 Shell
/// 会话」时关闭：用户自己启动的实例、正在用 Shell 的实例都不动。
pub fn should_stop_when_idle(auto_started: bool, active_jobs: usize, shell_open: bool) -> bool {
    auto_started && active_jobs == 0 && !shell_open
}

/// 应用内 Shell：multipass 走 exec bash（管道友好，规避非 TTY 限制）；
/// wsl 本身就是管道友好的 Linux 进程桥。
pub fn shell_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => {
            vec![
                "multipass".into(),
                "exec".into(),
                INSTANCE_NAME.into(),
                "--".into(),
                "bash".into(),
                "--login".into(),
            ]
        }
        VmProviderKind::Wsl => vec!["wsl".into(), "-d".into(), WSL_DISTRO.into()],
        // 原生环境：Shell 就是本地登录 bash。
        VmProviderKind::Native => vec!["bash".into(), "--login".into()],
    }
}

/// 目标环境里执行一段 bash 脚本的命令行（脚本由本模块的 `*_command` 系列构造）。
///
/// 实例名 / 发行版名 / `bash -lc` 的形状只在这里出现一次：调用点全都要么在 VM 内
/// （multipass 的 `exec <实例> --`、WSL 的 `wsl -d <发行版>`），要么在本机
/// （原生 Linux 直接 `bash -lc`），把这段形状抄在调用点就等于把「跑在哪」的知识
/// 散到各处——改实例名会漏掉一半，而漏掉的那半只在运行时才炸。
pub fn bash_script_args(provider: VmProviderKind, script: &str) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec![
            "multipass".into(),
            "exec".into(),
            INSTANCE_NAME.into(),
            "--".into(),
            "bash".into(),
            "-lc".into(),
            script.into(),
        ],
        VmProviderKind::Wsl => vec![
            "wsl".into(),
            "-d".into(),
            WSL_DISTRO.into(),
            "bash".into(),
            "-lc".into(),
            script.into(),
        ],
        VmProviderKind::Native => vec!["bash".into(), "-lc".into(), script.into()],
    }
}

/// case 在 VM 内的落脚根目录：每个 case 解压成 `~/<叶子名>`。
pub const VM_CASE_ROOT: &str = "/home/ubuntu";

/// case 目录在 VM 内的暂存路径。multipass 的 sshfs 挂载权限映射不可用，
/// 作业执行前把 case 归档传进 VM 原生文件系统，两侧同名。
pub fn vm_case_dir(case_dir: &str) -> String {
    let name = std::path::Path::new(case_dir)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "case".to_string());
    format!("{VM_CASE_ROOT}/{name}")
}

/// VM 侧的归档中转文件名（传输用，解压后即删）。
pub const VM_ARCHIVE_NAME: &str = ".kairos-transfer.tgz";

/// VM 内 case 归档的中转路径：传进去 / 打包出来的归档都落在这里。
pub fn vm_archive_path() -> String {
    format!("{VM_CASE_ROOT}/{VM_ARCHIVE_NAME}")
}

/// VM 内 case 归档的解压命令：清掉同名目录后解到 case 根，最后删掉归档。
///
/// 归档由宿主按「父目录 + 叶子名」打包（`tar -C <父> -czf <归档> <叶子>`），条目自带
/// 叶子名前缀；解到 case 目录本身会多套一层同名目录，求解脚本 `cd ~/<叶子>`
/// 就找不到 `system/controlDict`。先删同名目录是为了重跑时不带上次的时间目录。
pub fn vm_case_extract_from_archive_command(vm_case: &str) -> String {
    let archive = vm_archive_path();
    format!(
        "rm -rf '{}' && tar -xzf '{archive}' -C {VM_CASE_ROOT} && rm -f '{archive}'",
        bash_single_quote(vm_case)
    )
}

/// VM 内结果归档的打包命令：把给定时间目录打包到中转路径（供传输回宿主）。
/// 时间目录名来自 `results::time_dir_names`（宿主侧扫描同一套判定）。
pub fn vm_results_pack_command(vm_case: &str, time_dirs: &[String]) -> String {
    let archive = vm_archive_path();
    format!(
        "cd '{}' && tar -czf '{archive}' {}",
        bash_single_quote(vm_case),
        time_dirs.join(" ")
    )
}

/// 回传前的清单命令：列出 case 内的时间目录（`[0-9]*`），供宿主挑选要打包的目录。
/// 无输出 = 求解没产生结果（调用方按「没有可回传的结果」处理）。
pub fn vm_results_list_command(vm_case: &str) -> String {
    format!(
        "cd '{}' && ls -d [0-9]* 2>/dev/null",
        bash_single_quote(vm_case)
    )
}

/// 宿主 → VM（或反向）的文件传输命令：`multipass transfer <源> <目标>`。
///
/// 复制 case 与回传结果都不走 stdin/stdout 管道：multipass 的管道通道在远端命令
/// 结束后偶发不回退（CLI 进程自旋），改成落盘传输 + 独立的解压/打包命令，
/// 每一步都是「命令跑完即返回」。
pub fn transfer_args(source: &str, target: &str) -> Vec<String> {
    vec![
        // args[0] 恒为可执行名（与本模块其它 *args 同构），调用点不再拼前缀。
        "multipass".to_string(),
        "transfer".to_string(),
        source.to_string(),
        target.to_string(),
    ]
}

/// VM 内求解环境的部署根目录（vm_deploy_bundle 解压 bundle 的目标）。
pub const ENV_ROOT: &str = "~/moldingfoam-env";

/// 环境树 bashrc 在 bundle 内的相对路径。
///
/// `openfoam14/` 是**上游 release 产物的目录名**（资产名同样是
/// `moldingFoam-openfoam14-<arch>-<date>.tar.xz`），Kairos 只消费不重命名；
/// 上游若调整 bundle 布局，改动集中在这里与 ENV_ROOT 两处。
pub const ENV_BASHRC: &str = "openfoam14/etc/bashrc";

/// 部署进 VM 的 bundle 归档名（传输目标与解压来源都用它）。
pub const VM_BUNDLE_ARCHIVE: &str = "moldingfoam-bundle.tar.xz";

/// 环境树内的路径（`<环境根>/<相对路径>`）。
///
/// `ENV_ROOT` 是 **shell 相对形态**（`~` 开头），拼进命令时不能加引号：单引号会
/// 阻止波浪号展开，`cat '~/…'` 在 guest 里找不到文件。
pub fn env_path(relative: &str) -> String {
    format!("{ENV_ROOT}/{relative}")
}

/// bundle 归档在 VM 内的**绝对**路径（传输目标；`tar -xJf` 的来源）。
pub fn vm_bundle_archive_path() -> String {
    format!("{VM_CASE_ROOT}/{VM_BUNDLE_ARCHIVE}")
}

/// `multipass transfer` 的 VM 侧端点：`<实例名>:<VM 内绝对路径>`（源 / 目标同构）。
pub fn vm_transfer_endpoint(vm_path: &str) -> String {
    format!("{INSTANCE_NAME}:{vm_path}")
}

/// bundle 归档的传输目标（源在宿主侧，由调用方给绝对路径）。
pub fn vm_bundle_transfer_target() -> String {
    vm_transfer_endpoint(&vm_bundle_archive_path())
}

/// 求解脚本首段：加载求解环境（`source ~/moldingfoam-env/openfoam14/etc/bashrc`）。
pub fn env_source_command() -> String {
    format!("source {}", env_path(ENV_BASHRC))
}

/// 环境就绪探测命令：确认 bashrc 存在（部署收尾校验 bundle 结构）。
pub fn env_probe_command() -> String {
    format!("test -f {}", env_path(ENV_BASHRC))
}

/// 部署环境树：建根目录 → 解压归档 → 校验 bashrc 就位。
/// 三步用 `&&` 串联：解压半截（归档损坏 / 磁盘满）不算部署成功。
pub fn env_deploy_command() -> String {
    format!(
        "mkdir -p {ENV_ROOT} && tar -xJf {} -C {ENV_ROOT} && {}",
        vm_bundle_archive_path(),
        env_probe_command()
    )
}

/// VM 内 apt 源切国内镜像。best-effort：源文件布局随发行版变化（sources.list 与
/// sources.list.d/*.sources 并存），失败不阻断部署（慢一点仍能装）。
pub fn apt_mirror_command() -> String {
    format!(
        "sudo sed -i 's|http://archive.ubuntu.com/ubuntu|https://{CN_MIRROR_HOST}/ubuntu|g; \
         s|http://security.ubuntu.com/ubuntu|https://{CN_MIRROR_HOST}/ubuntu|g' \
         /etc/apt/sources.list /etc/apt/sources.list.d/*.sources 2>/dev/null || true"
    )
}

/// 安装求解器运行时依赖（OpenMPI）：foamRun 链接 libmpi.so.40。末尾用
/// `ldconfig -p | grep -q` 复核——装完仍找不到库（镜像缺包 / 缓存未更新）
/// 必须判失败，否则作业跑到一半才报缺库。
pub fn openmpi_install_command() -> String {
    "sudo apt-get update -qq && sudo apt-get install -y -qq libopenmpi-dev openmpi-bin \
     && sudo ldconfig && ldconfig -p | grep -q libmpi.so.40"
        .to_string()
}

/// 求解会话写在 case 目录里的三个文件：标准输出日志、退出码、以及流式回读
/// 分隔哨兵。求解进程脱离会话后，客户端只靠这三个文件就能回传进度与结果。
pub const SOLVE_LOG_NAME: &str = "kairos-solve.log";
/// 退出码文件：内容为求解脚本的退出码（写完即代表求解结束）。
pub const SOLVE_EXIT_NAME: &str = ".kairos-solve.exit";
/// 流式回读命令输出里的分隔哨兵：哨兵之前的字节是日志，之后是退出码。
pub const STREAM_MARK: &str = "##KAIROS-STATUS##";

/// 求解脚本：记录会话 id → 加载求解环境 → 进 VM 内 case 目录 → 分步求解。
///
/// 路径按 shell 字面量转义（case 目录可能含空格 / 引号），与原生分支共用同一口径。
/// 桌面端作业线程与脚本化集成测试都用它，避免各写一份「source + cd + solve」。
pub fn solve_script(vm_case: &str, cores: u32) -> String {
    format!(
        "echo $$ > '{}' 2>/dev/null; {} && cd '{}' && {}",
        bash_single_quote(&solve_sid_path(vm_case)),
        env_source_command(),
        bash_single_quote(vm_case),
        crate::services::moldingfoam::solve_command(cores)
    )
}

/// 求解会话 id 文件名：脱离会话的求解在 VM 里跑，宿主侧拿不到它的 pid，
/// 取消只能靠它自己记下来的会话 id（见 [`solver_stop_command`]）。
pub const SOLVE_SID_NAME: &str = ".kairos-solve.sid";

/// 会话 id 文件在 VM 内的路径（case 目录下）。
pub fn solve_sid_path(vm_case: &str) -> String {
    format!("{vm_case}/{SOLVE_SID_NAME}")
}

/// 取消求解：按会话 id 整组终止（`kill -TERM -<sid>` 杀的是**进程组**，
/// decomposePar / mpirun 及其各 rank / reconstructPar 一并结束）。
///
/// 求解已结束或 id 文件缺失时是空操作（`|| true`）：取消是用户意图，不该因为
/// 「刚好跑完了」而报错。
pub fn solver_stop_command(vm_case: &str) -> String {
    format!(
        "kill -TERM -$(cat '{}' 2>/dev/null) 2>/dev/null || true",
        bash_single_quote(&solve_sid_path(vm_case))
    )
}

/// 脱离会话的求解启动命令：`setsid` 起独立会话（不再是 ssh 会话的子进程），
/// 输出重定向到 case 内的日志文件，退出码写进退出码文件。
///
/// 这样求解不再依赖 ssh 会话存活——multipass 的 exec 客户端一退出，systemd-logind
/// 就会回收该会话的进程，求解会在跑到一半时无声消失；`setsid` 让求解脱离会话，
/// 客户端只负责「启动」，日志与退出码落盘供后续回读。
pub fn detached_launch_command(vm_case: &str, script: &str) -> String {
    let dir = format!("'{}'", bash_single_quote(vm_case));
    let log = format!("{dir}/{SOLVE_LOG_NAME}");
    let exit = format!("{dir}/{SOLVE_EXIT_NAME}");
    // 脚本必须整体落在单引号里（`bash_single_quote` 只做**引号内**的转义）：少了
    // 外层引号时，脚本里自带的 `cd '<case>'` 会把引号配平打乱，`bash -lc` 直接
    // 「unexpected EOF while looking for matching `''」——求解根本没启动。
    format!(
        "rm -f {log} {exit}; (setsid bash -lc '{}' >> {log} 2>&1 < /dev/null; echo $? > {exit}) &",
        bash_single_quote(script)
    )
}

/// 流式回读命令：从 `offset` 字节起输出日志，随后打印哨兵与退出码。
///
/// 退出码文件尚未出现时输出 `-`（仍在求解）。调用方按返回值推进 offset，
/// 因此每次只回传新增字节，长作业不会重复搬运整份日志。
pub fn detached_read_command(vm_case: &str, offset: u64) -> String {
    let dir = format!("'{}'", bash_single_quote(vm_case));
    let log = format!("{dir}/{SOLVE_LOG_NAME}");
    let exit = format!("{dir}/{SOLVE_EXIT_NAME}");
    format!(
        "tail -c +{} {log} 2>/dev/null; printf '\n{STREAM_MARK}\n'; cat {exit} 2>/dev/null || printf -- '-'",
        offset.saturating_add(1)
    )
}

/// 解析流式回读输出：返回（本次新增的日志片段, 最新偏移量, 退出码）。
///
/// 退出码为 `None` 表示求解仍在进行；哨兵缺失（输出被截断）时按「无新增」处理，
/// 下一轮从头续读，不把残缺输出当成求解结束。
pub fn parse_read_output(stdout: &str, offset: u64) -> (String, u64, Option<i32>) {
    let Some((chunk, status)) = stdout.rsplit_once(STREAM_MARK) else {
        return (String::new(), offset, None);
    };
    let new_offset = offset + chunk.len() as u64;
    let code = status
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|_| status.trim() != "-");
    (chunk.to_string(), new_offset, code)
}

/// 原生（Linux）求解环境的目录名：应用数据目录下的 `moldingfoam-env`。
pub const NATIVE_ENV_DIR: &str = "moldingfoam-env";

/// 版本标记文件名（VM 与原生环境共用；「更新未部署」提醒的比对源）。
pub const RELEASE_TAG_FILE: &str = ".kairos-release-tag";

/// 原生环境的 bashrc 路径：`<env_root>/openfoam14/etc/bashrc`。
pub fn native_env_bashrc(env_root: &Path) -> PathBuf {
    env_root.join(ENV_BASHRC)
}

/// 原生环境版本标记路径：`<env_root>/.kairos-release-tag`。
pub fn native_env_tag(env_root: &Path) -> PathBuf {
    env_root.join(RELEASE_TAG_FILE)
}

/// VM 内版本标记路径（`~/moldingfoam-env/.kairos-release-tag`）。
pub fn vm_env_tag_path() -> String {
    env_path(RELEASE_TAG_FILE)
}

/// 版本标签文本归一：两端空白不算版本，空 / 全空白 → None（未部署 / 文件写坏）。
/// VM 内标记、本机记录、原生标记三处的读回都走这一处口径。
pub fn parse_env_tag(text: &str) -> Option<String> {
    let tag = text.trim();
    (!tag.is_empty()).then(|| tag.to_string())
}

/// 版本标记读取命令输出里的哨兵：哨兵之后到行尾是本轮读到的标记值。
///
/// multipass 在隐式拉起实例时会把转圈动画写进 **stdout**（同一行内用 `\x08` 重绘），
/// 哨兵让标记值与这类噪声可分辨（同 `STREAM_MARK` 的用法）。
pub const ENV_TAG_MARK: &str = "KAIROS-TAG:";

/// 读取 VM 内版本标记的命令：标记值带哨兵输出，文件缺失时哨兵后为空。
pub fn vm_env_tag_read_command() -> String {
    format!(
        "printf '{ENV_TAG_MARK}%s\\n' \"$(cat {} 2>/dev/null)\"",
        vm_env_tag_path()
    )
}

/// 写入 VM 内版本标记的命令（标记值来自下载清单，走单引号字面量转义）。
pub fn vm_env_tag_write_command(tag: &str) -> String {
    format!(
        "printf '%s' '{}' > {}",
        bash_single_quote(tag),
        vm_env_tag_path()
    )
}

/// 解析版本标记读取命令的 stdout：取最后一个哨兵之后、行尾之前的内容。
/// 哨兵缺失（输出被截断 / 命令异常）→ None，按「未部署」处理，不拿噪声当版本。
pub fn parse_env_tag_reply(stdout: &str) -> Option<String> {
    let (_, tail) = stdout.rsplit_once(ENV_TAG_MARK)?;
    parse_env_tag(&clean_terminal_line(
        tail.lines().next().unwrap_or_default(),
    ))
}

/// 已部署版本视图：VM 可达（Running）时以 VM 内标记为准——读到什么就是什么，
/// 标记缺失即「未部署」（环境树可能被手工删掉）；VM 不可达（停机 / 启动中 /
/// 状态未知）时回落本机记录。
///
/// 「部署过」是持久事实：作业跑完与应用退出都会关掉虚拟机（省内存），若把停机
/// 当作「未部署」，用户每次都要重新搬一遍 120MB 的环境树。
pub fn resolve_deployed_tag(
    vm_reachable: bool,
    vm_tag: Option<String>,
    last_deployed: Option<String>,
) -> Option<String> {
    if vm_reachable { vm_tag } else { last_deployed }
}

/// 原生求解脚本首段：加载本机解压好的求解环境。
/// 路径用单引号包裹并转义（应用数据目录可能含空格或引号）。
pub fn native_env_source_command(env_root: &Path) -> String {
    format!(
        "source '{}'",
        bash_single_quote(&native_env_bashrc(env_root).to_string_lossy())
    )
}

/// 原生环境就绪探测：bashrc 存在即视为已部署（bundle 结构校验）。
pub fn native_env_probe_command(env_root: &Path) -> String {
    format!(
        "test -f '{}'",
        bash_single_quote(&native_env_bashrc(env_root).to_string_lossy())
    )
}

/// 单引号内的字面量转义：`'` → `'\''`（shell 单引号串里唯一的转义形式）。
pub fn bash_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

/// 原生依赖处置提示：OpenMPI 是 foamRun 的动态链接依赖（libmpi.so.40）。
/// 返回空 = 无需提示；`env_ready` = bundle 已解压且 bashrc 就位。
pub fn native_dependency_hints(mpi_ready: bool, env_ready: bool) -> Vec<String> {
    let mut hints = Vec::new();
    if !env_ready {
        hints.push(
            "求解环境未就绪：请在依赖面板下载 moldingFoam bundle（本机解压后即可直接提交作业）。"
                .to_string(),
        );
    }
    if !mpi_ready {
        hints.push(
            "缺少 OpenMPI 运行时（foamRun 依赖 libmpi.so.40）：Debian/Ubuntu 上执行 \
             `sudo apt install libopenmpi3 openmpi-bin`。"
                .to_string(),
        );
    }
    hints
}

/// 停止受管实例（退出联动时以 detached 方式派发，不等其退出）。
pub fn stop_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "stop".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--terminate".into(), WSL_DISTRO.into()],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 解析 `multipass info <name>` 输出（命令失败由适配层先行判 Missing）。
pub fn parse_multipass_state(text: &str) -> VmState {
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("State:") {
            // 新版 multipass 打印 Running（首字母大写），旧版全大写：统一小写匹配。
            return match value.trim().to_lowercase().as_str() {
                "running" => VmState::Running,
                "starting" | "delaying shutdown" => VmState::Starting,
                "stopped" | "suspended" => VmState::Stopped,
                _ => VmState::Unknown,
            };
        }
    }
    VmState::Unknown
}

/// 解析 `wsl -l -v` 输出（UTF-16LE 解码后传入）。
/// 行形如 `  Ubuntu-24.04     Running           2`；缺发行版行即 Missing。
pub fn parse_wsl_list(text: &str) -> VmState {
    let mut found = VmState::Missing;
    for line in text.lines() {
        // 行首的 `*` 是「默认发行版」标记，先剥掉再对列。
        let line = line.trim_start();
        let line = line.strip_prefix("* ").unwrap_or(line);
        let mut tokens = line.split_whitespace();
        if tokens.next() == Some(WSL_DISTRO) {
            found = match tokens.next() {
                Some("Running") => VmState::Running,
                Some("Installing") | Some("Converting") => VmState::Starting,
                Some("Stopped") => VmState::Stopped,
                _ => VmState::Unknown,
            };
        }
    }
    found
}

/// 解码 wsl 系列命令的输出：Windows 下是 UTF-16LE（带 BOM 与 NUL 填充），
/// 其余平台按 UTF-8 处理。统一在本函数内归一成干净的多行文本。
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    let looks_utf16 = bytes.len() >= 2 && bytes.get(1) == Some(&0);
    let text = if looks_utf16 {
        let (pairs, _rest) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.replace('\0', "")
}

/// 清洗终端行：剥掉 ANSI 转义序列与全部控制字符（multipass 的转圈动画
/// 既有 CSI 也有退格 \x08 重绘、夹 NUL），并把 \r / \x08 分段折叠为
/// 最后一帧（spinner 语义：只有最新帧有意义）。
pub fn clean_terminal_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                // CSI 序列：跳过到结束字母；其他转义跳过单字符
                if chars.next() == Some('[') {
                    for follow in chars.by_ref() {
                        if follow.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            // 回退重绘（回车 / 退格）：清空已收字符，只保留最后一帧
            '\r' | '\u{8}' => out.clear(),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.trim_end().to_string()
}

/// 组装状态视图与用户提示。
pub fn build_status(
    provider: VmProviderKind,
    tool_installed: bool,
    instance_state: VmState,
) -> VmStatus {
    let instance_name = match provider {
        VmProviderKind::Multipass => INSTANCE_NAME.to_string(),
        VmProviderKind::Wsl => WSL_DISTRO.to_string(),
        VmProviderKind::Native => "localhost".to_string(),
    };
    // 原生环境无需虚拟机：恒就绪、恒可进 Shell，安装/启动/停止语义全部短路。
    let (tool_installed, instance_state) = if provider == VmProviderKind::Native {
        (true, VmState::Running)
    } else {
        (tool_installed, instance_state)
    };
    let hint = if provider == VmProviderKind::Native {
        NATIVE_HINT.to_string()
    } else if !tool_installed {
        install_hint(provider).to_string()
    } else {
        instance_hint(provider, instance_state).to_string()
    };
    VmStatus {
        provider,
        tool_installed,
        instance_name,
        instance_state,
        hint,
    }
}

/// Native 平台的兜底提示：正常路径下 build_status 已短路，永远轮不到
/// 虚拟机提示；仍给真实文案而非 panic，保证提示函数全分支可测。
const NATIVE_HINT: &str = "Linux 原生环境，无需虚拟机，可直接进入 Shell。";

/// 工具未安装时的安装引导提示。
fn install_hint(provider: VmProviderKind) -> &'static str {
    match provider {
        VmProviderKind::Multipass => {
            "未检测到 Multipass。点击「安装虚拟机」（约需数分钟，依赖 Homebrew）。"
        }
        VmProviderKind::Wsl => {
            "未检测到 WSL2。点击「安装虚拟机」（需管理员授权，完成后可能要求重启）。"
        }
        VmProviderKind::Native => NATIVE_HINT,
    }
}

/// 工具就绪后的实例状态提示。
fn instance_hint(provider: VmProviderKind, instance_state: VmState) -> &'static str {
    if provider == VmProviderKind::Native {
        return NATIVE_HINT;
    }
    match instance_state {
        // 走到这里 provider 只可能是 Multipass / Wsl（Native 已在上方返回）。
        VmState::Missing if provider == VmProviderKind::Wsl => {
            "WSL2 已就绪，Ubuntu-24.04 尚未安装。点击「启动虚拟机」安装发行版。"
        }
        VmState::Missing => {
            "Multipass 已就绪，虚拟机尚未创建。点击「启动虚拟机」开始拉起 Ubuntu 实例。"
        }
        VmState::Stopped => "虚拟机已创建但未运行。点击「启动虚拟机」。",
        VmState::Starting => "虚拟机启动中…",
        VmState::Running => "虚拟机运行中，可进入 Shell。",
        VmState::Unknown => "虚拟机状态未知，请重新探测。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 原生（Linux）环境路径：bashrc 与版本标记都挂在环境根下，
    /// 目录名与 VM 内一致（`moldingfoam-env/openfoam14/etc/bashrc`）。
    #[test]
    fn native_env_paths_mirror_vm_layout() {
        let root = Path::new("/home/u/.local/share/kairos/moldingfoam-env");
        assert_eq!(NATIVE_ENV_DIR, "moldingfoam-env");
        assert_eq!(RELEASE_TAG_FILE, ".kairos-release-tag");
        assert_eq!(
            native_env_bashrc(root),
            Path::new("/home/u/.local/share/kairos/moldingfoam-env/openfoam14/etc/bashrc")
        );
        assert_eq!(
            native_env_tag(root),
            Path::new("/home/u/.local/share/kairos/moldingfoam-env/.kairos-release-tag")
        );
        // 与 VM 内的相对布局同源（ENV_BASHRC 单点维护）
        assert!(native_env_bashrc(root).ends_with(ENV_BASHRC));
    }

    /// 「目标环境里跑一段脚本」的命令行：实例名 / 发行版名 / bash -lc 的形状
    /// 只有这一处产出，脚本原样作为最后一个参数（不做二次转义——调用点已经处理）。
    #[test]
    fn bash_script_args_carry_instance_and_distro_names() {
        assert_eq!(
            bash_script_args(VmProviderKind::Multipass, "cd /tmp && ls"),
            [
                "multipass",
                "exec",
                "kairos",
                "--",
                "bash",
                "-lc",
                "cd /tmp && ls"
            ]
        );
        assert_eq!(
            bash_script_args(VmProviderKind::Wsl, "true"),
            ["wsl", "-d", "Ubuntu-24.04", "bash", "-lc", "true"]
        );
        // 原生环境就在本机：不经任何虚拟机包装。
        assert_eq!(
            bash_script_args(VmProviderKind::Native, "echo hi"),
            ["bash", "-lc", "echo hi"]
        );
        // 实例名只此一处：改成常量而不是字面量，重命名时不会漏
        assert!(
            bash_script_args(VmProviderKind::Multipass, "x").contains(&INSTANCE_NAME.to_string())
        );
    }

    /// 环境路径与部署命令：根目录 / bashrc / 标记同源，归档走绝对路径（传输目标
    /// 与解压来源同一处），部署命令串起「建目录 → 解压 → 校验」。
    #[test]
    fn env_paths_and_deploy_command_share_one_source() {
        assert_eq!(
            env_path(ENV_BASHRC),
            "~/moldingfoam-env/openfoam14/etc/bashrc"
        );
        assert_eq!(env_path(RELEASE_TAG_FILE), vm_env_tag_path());
        assert_eq!(
            vm_bundle_archive_path(),
            "/home/ubuntu/moldingfoam-bundle.tar.xz"
        );
        assert_eq!(
            vm_bundle_transfer_target(),
            "kairos:/home/ubuntu/moldingfoam-bundle.tar.xz"
        );
        let deploy = env_deploy_command();
        assert!(deploy.starts_with("mkdir -p ~/moldingfoam-env && tar -xJf /home/ubuntu/"));
        assert!(
            deploy.ends_with(&env_probe_command()),
            "部署收尾必须复核 bashrc：{deploy}"
        );
        assert!(env_source_command().ends_with(&env_path(ENV_BASHRC)));
    }

    /// 依赖安装与源切换命令：apt 源切国内镜像（best-effort，失败不阻断），
    /// OpenMPI 装完必须复核 libmpi.so.40（缺库时作业会跑到一半才失败）。
    #[test]
    fn dependency_commands_switch_mirror_and_verify_libmpi() {
        let mirror = apt_mirror_command();
        assert!(mirror.contains(CN_MIRROR_HOST));
        assert!(
            mirror.ends_with("|| true"),
            "源切换失败不阻断部署：{mirror}"
        );
        let install = openmpi_install_command();
        assert!(install.contains("libopenmpi-dev openmpi-bin"));
        assert!(install.ends_with("ldconfig -p | grep -q libmpi.so.40"));
        // 镜像主机与云镜像下载同源（同一常量，改一处全生效）
        assert!(image_mirror_urls("aarch64")[0].contains(CN_MIRROR_HOST));
    }

    /// 版本标签归一：两端空白不算版本，空 / 全空白 → None。VM 标记、本机记录、
    /// 原生标记三处读回共用这一处口径。
    #[test]
    fn parse_env_tag_trims_and_rejects_blank() {
        assert_eq!(parse_env_tag("v1.0.0\n"), Some("v1.0.0".to_string()));
        assert_eq!(parse_env_tag("  v1.0.0  "), Some("v1.0.0".to_string()));
        assert_eq!(parse_env_tag(""), None);
        assert_eq!(parse_env_tag(" \n\t "), None);
    }

    /// VM 内版本标记的读写命令：路径与 VM 布局同源（波浪号保持不加引号，
    /// 展开交给 guest 的 shell），读取带哨兵，写入把标记值按单引号字面量转义。
    #[test]
    fn vm_env_tag_commands_carry_sentinel_and_quote_tag() {
        assert_eq!(vm_env_tag_path(), "~/moldingfoam-env/.kairos-release-tag");
        let read = vm_env_tag_read_command();
        assert!(read.contains(ENV_TAG_MARK));
        assert!(read.contains("cat ~/moldingfoam-env/.kairos-release-tag 2>/dev/null"));
        assert!(!read.contains("'~"), "波浪号被引号裹住就不会展开：{read}");
        assert_eq!(
            vm_env_tag_write_command("v0.2.0"),
            "printf '%s' 'v0.2.0' > ~/moldingfoam-env/.kairos-release-tag"
        );
        // 标记值来自下载清单：单引号必须转义，否则命令被提前截断
        assert!(vm_env_tag_write_command("a'b").contains("'a'\\''b'"));
    }

    /// 解析版本标记回读：哨兵之后才是标记值，噪声（拉起实例的转圈动画）与截断
    /// 输出都不算版本；哨兵后为空（标记文件缺失）→ 未部署。
    #[test]
    fn parse_env_tag_reply_ignores_noise_and_missing_marker() {
        let noisy = "Starting kairos  /-\u{8}\\\u{8}\u{1b}[2K\u{1b}[0A\u{1b}[0EKAIROS-TAG:v0.2.0\n";
        assert_eq!(parse_env_tag_reply(noisy), Some("v0.2.0".to_string()));
        assert_eq!(
            parse_env_tag_reply("KAIROS-TAG:\n"),
            None,
            "标记文件缺失 → 值空 → 未部署"
        );
        assert_eq!(
            parse_env_tag_reply("Starting kairos\n"),
            None,
            "哨兵缺失（输出截断）→ 未部署，不把噪声当版本"
        );
    }

    /// 已部署版本视图：VM 可达时以 VM 内标记为准（含「标记没了」= 未部署），
    /// 不可达时才回落本机记录——停机不该把已部署退回未部署。
    #[test]
    fn resolve_deployed_tag_falls_back_to_record_only_when_vm_unreachable() {
        let vm = || Some("v1.0.0".to_string());
        let record = || Some("v0.9.0".to_string());
        assert_eq!(resolve_deployed_tag(true, vm(), record()), vm());
        assert_eq!(
            resolve_deployed_tag(true, None, record()),
            None,
            "VM 可达但标记缺失 = 环境树没了，不能拿旧记录充数"
        );
        assert_eq!(
            resolve_deployed_tag(false, None, record()),
            record(),
            "VM 停机时回落到上次部署记录"
        );
        assert_eq!(resolve_deployed_tag(false, None, None), None);
    }

    /// 路径含空格 / 单引号时仍能安全拼进 bash：全部走单引号字面量转义。
    /// 断言用「按平台拼出来的 bashrc 路径」而不是硬编码 `/`——Windows 上 `Path::join`
    /// 产出 `\`，写死 `/` 的期望值会红。
    #[test]
    fn native_env_commands_quote_paths() {
        let plain = Path::new("/opt/kairos env");
        let bashrc = native_env_bashrc(plain).to_string_lossy().to_string();
        assert_eq!(
            native_env_source_command(plain),
            format!("source '{bashrc}'")
        );
        assert_eq!(
            native_env_probe_command(plain),
            format!("test -f '{bashrc}'")
        );
        // 命令整体仍是单引号字面量（未有裸露的空格分隔符）
        assert!(native_env_source_command(plain).starts_with("source '"));
        assert!(native_env_source_command(plain).ends_with('\''));

        // 单引号路径：整条命令里唯一的转义形态是 '\''，且 bashrc 中的引号被转义
        let quoted = Path::new("/opt/it's here");
        let command = native_env_source_command(quoted);
        assert_eq!(command.matches("'\\''").count(), 1);
        assert!(!command.contains("it's"));

        // 转义函数本身（平台无关）
        assert_eq!(bash_single_quote("a'b"), "a'\\''b");
        assert_eq!(bash_single_quote("plain"), "plain");
    }

    /// 依赖提示按状态组合给出，且互不重复。
    #[test]
    fn native_dependency_hints_follow_state() {
        assert!(native_dependency_hints(true, true).is_empty());

        let env_only = native_dependency_hints(true, false);
        assert_eq!(env_only.len(), 1);
        assert!(env_only[0].contains("依赖面板下载"));

        let mpi_only = native_dependency_hints(false, true);
        assert_eq!(mpi_only.len(), 1);
        assert!(mpi_only[0].contains("libmpi.so.40"));
        assert!(mpi_only[0].contains("apt install libopenmpi3 openmpi-bin"));

        let both = native_dependency_hints(false, false);
        assert_eq!(both.len(), 2);
    }

    #[test]
    fn provider_maps_by_os() {
        assert_eq!(provider_for("macos"), Some(VmProviderKind::Multipass));
        assert_eq!(provider_for("windows"), Some(VmProviderKind::Wsl));
        assert_eq!(provider_for("linux"), Some(VmProviderKind::Native));
        assert_eq!(provider_for("freebsd"), None);
    }

    #[test]
    fn native_provider_needs_no_vm() {
        let status = build_status(VmProviderKind::Native, false, VmState::Missing);
        assert!(status.tool_installed);
        assert_eq!(status.instance_state, VmState::Running);
        assert_eq!(status.instance_name, "localhost");
        assert!(status.hint.contains("无需虚拟机"));
        // Shell 就是本地登录 bash；安装/启动/停止全部为无操作短路。
        assert_eq!(shell_args(VmProviderKind::Native), vec!["bash", "--login"]);
        assert!(start_args(VmProviderKind::Native).is_none());
        assert!(install_args(VmProviderKind::Native).is_none());
        for args in [
            version_args(VmProviderKind::Native),
            instance_info_args(VmProviderKind::Native),
            launch_args(VmProviderKind::Native, &plan_resources(8, 16), None),
            stop_args(VmProviderKind::Native),
        ] {
            assert_eq!(args, vec!["true"]);
        }
    }

    #[test]
    fn native_provider_hints_stay_native_specific() {
        assert!(install_hint(VmProviderKind::Native).contains("原生"));
        for state in [
            VmState::Missing,
            VmState::Stopped,
            VmState::Starting,
            VmState::Unknown,
        ] {
            assert!(instance_hint(VmProviderKind::Native, state).contains("原生"));
            assert!(instance_hint(VmProviderKind::Multipass, state).contains("虚拟机"));
        }
        assert!(instance_hint(VmProviderKind::Wsl, VmState::Missing).contains("WSL2"));
    }

    #[test]
    fn china_timezone_uses_mirror_image_for_launch() {
        assert!(is_china_timezone("Asia/Shanghai"));
        assert!(is_china_timezone("China Standard Time"));
        assert!(!is_china_timezone("America/New_York"));
        assert!(!is_china_timezone(""));

        assert_eq!(
            image_file_name("aarch64"),
            Some("noble-server-cloudimg-arm64.img")
        );
        assert_eq!(image_file_name("riscv"), None);
        let urls = image_mirror_urls("x86_64");
        assert_eq!(urls.len(), 1);
        assert!(urls[0].starts_with(
            "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-cloud-images/noble/current/"
        ));
        assert!(urls[0].ends_with("noble-server-cloudimg-amd64.img"));

        // file:// 导入：本地镜像替换 24.04 位置参数
        let launch = launch_args(
            VmProviderKind::Multipass,
            &plan_resources(8, 16),
            Some("/tmp/noble.img"),
        );
        assert!(launch.contains(&"file:///tmp/noble.img".to_string()));
        assert!(!launch.contains(&"24.04".to_string()));
        let direct = launch_args(VmProviderKind::Multipass, &plan_resources(8, 16), None);
        assert_eq!(direct.last().unwrap(), "24.04");
    }

    #[test]
    fn resources_plan_scales_with_host_and_clamps() {
        // 大机器：封顶 8 核 / 16G（moldingFoam 的推荐规格），磁盘恒 80G。
        assert_eq!(
            plan_resources(16, 64),
            VmResources {
                cpus: 8,
                memory_gib: 16,
                disk_gib: 80
            }
        );
        // 中等机器：宿主的一半。
        assert_eq!(plan_resources(12, 32).cpus, 6);
        assert_eq!(plan_resources(12, 32).memory_gib, 16);
        // 小机器：保底 2 核 / 4G，宿主与虚拟机各占一半。
        assert_eq!(plan_resources(4, 8).cpus, 2);
        assert_eq!(plan_resources(4, 8).memory_gib, 4);
    }

    #[test]
    fn args_builders_cover_both_providers() {
        let resources = plan_resources(8, 16);
        for (provider, bin) in [
            (VmProviderKind::Multipass, "multipass"),
            (VmProviderKind::Wsl, "wsl"),
        ] {
            assert_eq!(version_args(provider)[0], bin);
            assert_eq!(instance_info_args(provider)[0], bin);
            assert_eq!(launch_args(provider, &resources, None)[0], bin);
            assert_eq!(shell_args(provider)[0], bin);
            assert_eq!(stop_args(provider)[0], bin);
        }
        // multipass：实例名贯穿 launch/start/shell/stop；WSL：无独立 start 步骤
        let launch = launch_args(VmProviderKind::Multipass, &resources, None);
        assert!(launch.contains(&INSTANCE_NAME.to_string()));
        // 规格来自宿主探测结果：4 核 / 8G / 80G + Ubuntu 24.04（plan_resources(8,16)）
        let cpus_pos = launch.iter().position(|a| a == "--cpus").unwrap();
        assert_eq!(launch[cpus_pos + 1], "4");
        let mem_pos = launch.iter().position(|a| a == "--memory").unwrap();
        assert_eq!(launch[mem_pos + 1], "8G");
        assert_eq!(launch.last().unwrap(), "24.04");
        assert_eq!(
            start_args(VmProviderKind::Multipass).as_deref(),
            Some(
                &[
                    "multipass".to_string(),
                    "start".to_string(),
                    INSTANCE_NAME.to_string()
                ][..]
            )
        );
        assert!(start_args(VmProviderKind::Wsl).is_none());
        let brew_install = install_args(VmProviderKind::Multipass).expect("multipass 有安装步骤");
        assert!(brew_install.contains(&"--cask".to_string()));
        let wsl_install = install_args(VmProviderKind::Wsl).expect("wsl 有安装步骤");
        // 提权安装：装完由系统层弹窗，本进程拿不到输出
        assert!(wsl_install.iter().any(|a| a.contains("RunAs")));
        assert_eq!(shell_args(VmProviderKind::Wsl).last().unwrap(), WSL_DISTRO);
    }

    #[test]
    fn multipass_state_parser_covers_all_lines() {
        // 新版 multipass 打印 Running（首字母大写），旧版全大写
        let info = "Name: kairos\nState: Running\nIPv4: 192.168.64.3\n";
        assert_eq!(parse_multipass_state(info), VmState::Running);
        assert_eq!(parse_multipass_state("State: RUNNING\n"), VmState::Running);
        assert_eq!(parse_multipass_state("State: Stopped\n"), VmState::Stopped);
        assert_eq!(
            parse_multipass_state("State: STARTING\n"),
            VmState::Starting
        );
        assert_eq!(parse_multipass_state("State: DELETED\n"), VmState::Unknown);
        assert_eq!(parse_multipass_state("no state here"), VmState::Unknown);
    }

    #[test]
    fn wsl_list_parser_requires_distro_row() {
        let list =
            "  NAME            STATE           VERSION\n* Ubuntu-24.04    Running         2\n";
        assert_eq!(parse_wsl_list(list), VmState::Running);
        assert_eq!(
            parse_wsl_list("  Ubuntu-24.04    Stopped         2"),
            VmState::Stopped
        );
        assert_eq!(
            parse_wsl_list("  Ubuntu-24.04    Installing      2"),
            VmState::Starting
        );
        assert_eq!(
            parse_wsl_list("  Debian          Running         2"),
            VmState::Missing
        );
        assert_eq!(parse_wsl_list(""), VmState::Missing);
        assert_eq!(
            parse_wsl_list("* Ubuntu-24.04    Weird           2"),
            VmState::Unknown
        );
    }

    #[test]
    fn wsl_output_decoder_handles_utf16_and_utf8() {
        // `wsl -l -v` 在 Windows 上输出 UTF-16LE：逐字符低位在前
        let mut utf16 = Vec::new();
        for unit in "  Ubuntu-24.04\tRunning\r\n".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_wsl_output(&utf16), "  Ubuntu-24.04\tRunning\r\n");
        assert_eq!(decode_wsl_output(b"plain utf8"), "plain utf8");
        // 非法 UTF-8 落入替换字符而非 panic
        assert!(decode_wsl_output(&[0xff, 0xfe]).contains('\u{fffd}'));
    }

    #[test]
    fn terminal_line_cleaner_strips_ansi_and_collapses_carriage_returns() {
        assert_eq!(
            clean_terminal_line("\u{1b}[2K\u{1b}[0A\u{1b}[0EStarting kairos  / "),
            "Starting kairos  /"
        );
        // \r 折叠：spinner 多帧只保留最后一帧
        assert_eq!(
            clean_terminal_line("frame one\rframe two\rframe three"),
            "frame three"
        );
        // 退格重绘 + 夹 NUL
        assert_eq!(
            clean_terminal_line("Starting kairos  0/0-0\\0|0/0"),
            "Starting kairos  0/0-0\\0|0/0"
        );
        assert_eq!(clean_terminal_line("a\u{8}b\u{8}c"), "c");
        assert_eq!(clean_terminal_line("plain\u{0}line"), "plainline");
        assert_eq!(clean_terminal_line("plain line"), "plain line");
        // 纯控制序列清洗后为空
        assert_eq!(clean_terminal_line("\u{1b}[?25l"), "");
    }

    #[test]
    fn status_hint_branches_cover_every_combination() {
        for provider in [VmProviderKind::Multipass, VmProviderKind::Wsl] {
            let missing_tool = build_status(provider, false, VmState::Missing);
            assert!(!missing_tool.tool_installed);
            assert!(missing_tool.hint.contains("安装虚拟机"));
            assert!(!missing_tool.instance_name.is_empty());

            let no_instance = build_status(provider, true, VmState::Missing);
            assert!(no_instance.hint.contains("启动虚拟机"));
            assert!(
                build_status(provider, true, VmState::Stopped)
                    .hint
                    .contains("启动")
            );
            assert!(
                build_status(provider, true, VmState::Starting)
                    .hint
                    .contains("启动中")
            );
            assert!(
                build_status(provider, true, VmState::Running)
                    .hint
                    .contains("Shell")
            );
            assert!(
                build_status(provider, true, VmState::Unknown)
                    .hint
                    .contains("重新探测")
            );
        }
    }
    #[test]
    fn image_mirror_urls_skip_unsupported_arch() {
        assert!(image_mirror_urls("riscv64").is_empty());
    }

    #[test]
    fn terminal_line_cleaner_skips_non_csi_escape() {
        // ESC 后跟非 '[' 的单字符转义：连同转义符一起丢弃
        assert_eq!(clean_terminal_line("a\u{1b}Xb"), "ab");
    }

    #[test]
    fn vm_case_dir_keeps_leaf_name() {
        assert_eq!(
            vm_case_dir("/Users/me/Library/Application Support/com.yuki.kairos/cases/study-1"),
            "/home/ubuntu/study-1"
        );
        // 尾斜杠与空路径都归一到同一形态（回退名 "case"）
        assert_eq!(vm_case_dir("/tmp/case/"), "/home/ubuntu/case");
        assert_eq!(vm_case_dir(""), "/home/ubuntu/case");
    }

    #[test]
    fn vm_case_extract_targets_the_case_root_not_the_case() {
        // 归档条目自带叶子名前缀（宿主 tar -C <父> -czf <归档> <叶子>）：
        // 解压目标必须是 case 根，解到 case 目录会多套一层同名目录。
        assert_eq!(
            vm_case_extract_from_archive_command("/home/ubuntu/study-1"),
            "rm -rf '/home/ubuntu/study-1' && tar -xzf '/home/ubuntu/.kairos-transfer.tgz' -C /home/ubuntu && rm -f '/home/ubuntu/.kairos-transfer.tgz'"
        );
        // 单引号路径按 shell 字面量转义
        assert_eq!(
            vm_case_extract_from_archive_command("/home/ubuntu/it's"),
            "rm -rf '/home/ubuntu/it'\\''s' && tar -xzf '/home/ubuntu/.kairos-transfer.tgz' -C /home/ubuntu && rm -f '/home/ubuntu/.kairos-transfer.tgz'"
        );
    }

    #[test]
    fn results_pack_lists_time_dirs_under_the_case() {
        // 打包在 VM 内的 case 目录下执行，时间目录名原样列出（宿主侧同一套判定）
        assert_eq!(
            vm_results_pack_command(
                "/home/ubuntu/study-1",
                &["0.5".to_string(), "1".to_string(), "1.5".to_string()]
            ),
            "cd '/home/ubuntu/study-1' && tar -czf '/home/ubuntu/.kairos-transfer.tgz' 0.5 1 1.5"
        );
        // 回传前的清单用同一处转义：case 路径含单引号时不能把命令截断
        assert_eq!(
            vm_results_list_command("/home/ubuntu/study-1"),
            "cd '/home/ubuntu/study-1' && ls -d [0-9]* 2>/dev/null"
        );
        assert_eq!(
            vm_results_list_command("/home/ubuntu/o'brien"),
            "cd '/home/ubuntu/o'\\''brien' && ls -d [0-9]* 2>/dev/null"
        );
    }

    #[test]
    fn transfer_moves_files_by_path_not_by_pipe() {
        assert_eq!(
            transfer_args("/tmp/case.tgz", "kairos:/home/ubuntu/.kairos-transfer.tgz"),
            vec![
                "multipass",
                "transfer",
                "/tmp/case.tgz",
                "kairos:/home/ubuntu/.kairos-transfer.tgz"
            ]
        );
        assert_eq!(vm_archive_path(), "/home/ubuntu/.kairos-transfer.tgz");
        // 端点形态只此一处：case 中转与 bundle 走同一个构造
        assert_eq!(
            vm_transfer_endpoint(&vm_archive_path()),
            "kairos:/home/ubuntu/.kairos-transfer.tgz"
        );
    }

    /// 求解脚本：先记会话 id（取消要靠它），再 source 环境（它决定 OpenFOAM 的
    /// PATH / LD_LIBRARY_PATH），随后进 VM 内 case 目录，最后是求解命令。
    #[test]
    fn solve_script_records_sid_then_sources_env_and_enters_case() {
        let script = solve_script("/home/ubuntu/study-1", 4);
        assert!(
            script.starts_with("echo $$ > '/home/ubuntu/study-1/.kairos-solve.sid'"),
            "首段必须记会话 id：{script}"
        );
        assert!(script.contains(&format!("; {} && cd ", env_source_command())));
        assert!(script.contains("cd '/home/ubuntu/study-1'"));
        assert!(script.ends_with(&crate::services::moldingfoam::solve_command(4)));
        // case 路径含单引号时不能把命令截断
        assert!(solve_script("/home/ubuntu/o'brien", 4).contains("'/home/ubuntu/o'\\''brien'"));
    }

    /// 取消命令：按会话 id 杀**进程组**，求解已结束或 id 缺失时是空操作（`|| true`）——
    /// 取消是用户意图，不该因为「刚好跑完」而报错。
    #[test]
    fn solver_stop_kills_the_session_group_and_tolerates_missing_sid() {
        assert_eq!(
            solver_stop_command("/home/ubuntu/study-1"),
            "kill -TERM -$(cat '/home/ubuntu/study-1/.kairos-solve.sid' 2>/dev/null) 2>/dev/null || true"
        );
        assert_eq!(
            solve_sid_path("/home/ubuntu/study-1"),
            "/home/ubuntu/study-1/.kairos-solve.sid"
        );
        assert!(
            solver_stop_command("/home/ubuntu/a'b")
                .contains("'/home/ubuntu/a'\\''b/.kairos-solve.sid'")
        );
    }

    #[test]
    fn detached_launch_uses_setsid_and_case_side_markers() {
        let command = detached_launch_command(
            "/home/ubuntu/study-1",
            "cd '/home/ubuntu/study-1' && foamRun",
        );
        // 求解必须在独立会话里跑（脱离 ssh 会话），日志与退出码写在 case 内
        assert!(command.contains("setsid bash -lc "));
        assert!(command.contains("'/home/ubuntu/study-1'/kairos-solve.log"));
        assert!(command.contains("'/home/ubuntu/study-1'/.kairos-solve.exit"));
        assert!(command.contains("echo $? > "));
        // 脚本整体落在单引号里、内部引号按 '\'' 转义（少外层引号就是引号失衡）
        assert!(command.contains("bash -lc 'cd '\\''/home/ubuntu/study-1'\\'' && foamRun'"));
        // 启动命令自身后台化：客户端拿到返回即结束，不等求解
        assert!(command.ends_with(") &"));
    }

    /// 生成的 shell 命令必须能被 shell 解析：引号失衡这类问题只会表现为命令报
    /// 「unexpected EOF while looking for matching `''」，而「字符串包含某个片段」的
    /// 断言抓不住（`detached_launch_command` 少写外层引号的写法就这样被锁死过一次）。
    /// 含单引号的路径一并覆盖——转义口径对不对，只有交给 shell 解析才算验证过。
    #[test]
    #[cfg(unix)]
    fn generated_shell_commands_parse_with_bash() {
        let quoted_case = "/home/ubuntu/o'brien";
        let commands = [
            solve_script("/home/ubuntu/study-1", 4),
            solve_script(quoted_case, 2),
            solver_stop_command("/home/ubuntu/study-1"),
            solver_stop_command(quoted_case),
            detached_launch_command(
                "/home/ubuntu/study-1",
                &solve_script("/home/ubuntu/study-1", 4),
            ),
            detached_launch_command(quoted_case, &solve_script(quoted_case, 2)),
            detached_read_command("/home/ubuntu/study-1", 0),
            vm_case_extract_from_archive_command("/home/ubuntu/study-1"),
            vm_case_extract_from_archive_command(quoted_case),
            vm_results_pack_command(
                "/home/ubuntu/study-1",
                &["0.5".to_string(), "1".to_string()],
            ),
            vm_results_list_command(quoted_case),
            vm_env_tag_read_command(),
            vm_env_tag_write_command("v1.0.0"),
            vm_env_tag_write_command("a'b"),
            env_deploy_command(),
            apt_mirror_command(),
            openmpi_install_command(),
        ];
        for command in commands {
            let parsed = std::process::Command::new("bash")
                .args(["-n", "-c", &command])
                .output()
                .expect("bash 不可用");
            // 诊断信息先绑定再断言：assert 的消息参数只在失败时求值，直接写进去
            // 会变成「永远未覆盖」的假落点（覆盖率门槛会如实报出来）。
            let stderr = String::from_utf8_lossy(&parsed.stderr).into_owned();
            assert!(
                parsed.status.success(),
                "生成的命令无法被 shell 解析：\n{command}\n{stderr}"
            );
        }
    }

    #[test]
    fn detached_read_streams_from_offset_and_reports_status() {
        let command = detached_read_command("/home/ubuntu/study-1", 0);
        assert!(command.contains("tail -c +1 "));
        assert!(command.contains(STREAM_MARK));
        assert!(command.contains("|| printf -- '-'"));
        // offset 之后的字节从 offset+1 开始读（tail 的计数是 1 基）
        assert!(detached_read_command("/home/ubuntu/study-1", 4096).contains("tail -c +4097 "));
    }

    #[test]
    fn read_output_splits_log_from_exit_code() {
        // 仍在求解：退出码位是 `-`
        let (chunk, offset, code) = parse_read_output("Time = 0.1\n##KAIROS-STATUS##\n-", 0);
        assert_eq!(chunk, "Time = 0.1\n");
        assert_eq!(offset, 11);
        assert_eq!(code, None);
        // 求解结束：退出码可解析
        let (_, _, code) = parse_read_output("\n##KAIROS-STATUS##\n0", 10);
        assert_eq!(code, Some(0));
        let (_, _, code) = parse_read_output("\n##KAIROS-STATUS##\n137", 10);
        assert_eq!(code, Some(137));
        // 输出被截断（没有哨兵）：不推进偏移、不当成结束
        let (chunk, offset, code) = parse_read_output("Time = 0.2", 10);
        assert!(chunk.is_empty());
        assert_eq!(offset, 10);
        assert_eq!(code, None);
    }

    #[test]
    fn ready_probe_covers_vm_providers_only() {
        // multipass：exec 一条 true（启动返回时 guest 可能还没起完 sshd）
        assert_eq!(
            ready_probe_args(VmProviderKind::Multipass).unwrap(),
            vec!["multipass", "exec", "kairos", "--", "true"]
        );
        // WSL：distro 内执行（首次执行自启）
        assert_eq!(
            ready_probe_args(VmProviderKind::Wsl).unwrap(),
            vec!["wsl", "-d", "Ubuntu-24.04", "--", "true"]
        );
        assert_eq!(ready_probe_args(VmProviderKind::Native), None);
    }

    #[test]
    fn stop_when_idle_only_for_auto_started_leased_vm() {
        // 自动拉起 + 无作业 + 无 Shell → 关闭
        assert!(should_stop_when_idle(true, 0, false));
        // 还有作业（排队 / 运行中）
        assert!(!should_stop_when_idle(true, 1, false));
        // 用户正在用 Shell
        assert!(!should_stop_when_idle(true, 0, true));
        // 不是我们拉起的（用户手动启动 / 原生环境）→ 不动
        assert!(!should_stop_when_idle(false, 0, false));
    }

    #[test]
    fn env_paths_come_from_the_bundle_layout() {
        // 环境树目录名是上游 bundle 产物（资产名同带 openfoam14），Kairos 只消费
        assert_eq!(ENV_ROOT, "~/moldingfoam-env");
        assert_eq!(ENV_BASHRC, "openfoam14/etc/bashrc");
        assert_eq!(
            env_source_command(),
            "source ~/moldingfoam-env/openfoam14/etc/bashrc"
        );
        assert_eq!(
            env_probe_command(),
            "test -f ~/moldingfoam-env/openfoam14/etc/bashrc"
        );
    }
}
