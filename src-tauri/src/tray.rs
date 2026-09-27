//! Menu-bar / system-tray presence.
//!
//! This is the app's real home: the settings window is incidental and stays
//! hidden most of the time. The menu doubles as the status display, so it is
//! rebuilt whenever something it shows changes.

use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager};

use crate::scheduler::{self, SchedulerState};
use crate::settings::{Posture, RuleKind};

const TRAY_ID: &str = "main";

pub fn init(app: &App) -> tauri::Result<()> {
    // A purpose-drawn figure rather than the app icon: at 22pt the app icon is
    // an unrecognisable blob, and every Tauri app ships the same default one.
    // Regenerate with `python3 scripts/make_tray_icon.py`.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        // macOS renders a template image as a monochrome glyph that follows the
        // menu bar's light/dark appearance. Ignored on other platforms.
        .icon_as_template(true)
        .menu(&build_menu(app.handle())?)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    eprintln!("habitapp: menu bar icon created");

    Ok(())
}

/// Rebuild the menu so the countdowns and pause state stay honest.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let status = scheduler::status(app);

    let posture_label = match status.posture {
        Posture::Sitting => "Currently sitting",
        Posture::Standing => "Currently standing",
    };
    // Disabled entries: status, not actions.
    let posture = MenuItem::with_id(app, "noop_posture", posture_label, false, None::<&str>)?;

    let next_label = if status.paused {
        "Reminders paused".to_string()
    } else if !status.within_working_hours {
        "Outside working hours".to_string()
    } else {
        format!(
            "Next: exercise {}, posture {}",
            countdown(status.next_exercise_seconds),
            countdown(status.next_posture_seconds),
        )
    };
    let next = MenuItem::with_id(app, "noop_next", next_label, false, None::<&str>)?;

    let exercise_now = MenuItem::with_id(app, "exercise_now", "Exercise now", true, None::<&str>)?;
    let posture_now = MenuItem::with_id(
        app,
        "posture_now",
        "Change position now",
        true,
        None::<&str>,
    )?;

    let pause_30 = MenuItem::with_id(app, "pause_30", "Pause for 30 minutes", true, None::<&str>)?;
    let pause_60 = MenuItem::with_id(app, "pause_60", "Pause for 1 hour", true, None::<&str>)?;
    let pause_day =
        MenuItem::with_id(app, "pause_day", "Pause until tomorrow", true, None::<&str>)?;
    let resume = MenuItem::with_id(
        app,
        "resume",
        "Resume reminders",
        status.paused,
        None::<&str>,
    )?;

    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit HabitHealthApp", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &posture,
            &next,
            &PredefinedMenuItem::separator(app)?,
            &exercise_now,
            &posture_now,
            &PredefinedMenuItem::separator(app)?,
            &pause_30,
            &pause_60,
            &pause_day,
            &resume,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn countdown(seconds: Option<i64>) -> String {
    match seconds {
        None => "off".to_string(),
        Some(seconds) if seconds < 60 => "under a minute".to_string(),
        Some(seconds) => {
            let minutes = seconds / 60;
            if minutes < 60 {
                format!("{minutes} min")
            } else {
                format!("{}h {:02}m", minutes / 60, minutes % 60)
            }
        }
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "exercise_now" => scheduler::fire(app, RuleKind::Exercise),
        "posture_now" => scheduler::fire(app, RuleKind::Posture),
        "pause_30" => pause_for(app, 30),
        "pause_60" => pause_for(app, 60),
        "pause_day" => pause_until_tomorrow(app),
        "resume" => {
            with_state(app, |state| state.resume());
            refresh(app);
        }
        "settings" => show_settings(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn pause_for(app: &AppHandle, minutes: u64) {
    with_state(app, |state| {
        state.pause_for(Duration::from_secs(minutes * 60))
    });
    refresh(app);
}

/// "Until tomorrow" means until the next local midnight, not 24 hours from now.
fn pause_until_tomorrow(app: &AppHandle) {
    use chrono::{Local, NaiveTime};

    let now = Local::now();
    let midnight = (now + chrono::Duration::days(1))
        .with_time(NaiveTime::MIN)
        .single();

    let seconds = midnight
        .map(|midnight| (midnight - now).num_seconds().max(0) as u64)
        .unwrap_or(12 * 60 * 60);

    with_state(app, |state| state.pause_for(Duration::from_secs(seconds)));
    refresh(app);
}

fn with_state(app: &AppHandle, action: impl FnOnce(&mut SchedulerState)) {
    let state = app.state::<Mutex<SchedulerState>>();
    let mut state = state.lock().expect("scheduler state");
    action(&mut state);
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
