pub mod activities;
pub mod db;
pub mod decoder;
pub mod import;
pub mod recompute;
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

/// The Athlete's Activities with their Activity Summary, newest first.
#[tauri::command(async)]
fn list_activities(
    database: State<'_, db::Database>,
) -> Result<Vec<activities::ListedActivity>, String> {
    let conn = database
        .0
        .lock()
        .map_err(|_| "the database is unavailable".to_owned())?;
    activities::list(&conn, db::DEFAULT_ATHLETE).map_err(|err| err.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&db::path_in(&data_dir))?;
            let library = import::library_in(&data_dir);
            // Before the commands get the database, so the activity list
            // never shows figures computed by older rules.
            log_skipped(recompute::stale_summaries(&conn, &library));
            app.manage(db::Database(Mutex::new(conn)));
            app.manage(Library(library));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![import_paths, list_activities])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Logs the summaries the recompute left with their older figures. None of
/// it stops the app from starting.
fn log_skipped(recompute: rusqlite::Result<recompute::RecomputeReport>) {
    match recompute {
        Ok(report) => {
            for skipped in report.skipped {
                eprintln!(
                    "summary of Activity {} not recomputed: {}",
                    skipped.activity_id.0, skipped.reason
                );
            }
        }
        Err(e) => eprintln!("stale summaries not recomputed: {e}"),
    }
}
