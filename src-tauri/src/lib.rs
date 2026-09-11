#[cfg(target_os = "macos")]
use tauri::menu::{
    AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
};
use tauri::{Emitter, Manager};

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 窗口装饰按平台运行时管理：Windows/Linux 无边框 + 内嵌控制按钮
        // （Win11 含 Snap Layout 热区），macOS 保留红绿灯；激活失败回退原生标题栏。
        // 必须先于任何 webview 创建注册。
        .plugin(tauri_plugin_decoration::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::geometry::GeometryStore::default())
        .manage(commands::jobs::JobScheduler::default().with_vm_shell(
            // 启动时探测一次 VM 执行通道（macOS→multipass / Windows→wsl）
            commands::jobs::detect_vm_shell(),
        ))
        .manage(commands::vm::VmShellState::default())
        .setup(|_app| {
            // 窗口铺满统一在启动时处理：配置式的 center/maximized 在 macOS
            // 不扣除 Dock 与菜单栏的可见区域，居中窗口会显得偏左，且 maximized
            // 在 dev 下常不生效。三平台都最大化——Windows / Linux 无边框窗口
            // 自绘标题栏（tauri.macos.conf.json 恢复系统边框），原生全屏会把它藏掉。
            if let Some(window) = _app.get_webview_window("main") {
                let _ = window.maximize();
            }

            // 原生菜单仅 macOS：系统栏渲染，动作经 menu-action 事件桥回前端。
            // Windows / Linux 为无边框窗口（自绘标题栏 + web 菜单，见 MenuBar.vue），
            // 原生菜单在无边框窗口不渲染，故不构建。
#[cfg(target_os = "macos")]
            {
                let action = |id: &str,
                              label: &str,
                              accelerator: Option<&str>|
                 -> Result<tauri::menu::MenuItem<tauri::Wry>, tauri::Error> {
                    let mut item = MenuItemBuilder::with_id(id, label);
                    if let Some(accelerator) = accelerator {
                        item = item.accelerator(accelerator);
                    }
                    item.build(_app)
                };

                let version = _app.package_info().version.to_string();
                let credits = "感谢 OpenFOAM (openfoam.org) 提供 CFD 求解基座\n感谢 moldingFoam 项目提供注塑求解模块";
                let about_metadata = |version: &str| AboutMetadata {
                    name: Some("Kairos".into()),
                    version: Some(version.into()),
                    copyright: Some("Copyright © 2026 Yuki".into()),
                    credits: Some(credits.into()),
                    ..Default::default()
                };

                // macOS 规范：首个子菜单是应用菜单（关于 / 服务 / 隐藏 / 退出），
                // 没有它 ⌘Q 退出与「关于」入口都会缺席。
                // 系统预定义项默认英文标题（About / Hide / Quit…），统一给中文标签。
                #[cfg(target_os = "macos")]
                let app_menu = {
                    let about = PredefinedMenuItem::about(
                        _app,
                        Some("关于 Kairos"),
                        Some(about_metadata(&version)),
                    )?;
                    let services = PredefinedMenuItem::services(_app, Some("服务"))?;
                    let hide = PredefinedMenuItem::hide(_app, Some("隐藏 Kairos"))?;
                    let hide_others = PredefinedMenuItem::hide_others(_app, Some("隐藏其他"))?;
                    let show_all = PredefinedMenuItem::show_all(_app, Some("显示全部"))?;
                    let quit = PredefinedMenuItem::quit(_app, Some("退出 Kairos"))?;
                    SubmenuBuilder::new(_app, "Kairos")
                        .item(&about)
                        .separator()
                        .item(&services)
                        .separator()
                        .item(&hide)
                        .item(&hide_others)
                        .item(&show_all)
                        .separator()
                        .item(&quit)
                        .build()?
                };

                let file = SubmenuBuilder::new(_app, "文件")
                    .item(&action("file.new", "新建项目", Some("CmdOrCtrl+N"))?)
                    .item(&action("file.open", "打开项目…", Some("CmdOrCtrl+O"))?)
                    .separator()
                    .item(&action("file.save", "保存", Some("CmdOrCtrl+S"))?)
                    .item(&action(
                        "file.saveAs",
                        "另存为…",
                        Some("Shift+CmdOrCtrl+S"),
                    )?)
                    .build()?;

                // 编辑菜单用系统预定义项：没有它 macOS 的 ⌘C/⌘V 在输入框里不生效。
                let edit = {
                    let undo = PredefinedMenuItem::undo(_app, Some("撤销"))?;
                    let redo = PredefinedMenuItem::redo(_app, Some("重做"))?;
                    let cut = PredefinedMenuItem::cut(_app, Some("剪切"))?;
                    let copy = PredefinedMenuItem::copy(_app, Some("拷贝"))?;
                    let paste = PredefinedMenuItem::paste(_app, Some("粘贴"))?;
                    let select_all = PredefinedMenuItem::select_all(_app, Some("全选"))?;
                    SubmenuBuilder::new(_app, "编辑")
                        .item(&undo)
                        .item(&redo)
                        .separator()
                        .item(&cut)
                        .item(&copy)
                        .item(&paste)
                        .separator()
                        .item(&select_all)
                        .build()?
                };

                let view = SubmenuBuilder::new(_app, "视图")
                    .item(&action("view.theme", "切换主题", None)?)
                    .build()?;

                // 工具聚合校验与依赖 / 虚拟机管理（设计稿菜单结构：文件 编辑 视图 工具 结果 报告 帮助）
                let tools = SubmenuBuilder::new(_app, "工具")
                    .item(&action("analysis.checkNetwork", "校验模具网络", None)?)
                    .item(&action("tools.refreshDeps", "探测运行时依赖", None)?)
                    .separator()
                    .item(&action("tools.vmPanel", "虚拟机面板", None)?)
                    .item(&action("tools.vmStart", "启动虚拟机", None)?)
                    .item(&action("tools.vmShell", "进入虚拟机 Shell", None)?)
                    .item(&action("tools.vmStop", "关闭虚拟机", None)?)
                    .build()?;

                let results = SubmenuBuilder::new(_app, "结果")
                    .item(&action("results.exportCsv", "导出当前场为 CSV", None)?)
                    .build()?;

                let report = SubmenuBuilder::new(_app, "报告")
                    .item(&action("report.open", "打开报告工作台", None)?)
                    .build()?;

                // macOS 的「关于」在应用菜单里，帮助菜单只剩空壳就不保留；
                // Windows / Linux 没有应用菜单，关于入口放帮助菜单。
                #[cfg(not(target_os = "macos"))]
                let help = {
                    let about = PredefinedMenuItem::about(
                        _app,
                        Some("关于 Kairos"),
                        Some(about_metadata(&version)),
                    )?;
                    SubmenuBuilder::new(_app, "帮助").item(&about).build()?
                };

                let menu = {
                    let builder = MenuBuilder::new(_app);
                    #[cfg(target_os = "macos")]
                    let builder = builder.item(&app_menu);
                    let builder = builder
                        .item(&file)
                        .item(&edit)
                        .item(&view)
                        .item(&tools)
                        .item(&results)
                        .item(&report);
                    #[cfg(not(target_os = "macos"))]
                    let builder = builder.item(&help);
                    builder.build()?
                };
                _app.set_menu(menu)?;
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit("menu-action", event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::system_info,
            commands::project::create_project,
            commands::project::save_project_file,
            commands::project::load_project_file,
            commands::project::list_recent_projects,
            commands::project::default_case_dir,
            commands::material::list_builtin_materials,
            commands::material::list_custom_materials,
            commands::material::import_custom_materials,
            commands::material::upsert_custom_material,
            commands::material::delete_custom_material,
            commands::material::export_materials_to_file,
            commands::geometry::import_stl,
            commands::geometry::remove_geometry,
            commands::geometry::import_sample_box,
            commands::geometry::get_render_mesh,
            commands::geometry::generate_volume_mesh,
            commands::geometry::generate_gmsh_mesh,
            commands::mold::check_mold_network,
            commands::process::check_process,
            commands::solver::probe_openfoam,
            commands::gpu::probe_gpu,
            commands::solver::generate_openfoam_case,
            commands::jobs::submit_job,
            commands::jobs::cancel_job,
            commands::jobs::list_jobs,
            commands::dependencies::list_runtime_dependencies,
            commands::dependencies::check_dependency_update,
            commands::dependencies::open_dependency_page,
            commands::downloads::download_file,
            commands::downloads::get_downloads_dir,
            commands::downloads::open_downloads_dir,
            commands::downloads::list_downloads,
            commands::results::list_result_times,
            commands::results::load_result_field,
            commands::vm::vm_status,
            commands::vm::vm_install,
            commands::vm::vm_start,
            commands::vm::vm_deploy_bundle,
            commands::vm::vm_shell_start,
            commands::vm::vm_shell_send,
            commands::vm::vm_shell_stop,
            commands::vm::vm_stop,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            // 应用退出（窗口关闭 / Cmd+Q）：杀 Shell 子进程并派发虚拟机关机。
            if let tauri::RunEvent::Exit = event {
                commands::vm::cleanup_on_exit(&_app.state::<commands::vm::VmShellState>());
            }
        });
}
