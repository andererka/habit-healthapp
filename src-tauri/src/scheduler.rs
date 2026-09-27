//! Reminder scheduling.
//!
//! The timer lives in Rust rather than in the webview on purpose: a
//! `setInterval` in a hidden or backgrounded webview gets throttled by the OS,
//! which for a reminder app means silently missed reminders — the one failure
//! mode that makes the whole thing worthless.
//!
//! Scheduling is wall-clock based. Each rule stores the `SystemTime` it is next
//! due, and the tick compares against the clock rather than counting elapsed
//! ticks, so suspending the machine does not drift the schedule.

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Datelike, Local, NaiveTime, Timelike};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::exercises::ExerciseState;
use crate::notifier;
use crate::reminder::{self, Reminder};
use crate::settings::{self, PopupStyle, Posture, RuleKind, Settings, WorkingHours};

/// How often we wake to check the clock. Short enough that a due reminder is
/// never noticeably late, long enough to cost nothing.
const TICK: Duration = Duration::from_secs(5);

pub struct SchedulerState {
    pub settings: Settings,
    next_due: HashMap<RuleKind, SystemTime>,
    /// Tracks whether we have already logged that we are outside working hours,
    /// so a quiet weekend does not fill the log with one line every 5 seconds.
    reported_quiet: bool,
    /// Set by an explicit pause, or by snoozing every rule at once.
    paused_until: Option<SystemTime>,
}

impl SchedulerState {
    pub fn new(settings: Settings) -> Self {
        let mut state = Self {
            settings,
            next_due: HashMap::new(),
            reported_quiet: false,
            paused_until: None,
        };
        state.reschedule_all();
        state
    }

    /// Recompute every rule's due time from now. Called on startup and whenever
    /// settings change, so an edited interval takes effect immediately instead
    /// of after the old one elapses.
    pub fn reschedule_all(&mut self) {
        let now = SystemTime::now();
        let kinds: Vec<RuleKind> = self.settings.rules.iter().map(|rule| rule.kind).collect();
        for kind in kinds {
            self.reschedule(kind, now);
        }
    }

    fn reschedule(&mut self, kind: RuleKind, from: SystemTime) {
        if let Some(rule) = self.settings.rule(kind) {
            let interval = Duration::from_secs(u64::from(rule.interval_minutes) * 60);
            self.next_due.insert(kind, from + interval);
        }
    }

    fn due(&self, kind: RuleKind) -> Option<SystemTime> {
        self.next_due.get(&kind).copied()
    }

    pub fn is_paused(&self) -> bool {
        self.paused_until
            .is_some_and(|until| SystemTime::now() < until)
    }

    pub fn pause_for(&mut self, duration: Duration) {
        self.paused_until = Some(SystemTime::now() + duration);
    }

    pub fn resume(&mut self) {
        self.paused_until = None;
        self.reschedule_all();
    }

    /// Push a single rule back, leaving the others alone.
    pub fn snooze(&mut self, kind: RuleKind) {
        let snooze = Duration::from_secs(u64::from(self.settings.snooze_minutes) * 60);
        self.next_due.insert(kind, SystemTime::now() + snooze);
    }
}

/// A snapshot for the tray menu and the settings window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub paused: bool,
    pub posture: Posture,
    /// Seconds until each enabled rule next fires, keyed by rule name.
    pub next_exercise_seconds: Option<i64>,
    pub next_posture_seconds: Option<i64>,
    pub within_working_hours: bool,
}

pub fn status(app: &AppHandle) -> Status {
    let state = app.state::<Mutex<SchedulerState>>();
    let state = state.lock().expect("scheduler state");

    let seconds_until = |kind: RuleKind| -> Option<i64> {
        let enabled = state.settings.rule(kind).is_some_and(|rule| rule.enabled);
        if !enabled {
            return None;
        }
        state.due(kind).map(|due| {
            due.duration_since(SystemTime::now())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
    };

    Status {
        paused: state.is_paused(),
        posture: state.settings.posture,
        next_exercise_seconds: seconds_until(RuleKind::Exercise),
        next_posture_seconds: seconds_until(RuleKind::Posture),
        within_working_hours: within_working_hours(Local::now(), &state.settings.working_hours),
    }
}

pub fn start(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(TICK);
        tick(&app);
    });
}

fn tick(app: &AppHandle) {
    let now = SystemTime::now();

    // Decide what to fire while holding the lock, then fire after releasing it:
    // showing a notification can block, and holding the lock across it would
    // stall the settings window.
    let mut to_fire: Vec<RuleKind> = Vec::new();

    {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");

        if state.is_paused() {
            return;
        }

        if !within_working_hours(Local::now(), &state.settings.working_hours) {
            if !state.reported_quiet {
                eprintln!("habitapp: outside working hours — reminders are quiet");
                state.reported_quiet = true;
            }
            // Outside working hours we keep pushing the due times forward, so
            // the morning does not open with a backlog of overnight reminders.
            state.reschedule_all();
            return;
        }

        if state.reported_quiet {
            eprintln!("habitapp: inside working hours — reminders resume");
            state.reported_quiet = false;
        }

        let kinds: Vec<RuleKind> = state
            .settings
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .map(|rule| rule.kind)
            .collect();

        for kind in kinds {
            if state.due(kind).is_some_and(|due| now >= due) {
                to_fire.push(kind);
                // Re-anchor to now rather than to the old due time: after a wake
                // from sleep that anchor can be far in the past, and we want one
                // catch-up reminder followed by a fresh interval, not a burst of
                // everything missed overnight.
                state.reschedule(kind, now);
            }
        }
    }

    for kind in to_fire {
        fire(app, kind);
    }
}

