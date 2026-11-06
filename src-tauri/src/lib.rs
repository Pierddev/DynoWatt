pub mod activities;
pub mod db;
pub mod decoder;
pub mod import;
pub mod library;
pub mod recompute;
pub mod summary;
pub mod telemetry;
pub mod workouts;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::Connection;

use tauri::{AppHandle, Emitter, Manager, State};

/// Where the Library lives, as managed state.
struct Library(PathBuf);

/// Where the recompute of stale summaries started at launch stands, as
/// managed state.
struct Recompute(Mutex<recompute::Status>);

/// The event telling the window the recompute is over, with its status.
const RECOMPUTE_FINISHED: &str = "recompute-finished";

/// Imports the Source Files at `paths`, files or folders, and returns the
/// Import Report. Runs off the main thread, so the window stays responsive
/// through a long run.
#[tauri::command(async)]
fn import_paths(
    paths: Vec<PathBuf>,
    database: State<'_, db::Database>,
    library: State<'_, Library>,
) -> Result<import::ImportReport, String> {
    let conn = connection(&database)?;
    Ok(import::run(&conn, &library.0, &paths, chrono::Utc::now()))
}

/// The Athlete's Activities with their Activity Summary, newest first.
#[tauri::command(async)]
fn list_activities(
    database: State<'_, db::Database>,
) -> Result<Vec<activities::ListedActivity>, String> {
    let conn = connection(&database)?;
    activities::list(&conn, db::DEFAULT_ATHLETE).map_err(|err| err.to_string())
}

/// The Samples of the Activity `id`, decoded again from its Source File, for
/// the detail charts.
#[tauri::command(async)]
fn activity_telemetry(
    id: i64,
    database: State<'_, db::Database>,
    library: State<'_, Library>,
) -> Result<telemetry::Telemetry, String> {
    let conn = connection(&database)?;
    telemetry::of_activity(&conn, &library.0, db::ActivityId(id))
}

/// Deletes the Activity `id` together with its Source File in the Library:
/// both, or neither when one of them fails.
#[tauri::command(async)]
fn delete_activity(
    id: i64,
    database: State<'_, db::Database>,
    library: State<'_, Library>,
) -> Result<(), String> {
    let mut conn = connection(&database)?;
    library::delete_activity(&mut conn, &library.0, db::ActivityId(id))
}

/// Where the recompute of stale summaries started at launch stands.
#[tauri::command]
fn recompute_status(recompute: State<'_, Recompute>) -> recompute::Status {
    recompute
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// The Athlete's Workouts with their total duration, in the order they were
/// created.
#[tauri::command(async)]
fn list_workouts(
    database: State<'_, db::Database>,
) -> Result<Vec<workouts::ListedWorkout>, String> {
    let conn = connection(&database)?;
    workouts::list(&conn, db::DEFAULT_ATHLETE).map_err(|err| err.to_string())
}

/// The Workout `id`, for the editor.
#[tauri::command(async)]
fn get_workout(id: i64, database: State<'_, db::Database>) -> Result<workouts::Workout, String> {
    workouts::get(&*connection(&database)?, db::WorkoutId(id))
}

/// Every rule `workout` breaks, for the editor to show on the Steps at fault.
#[tauri::command]
fn validate_workout(workout: workouts::Workout) -> Vec<workouts::Problem> {
    workouts::validate(&workout)
}

/// Saves `workout` as a new Workout, unless it breaks a rule, and returns its
/// id.
#[tauri::command(async)]
fn create_workout(
    workout: workouts::Workout,
    database: State<'_, db::Database>,
) -> Result<db::WorkoutId, String> {
    workouts::create(&mut *connection(&database)?, db::DEFAULT_ATHLETE, workout)
}

/// Replaces the Workout `id` with `workout`, unless it breaks a rule.
#[tauri::command(async)]
fn update_workout(
    id: i64,
    workout: workouts::Workout,
    database: State<'_, db::Database>,
) -> Result<(), String> {
    workouts::update(&mut *connection(&database)?, db::WorkoutId(id), workout)
}

/// Saves a copy of the Workout `id` named `name`, and returns the copy's id.
#[tauri::command(async)]
fn duplicate_workout(
    id: i64,
    name: String,
    database: State<'_, db::Database>,
) -> Result<db::WorkoutId, String> {
    workouts::duplicate(
        &mut *connection(&database)?,
        db::DEFAULT_ATHLETE,
        db::WorkoutId(id),
        name,
    )
}

#[tauri::command(async)]
fn delete_workout(id: i64, database: State<'_, db::Database>) -> Result<(), String> {
    workouts::delete(&*connection(&database)?, db::WorkoutId(id))
}

/// The connection to the database, once no other command holds it.
fn connection(database: &db::Database) -> Result<MutexGuard<'_, Connection>, String> {
    database
        .0
        .lock()
        .map_err(|_| "the database is unavailable".to_owned())
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
            app.manage(Recompute(Mutex::new(recompute::Status::Running)));
            recompute_in_background(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_paths,
            list_activities,
            activity_telemetry,
            delete_activity,
            recompute_status,
            list_workouts,
            get_workout,
            validate_workout,
            create_workout,
            update_workout,
            duplicate_workout,
            delete_workout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Brings the stale summaries up to date off the main thread, one Activity at
/// a time, so the window and the commands stay usable meanwhile. The window
/// learns how it ended from `recompute_status` or the `recompute-finished`
/// event.
fn recompute_in_background(app: AppHandle) {
    std::thread::spawn(move || {
        let status = recompute::run(&app.state::<db::Database>().0, &app.state::<Library>().0);
        // Recorded before the event: a window that missed it still finds it.
        *app.state::<Recompute>()
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = status.clone();
        if let Err(e) = app.emit(RECOMPUTE_FINISHED, status) {
            eprintln!("cannot tell the window the recompute is over: {e}");
        }
    });
}
