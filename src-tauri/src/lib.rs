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
        .manage(commands::results::ResultSession::default())
        .setup(|_app| {
            // 原生（Linux）求解环境：已解压的 bundle 直接 source 其 bashrc。
            // 需要 AppHandle 才能定位应用数据目录，因此在 setup 里注入。
            commands::jobs::refresh_native_env(_app.handle());
            // 启动即最大化：配置式的 center/maximized 在 macOS 不扣除 Dock 与
            // 菜单栏可见区域，居中窗口会偏左。原生全屏会藏掉标题栏，故不用。
            if let Some(window) = _app.get_webview_window("main") {
                let _ = window.maximize();
                // macOS 不经插件激活（激活会抹掉 Overlay 标题栏），配置直接生效；
                // Windows / Linux 保持隐藏，等前端就绪后由插件激活并显示。
                #[cfg(target_os = "macos")]
                let _ = window.show();
            }

            // 原生菜单只做 macOS：构建与菜单树在 commands/menu.rs。
            // Windows / Linux 是无边框窗口，原生菜单不渲染，改用标题栏里的
            // web 菜单（MenuBar.vue）。
            #[cfg(target_os = "macos")]
            commands::menu::install(_app)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit("menu-action", event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::system_info,
            commands::project::create_project,
            commands::project::reset_project_session,
            commands::project::save_project_file,
            commands::project::load_project_file,
            commands::project::list_recent_projects,
            commands::project::default_case_dir,
            commands::project::default_workspace_path,
            commands::project::project_path,
            commands::project::workspace_root_of,
            commands::project::archive_workspace_geometry,
            commands::project::save_report_to_workspace,
            commands::project::save_report_pptx_to_workspace,
            commands::material::list_builtin_materials,
            commands::material::list_custom_materials,
            commands::material::import_custom_materials,
            commands::material::upsert_custom_material,
            commands::material::delete_custom_material,
            commands::material::export_materials_to_file,
            commands::geometry::remove_geometry,
            commands::geometry::import_geometry,
            commands::geometry::import_sample_box,
            commands::geometry::repair_geometry,
            commands::geometry::get_render_mesh,
            commands::geometry::generate_volume_mesh,
            commands::geometry::generate_dual_domain_mesh,
            commands::geometry::generate_midplane_mesh,
            commands::geometry::generate_gmsh_mesh,
            commands::geometry::estimate_volume_mesh,
            commands::geometry::analyze_gate_location,
            commands::geometry::preview_fill,
            commands::geometry::load_workspace_geometry,
            commands::geometry::save_study_mesh,
            commands::geometry::restore_study_mesh,
            commands::mold::check_mold_network,
            commands::process::check_process,
            commands::solver::probe_moldingfoam,
            commands::gpu::probe_gpu,
            commands::solver::generate_moldingfoam_case,
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
            commands::results::sample_probe_series,
            commands::results::load_result_field,
            commands::results::load_result_field_binary,
            commands::results::export_result_field_csv,
            commands::results::summarize_result_field,
            commands::results::summarize_vector_field,
            commands::results::summarize_tensor_field,
            commands::results::load_vector_field_binary,
            commands::results::deform_render_mesh,
            commands::results::load_tensor_field,
            commands::results::derive_field,
            commands::vm::vm_status,
            commands::vm::vm_install,
            commands::vm::vm_start,
            commands::vm::vm_deploy_bundle,
            commands::vm::vm_deployed_release_tag,
            commands::vm::native_env_status,
            commands::vm::native_deploy_bundle,
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
