pub mod db;
pub mod decoder;
pub mod import;
pub mod summary;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{Manager, State};

/// Where the Library lives, as managed state.
struct Library(PathBuf);

/// Imports the Source Files at `paths`, files or folders, and returns the
/// Import Report. Runs off the main thread, so the window stays responsive
/// through a long run.
#[tauri::command(async)]
fn import_paths(
    paths: Vec<PathBuf>,
    database: State<'_, db::Database>,
    library: State<'_, Library>,
) -> Result<import::ImportReport, String> {
    let conn = database
        .0
        .lock()
        .map_err(|_| "the database is unavailable".to_owned())?;
    Ok(import::run(&conn, &library.0, &paths, chrono::Utc::now()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&db::path_in(&data_dir))?;
            app.manage(db::Database(Mutex::new(conn)));
            app.manage(Library(import::library_in(&data_dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![import_paths])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
