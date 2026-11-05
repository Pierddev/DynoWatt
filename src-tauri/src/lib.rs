pub mod db;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&db::path_in(&data_dir))?;
            app.manage(db::Database(Mutex::new(conn)));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
