pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::geometry::GeometryStore::default())
        .manage(commands::solver::SolverRuns::default())
        .invoke_handler(tauri::generate_handler![
            commands::system::system_info,
            commands::project::create_project,
            commands::project::save_project_file,
            commands::project::load_project_file,
            commands::project::list_recent_projects,
            commands::material::list_builtin_materials,
            commands::material::list_custom_materials,
            commands::material::import_custom_materials,
            commands::material::upsert_custom_material,
            commands::material::delete_custom_material,
            commands::material::export_materials_to_file,
            commands::geometry::import_stl,
            commands::geometry::generate_volume_mesh,
            commands::mold::check_mold_network,
            commands::solver::probe_openfoam,
            commands::solver::generate_openfoam_case,
            commands::solver::start_openfoam_run,
            commands::solver::cancel_openfoam_run,
            commands::process::check_process,
            commands::geometry::remove_geometry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
