pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::geometry::GeometryStore::default())
        .manage(commands::jobs::JobScheduler::default())
        .setup(|_app| {
            // macOS 原生全屏会强制隐藏标题栏，与「工具栏可见」冲突：macOS 停留在
            // 配置的 maximized；Windows / Linux 无此问题，按需求进入全屏。
            #[cfg(not(target_os = "macos"))]
            {
                if let Some(window) = _app.get_webview_window("main") {
                    let _ = window.set_fullscreen(true);
                }
            }
            Ok(())
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
            commands::results::list_result_times,
            commands::results::load_result_field,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
