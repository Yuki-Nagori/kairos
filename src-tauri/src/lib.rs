use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::geometry::GeometryStore::default())
        .manage(commands::jobs::JobScheduler::default())
        .manage(commands::vm::VmShellState::default())
        .setup(|_app| {
            // 窗口铺满与全屏统一在启动时处理：配置式的 center/maximized 在 macOS
            // 不扣除 Dock 与菜单栏的可见区域，居中窗口会显得偏左，且 maximized
            // 在 dev 下常不生效。macOS 最大化（原生全屏会隐藏标题栏，故不用）；
            // Windows / Linux 无此问题，直接进入全屏。
            if let Some(window) = _app.get_webview_window("main") {
                #[cfg(not(target_os = "macos"))]
                let _ = window.set_fullscreen(true);
                #[cfg(target_os = "macos")]
                let _ = window.maximize();
            }

            // 原生应用菜单：macOS 在屏幕顶部系统栏，Windows / Linux 在窗口标题栏下方。
            // 菜单项只做「动作 id」的发射，具体行为由前端监听 menu-action 路由。
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
            let edit = SubmenuBuilder::new(_app, "编辑")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .separator()
                .select_all()
                .build()?;

            let view = SubmenuBuilder::new(_app, "视图")
                .item(&action("view.theme", "切换主题", None)?)
                .build()?;
            let analysis = SubmenuBuilder::new(_app, "分析")
                .item(&action("analysis.checkNetwork", "校验模具网络", None)?)
                .build()?;
            let results = SubmenuBuilder::new(_app, "结果")
                .item(&action("results.exportCsv", "导出当前场为 CSV", None)?)
                .build()?;
            let tools = SubmenuBuilder::new(_app, "工具")
                .item(&action("tools.refreshDeps", "探测运行时依赖", None)?)
                .build()?;

            let help = SubmenuBuilder::new(_app, "帮助")
                .about(None::<AboutMetadata>)
                .build()?;

            let menu = MenuBuilder::new(_app)
                .item(&file)
                .item(&edit)
                .item(&view)
                .item(&analysis)
                .item(&results)
                .item(&tools)
                .item(&help)
                .build()?;
            _app.set_menu(menu)?;
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
            commands::mold::check_mold_network,
            commands::process::check_process,
            commands::solver::probe_openfoam,
            commands::gpu::probe_gpu,
            commands::solver::generate_openfoam_case,
            commands::jobs::submit_job,
            commands::jobs::cancel_job,
            commands::jobs::list_jobs,
            commands::dependencies::list_runtime_dependencies,
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