pub fn fire(app: &AppHandle, kind: RuleKind) {
    let reminder = match kind {
        RuleKind::Exercise => {
            let chosen = {
                let state = app.state::<Mutex<ExerciseState>>();
                let mut state = state.lock().expect("exercise state");
                state.next()
            };

            match chosen {
                Some(exercise) => Reminder {
                    kind,
                    title: exercise.title,
                    body: exercise.instructions,
                    duration_seconds: exercise.duration_seconds,
                    media: exercise.media,
                },
                // Every exercise disabled is a deliberate choice, not an error —
                // say so rather than firing an empty reminder.
                None => Reminder {
                    kind,
                    title: "Time to move".to_string(),
                    body: "No exercises are enabled. Open Settings to pick some.".to_string(),
                    duration_seconds: None,
                    media: None,
                },
            }
        }
        RuleKind::Posture => {
            let posture = {
                let state = app.state::<Mutex<SchedulerState>>();
                let state = state.lock().expect("scheduler state");
                state.settings.posture
            };

            Reminder {
                kind,
                title: "Change position".to_string(),
                body: posture.prompt().to_string(),
                duration_seconds: None,
                media: None,
            }
        }
    };

    let style = {
        let state = app.state::<Mutex<SchedulerState>>();
        let state = state.lock().expect("scheduler state");
        state.settings.popup_style
    };

    match style {
        PopupStyle::Window => reminder::show(app, reminder),
        PopupStyle::Notification => {
            notifier::show(app, &reminder.title, &reminder.body);
            // With no window there is no Done button to press, so a posture
            // reminder has to assume you complied. The popup path waits for a
            // real answer instead.
            if kind == RuleKind::Posture {
                toggle_posture(app);
            }
        }
    }

    crate::tray::refresh(app);
}

/// Flip the stored posture and persist it.
pub fn toggle_posture(app: &AppHandle) {
    let updated = {
        let state = app.state::<Mutex<SchedulerState>>();
        let mut state = state.lock().expect("scheduler state");
        state.settings.posture = state.settings.posture.toggled();
        state.settings.clone()
    };
    let _ = settings::save(app, &updated);
    crate::tray::refresh(app);
}

/// Whether reminders are allowed to fire at `now`.
pub fn within_working_hours(now: DateTime<Local>, hours: &WorkingHours) -> bool {
    if !hours.enabled {
        return true;
    }

    let weekday = now.weekday().number_from_monday() as u8;
    if !hours.days.contains(&weekday) {
        return false;
    }

    let (Some(start), Some(end)) = (parse_time(&hours.start), parse_time(&hours.end)) else {
        // An unparseable window should not silence the app entirely.
        return true;
    };

    let current = NaiveTime::from_hms_opt(now.hour(), now.minute(), 0).unwrap_or(start);

    if start <= end {
        current >= start && current < end
    } else {
        // A window that wraps past midnight, e.g. a night shift.
        current >= start || current < end
    }
}

fn parse_time(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn hours(start: &str, end: &str, days: Vec<u8>) -> WorkingHours {
        WorkingHours {
            enabled: true,
            start: start.to_string(),
            end: end.to_string(),
            days,
        }
    }

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("unambiguous local time")
    }

    #[test]
    fn fires_inside_the_window() {
        // 2026-09-21 is a Monday.
        let monday_noon = at(2026, 9, 21, 12, 0);
        assert!(within_working_hours(
            monday_noon,
            &hours("09:00", "17:00", vec![1, 2, 3, 4, 5])
        ));
    }

    #[test]
    fn silent_outside_the_window() {
        let monday_evening = at(2026, 9, 21, 20, 0);
        assert!(!within_working_hours(
            monday_evening,
            &hours("09:00", "17:00", vec![1, 2, 3, 4, 5])
        ));
    }

    #[test]
    fn silent_at_weekends() {
        // 2026-09-20 is a Sunday.
        let sunday_noon = at(2026, 9, 20, 12, 0);
        assert!(!within_working_hours(
            sunday_noon,
            &hours("09:00", "17:00", vec![1, 2, 3, 4, 5])
        ));
    }

    #[test]
    fn end_is_exclusive() {
        let at_five = at(2026, 9, 21, 17, 0);
        assert!(!within_working_hours(
            at_five,
            &hours("09:00", "17:00", vec![1, 2, 3, 4, 5])
        ));
    }

    #[test]
    fn handles_a_window_wrapping_midnight() {
        let night = hours("22:00", "06:00", vec![1, 2, 3, 4, 5, 6, 7]);
        assert!(within_working_hours(at(2026, 9, 21, 23, 0), &night));
        assert!(within_working_hours(at(2026, 9, 21, 2, 0), &night));
        assert!(!within_working_hours(at(2026, 9, 21, 12, 0), &night));
    }

    #[test]
    fn disabled_window_always_fires() {
        let mut always = hours("09:00", "17:00", vec![1]);
        always.enabled = false;
        assert!(within_working_hours(at(2026, 9, 20, 3, 0), &always));
    }

    #[test]
    fn unparseable_window_does_not_silence_the_app() {
        assert!(within_working_hours(
            at(2026, 9, 21, 12, 0),
            &hours("nonsense", "17:00", vec![1, 2, 3, 4, 5])
        ));
    }

    #[test]
    fn posture_alternates() {
        assert_eq!(Posture::Sitting.toggled(), Posture::Standing);
        assert_eq!(Posture::Standing.toggled(), Posture::Sitting);
    }
}
