mod commands;
mod exercises;
mod media;
mod notifier;
mod reminder;
mod scheduler;
mod settings;
mod tray;

use std::sync::Mutex;

use exercises::ExerciseState;
use scheduler::SchedulerState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. Launching a second copy — the build in
        // target/, say, or a double-clicked installer — would otherwise give
        // you two tray icons, duplicate reminders, and two processes writing
        // the same settings file. Instead the second copy exits immediately and
        // surfaces the one already running.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            eprintln!("habitapp: already running — focusing the existing instance");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::get_status,
            commands::remind_now,
            commands::snooze,
            commands::pause,
            commands::resume,
            commands::get_exercises,
            commands::set_exercises,
            commands::export_library,
            commands::import_library,
            commands::take_pending_reminder,
            commands::answer_reminder,
            commands::attach_media,
            commands::remove_media,
            commands::media_path,
        ])
        .setup(|app| {
            // macOS: menu-bar app with no Dock icon and no app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let loaded = settings::load(&handle);
            app.manage(Mutex::new(SchedulerState::new(loaded)));
            app.manage(Mutex::new(ExerciseState::new(exercises::load(&handle))));
            app.manage(reminder::Pending::default());

            // A tray failure must not stop the app: the menu bar is the only
            // way in for an Accessory app, so losing it and quitting would
            // leave no way to reach settings at all. Fall back to showing the
            // settings window instead.
            if let Err(error) = tray::init(app) {
                eprintln!("habitapp: could not create the menu bar icon: {error}");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Ask before the first reminder fires, so the prompt is not racing
            // a notification that would then be dropped.
            notifier::request_authorization(&handle);

            scheduler::start(handle);

            Ok(())
        })
        .on_window_event(|window, event| {
            // The app lives in the tray, so closing the settings window hides it
            // rather than quitting. Quit is an explicit tray action.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
