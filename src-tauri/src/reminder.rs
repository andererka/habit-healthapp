//! The reminder popup window.
//!
//! A notification banner disappears after a few seconds whether or not you saw
//! it, which for a reminder is a silent failure. This window stays up until it
//! is answered, and is the default way reminders are delivered.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::exercises::Media;
use crate::settings::RuleKind;

const WINDOW_LABEL: &str = "reminder";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub kind: RuleKind,
    pub title: String,
    pub body: String,
    /// Drives the countdown ring. `None` means no suggested duration.
    pub duration_seconds: Option<u32>,
    /// The exercise's image or video, if it has one.
    pub media: Option<Media>,
}

/// What the window hands back when you answer it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Done,
    Snooze,
    Skip,
}

/// The reminder waiting to be shown.
///
/// The window cannot simply be sent an event on creation — the webview is not
/// listening yet, and the event would be dropped. So the payload is parked
/// here and the window collects it once it has mounted.
#[derive(Default)]
pub struct Pending(pub Mutex<Option<Reminder>>);

pub fn show(app: &AppHandle, reminder: Reminder) {
    eprintln!(
        "habitapp: showing reminder ({:?}) — {}",
        reminder.kind, reminder.title
    );

    {
        let pending = app.state::<Pending>();
        *pending.0.lock().expect("pending reminder") = Some(reminder.clone());
    }

    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        // Already open — a second reminder can land while the first is still
        // up. Replace its contents rather than stacking windows.
        let _ = app.emit_to(WINDOW_LABEL, "reminder", reminder);
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let built = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
        .title("Time to move")
        .inner_size(380.0, 340.0)
        .resizable(false)
        .always_on_top(true)
        .decorations(false)
        .center()
        .skip_taskbar(true)
        .focused(true)
        .build();

    if let Err(error) = built {
        eprintln!("habitapp: could not open the reminder window: {error}");
    }
}

pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        // Closing outright would mean rebuilding the webview for every
        // reminder; hiding keeps the next one instant.
        let _ = window.hide();
    }
}
