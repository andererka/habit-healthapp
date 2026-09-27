//! Settings: the data model, its defaults, and persistence.
//!
//! Stored as one JSON file in the app data directory so it can be inspected,
//! hand-edited, or copied between machines.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

const STORE_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "settings";

/// Which reminder a rule drives. Exercise and posture are deliberately separate
/// rules so they can run on different rhythms — hourly exercises but a posture
/// change every half hour, say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    Exercise,
    Posture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderRule {
    pub kind: RuleKind,
    pub enabled: bool,
    pub interval_minutes: u32,
}

/// How a reminder asks for your attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PopupStyle {
    /// A system notification. Gentle, but a banner auto-dismisses after a few
    /// seconds — easy to miss entirely, which defeats the point.
    Notification,
    /// An always-on-top window that stays until you answer it.
    Window,
}

impl Default for PopupStyle {
    fn default() -> Self {
        Self::Window
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Posture {
    Sitting,
    Standing,
}

impl Posture {
    pub fn toggled(self) -> Self {
        match self {
            Posture::Sitting => Posture::Standing,
            Posture::Standing => Posture::Sitting,
        }
    }

    /// What the notification asks you to do next.
    pub fn prompt(self) -> &'static str {
        match self {
            Posture::Sitting => "You have been sitting a while — stand up.",
            Posture::Standing => "You have been standing a while — sit down.",
        }
    }
}

/// When reminders are allowed to fire.
///
/// Without this the app nags through evenings and weekends, which is the
/// fastest way to get uninstalled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingHours {
    pub enabled: bool,
    /// "HH:MM", local time.
    pub start: String,
    pub end: String,
    /// Weekdays the reminders run on, Monday = 1 … Sunday = 7 (ISO).
    pub days: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub rules: Vec<ReminderRule>,
    pub working_hours: WorkingHours,
    pub posture: Posture,
    pub snooze_minutes: u32,
    /// `serde(default)` so settings files written before M4 still load.
    #[serde(default)]
    pub popup_style: PopupStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rules: vec![
                ReminderRule {
                    kind: RuleKind::Exercise,
                    enabled: true,
                    interval_minutes: 60,
                },
                ReminderRule {
                    kind: RuleKind::Posture,
                    enabled: true,
                    interval_minutes: 30,
                },
            ],
            working_hours: WorkingHours {
                enabled: true,
                start: "09:00".to_string(),
                end: "17:00".to_string(),
                days: vec![1, 2, 3, 4, 5],
            },
            posture: Posture::Sitting,
            snooze_minutes: 10,
            popup_style: PopupStyle::Window,
        }
    }
}

impl Settings {
    pub fn rule(&self, kind: RuleKind) -> Option<&ReminderRule> {
        self.rules.iter().find(|rule| rule.kind == kind)
    }
}

fn store(app: &AppHandle) -> Result<Arc<Store<Wry>>, String> {
    app.store(STORE_FILE).map_err(|error| error.to_string())
}

/// Read settings, falling back to defaults. A malformed or partial file is
/// treated as absent rather than fatal: losing your interval preference is a
/// nuisance, refusing to start is a bug report.
pub fn load(app: &AppHandle) -> Settings {
    let Ok(store) = store(app) else {
        return Settings::default();
    };

    store
        .get(SETTINGS_KEY)
        .and_then(|value| match serde_json::from_value(value) {
            Ok(settings) => Some(settings),
            Err(error) => {
                eprintln!("HabitHealthApp: ignoring unreadable settings ({error}), using defaults");
                None
            }
        })
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let store = store(app)?;
    let value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    store.set(SETTINGS_KEY, value);
    store.save().map_err(|error| error.to_string())
}
