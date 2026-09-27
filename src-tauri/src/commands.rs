//! Tauri commands — the settings window's entire interface to the scheduler.

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::scheduler::{self, SchedulerState, Status};
use crate::settings::{self, RuleKind, Settings};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    let state = app.state::<Mutex<SchedulerState>>();
    let state = state.lock().expect("scheduler state");
    state.settings.clone()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");
        state.settings = settings.clone();
        // Apply edited intervals now rather than after the old one elapses.
        state.reschedule_all();
    }

    settings::save(&app, &settings)?;
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn get_status(app: AppHandle) -> Status {
    scheduler::status(&app)
}

#[tauri::command]
pub fn remind_now(app: AppHandle, kind: RuleKind) {
    scheduler::fire(&app, kind);
}

#[tauri::command]
pub fn snooze(app: AppHandle, kind: RuleKind) {
    {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");
        state.snooze(kind);
    }
    crate::tray::refresh(&app);
}

#[tauri::command]
pub fn pause(app: AppHandle, minutes: u64) {
    {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");
        state.pause_for(Duration::from_secs(minutes * 60));
    }
    crate::tray::refresh(&app);
}

#[tauri::command]
pub fn resume(app: AppHandle) {
    {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");
        state.resume();
    }
    crate::tray::refresh(&app);
}

// --- Exercise library -------------------------------------------------------

use crate::exercises::{self, Exercise, ExerciseState};

#[tauri::command]
pub fn get_exercises(app: AppHandle) -> Vec<Exercise> {
    let state = app.state::<Mutex<ExerciseState>>();
    let state = state.lock().expect("exercise state");
    state.library.clone()
}

#[tauri::command]
pub fn set_exercises(app: AppHandle, exercises: Vec<Exercise>) -> Result<(), String> {
    {
        let state = app.state::<Mutex<ExerciseState>>();
        let mut state = state.lock().expect("exercise state");
        state.library = exercises.clone();
    }
    exercises::save(&app, &exercises)
}

/// Write the library to a file the user picked. This plus `import_library` is
/// how exercise sets travel between colleagues — no backend, just a JSON file.
#[tauri::command]
pub fn export_library(app: AppHandle, path: String) -> Result<(), String> {
    let library = get_exercises(app);
    let json = serde_json::to_string_pretty(&library).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_library(app: AppHandle, path: String) -> Result<Vec<Exercise>, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let imported: Vec<Exercise> =
        serde_json::from_str(&contents).map_err(|error| format!("not a valid library: {error}"))?;

    if imported.is_empty() {
        return Err("that file contains no exercises".to_string());
    }

    set_exercises(app, imported.clone())?;
    Ok(imported)
}

// --- Reminder popup ---------------------------------------------------------

use crate::reminder::{self, Answer, Reminder};

/// The reminder window collects its payload here once it has mounted; an event
/// emitted at creation time would arrive before the webview is listening.
#[tauri::command]
pub fn take_pending_reminder(app: AppHandle) -> Option<Reminder> {
    let pending = app.state::<reminder::Pending>();
    let pending = pending.0.lock().expect("pending reminder");
    pending.clone()
}

#[tauri::command]
pub fn answer_reminder(app: AppHandle, kind: RuleKind, answer: Answer) {
    match answer {
        Answer::Done => {
            // Only a real answer flips the posture — skipping should not
            // convince the app you are now standing.
            if kind == RuleKind::Posture {
                scheduler::toggle_posture(&app);
            }
        }
        Answer::Snooze => {
            let state = app.state::<Mutex<SchedulerState>>();
            let mut state = state.lock().expect("scheduler state");
            state.snooze(kind);
        }
        Answer::Skip => {}
    }

    reminder::close(&app);
    crate::tray::refresh(&app);
}

// --- Exercise media ---------------------------------------------------------

use crate::exercises::Media;
use crate::media;

/// Copy a file the user picked into the app's media directory and describe it.
///
/// The exercise itself is not touched here — the caller puts the returned
/// `Media` on the exercise and saves through `set_exercises`, so there is one
/// path that writes the library rather than two.
#[tauri::command]
pub fn attach_media(app: AppHandle, path: String) -> Result<Media, String> {
    media::attach(&app, std::path::Path::new(&path))
}

#[tauri::command]
pub fn remove_media(app: AppHandle, media: Media) -> Result<(), String> {
    media::remove(&app, &media)
}

/// Resolve a media reference to an absolute path the webview can load.
///
/// The frontend cannot work this out itself: a file may live either in the
/// user's media directory or inside the app bundle, and only the Rust side
/// knows where the bundle is.
#[tauri::command]
pub fn media_path(app: AppHandle, src: String) -> Result<String, String> {
    media::resolve(&app, &src).map(|path| path.to_string_lossy().into_owned())
}
