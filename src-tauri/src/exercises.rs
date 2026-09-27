//! The exercise library.
//!
//! Exercises are user-editable and stored as plain JSON in the app data
//! directory, seeded on first run with the defaults below. Keeping it plain
//! JSON is deliberate: exporting the file and sending it to a colleague is how
//! exercise sets spread through a team without any backend.

use std::sync::Arc;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

const STORE_FILE: &str = "exercises.json";
const EXERCISES_KEY: &str = "exercises";

/// Optional media for an exercise.
///
/// `src` is a bare filename inside the app's `media/` directory, never a full
/// path: the files are copied in on attach, so an exercise cannot break because
/// the original was moved, and the whole library stays in one directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Media {
    Image { src: String },
    Video { src: String },
}

impl Media {
    pub fn src(&self) -> &str {
        match self {
            Media::Image { src } | Media::Video { src } => src,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exercise {
    pub id: String,
    pub title: String,
    /// Shown in the reminder. Plain text for now; M4 renders it in the popup.
    pub instructions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Media>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub enabled: bool,
}

fn exercise(id: &str, title: &str, instructions: &str, seconds: u32, tags: &[&str]) -> Exercise {
    Exercise {
        id: id.to_string(),
        title: title.to_string(),
        instructions: instructions.to_string(),
        duration_seconds: Some(seconds),
        // Every default ships with an illustration bundled inside the app, named
        // after the exercise id. Attaching your own file replaces it, because
        // media resolution prefers the user's media directory.
        media: Some(Media::Image {
            src: format!("{id}.jpg"),
        }),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        enabled: true,
    }
}

/// The seed library: short, equipment-free, and doable in an open-plan office
/// without attracting attention — an exercise you would feel silly doing at
/// your desk is one you will skip.
pub fn defaults() -> Vec<Exercise> {
    vec![
        exercise(
            "neck-rolls",
            "Neck rolls",
            "Drop your chin to your chest and roll your head slowly in a half circle to one \
             shoulder, then the other. Keep it slow — five circles each way.",
            40,
            &["neck"],
        ),
        exercise(
            "shoulder-rolls",
            "Shoulder rolls",
            "Roll both shoulders backwards in a big circle ten times, then forwards ten times. \
             Let your arms hang loose.",
            30,
            &["shoulders"],
        ),
        exercise(
            "chest-opener",
            "Chest opener",
            "Clasp your hands behind your back, straighten your arms and lift them gently while \
             opening your chest. Hold for twenty seconds and breathe.",
            30,
            &["chest", "posture"],
        ),
        exercise(
            "wrist-stretch",
            "Wrist stretch",
            "Extend one arm, palm forward like a stop sign, and pull the fingers gently back with \
             the other hand. Fifteen seconds, then swap. Repeat with the palm facing down.",
            45,
            &["wrists"],
        ),
        exercise(
            "seated-spinal-twist",
            "Seated spinal twist",
            "Sit tall, place one hand on the opposite knee and turn your torso towards the back of \
             the chair. Hold twenty seconds each side.",
            45,
            &["back"],
        ),
        exercise(
            "hip-flexor-stretch",
            "Hip flexor stretch",
            "Stand up. Take a big step back with one foot, tuck your pelvis under and lean forward \
             slightly. Twenty seconds per side.",
            50,
            &["hips", "standing"],
        ),
        exercise(
            "calf-raises",
            "Calf raises",
            "Stand up, rise onto your toes and lower slowly. Twenty repetitions. Hold the desk if \
             you need the balance.",
            40,
            &["legs", "standing"],
        ),
        exercise(
            "eye-break",
            "Eye break",
            "Look at something at least twenty metres away for twenty seconds. Out of a window is \
             ideal. Then blink deliberately ten times.",
            25,
            &["eyes"],
        ),
        exercise(
            "standing-back-extension",
            "Standing back extension",
            "Stand up, put your hands on your lower back and lean gently backwards, looking up. \
             Five slow repetitions.",
            30,
            &["back", "standing"],
        ),
        exercise(
            "walk-about",
            "Walk about",
            "Get up and walk somewhere — the kitchen, the window, the far end of the corridor. \
             Fill a glass of water while you are at it.",
            120,
            &["movement"],
        ),
    ]
}

/// The library plus the queue that walks it, held together because the queue is
/// only meaningful against a particular library.
pub struct ExerciseState {
    pub library: Vec<Exercise>,
    pub queue: Queue,
}

impl ExerciseState {
    pub fn new(library: Vec<Exercise>) -> Self {
        Self {
            library,
            queue: Queue::default(),
        }
    }

    pub fn next(&mut self) -> Option<Exercise> {
        let library = self.library.clone();
        self.queue.next(&library)
    }
}

fn store(app: &AppHandle) -> Result<Arc<Store<Wry>>, String> {
    app.store(STORE_FILE).map_err(|error| error.to_string())
}

/// Read the library, seeding it with the defaults the first time.
pub fn load(app: &AppHandle) -> Vec<Exercise> {
    let Ok(store) = store(app) else {
        return defaults();
    };

    let stored: Option<Vec<Exercise>> =
        store
            .get(EXERCISES_KEY)
            .and_then(|value| match serde_json::from_value(value) {
                Ok(exercises) => Some(exercises),
                Err(error) => {
                    eprintln!("habitapp: ignoring unreadable exercise library ({error})");
                    None
                }
            });

    match stored {
        Some(exercises) => exercises,
        None => {
            let seeded = defaults();
            let _ = save(app, &seeded);
            seeded
        }
    }
}

pub fn save(app: &AppHandle, exercises: &[Exercise]) -> Result<(), String> {
    let store = store(app)?;
    let value = serde_json::to_value(exercises).map_err(|error| error.to_string())?;
    store.set(EXERCISES_KEY, value);
    store.save().map_err(|error| error.to_string())
}

/// Picks exercises in a shuffled round-robin.
///
/// A plain random pick repeats often enough to feel broken — being told to do
/// neck rolls three times running reads as a bug. Instead we shuffle the whole
/// enabled set, walk it to the end, then reshuffle.
#[derive(Default)]
pub struct Queue {
    upcoming: Vec<String>,
    last_id: Option<String>,
}

impl Queue {
    pub fn next(&mut self, library: &[Exercise]) -> Option<Exercise> {
        let enabled: Vec<&Exercise> = library.iter().filter(|item| item.enabled).collect();
        if enabled.is_empty() {
            return None;
        }

        if self.upcoming.is_empty() {
            self.refill(&enabled);
        }

        // Ids can disappear when the library is edited between refills.
        while let Some(id) = self.upcoming.pop() {
            if let Some(found) = enabled.iter().find(|item| item.id == id) {
                self.last_id = Some(found.id.clone());
                return Some((*found).clone());
            }
        }

        self.refill(&enabled);
        let chosen = self.upcoming.pop()?;
        let found = enabled.iter().find(|item| item.id == chosen)?;
        self.last_id = Some(found.id.clone());
        Some((*found).clone())
    }

    fn refill(&mut self, enabled: &[&Exercise]) {
        let mut ids: Vec<String> = enabled.iter().map(|item| item.id.clone()).collect();
        ids.shuffle(&mut rand::rng());

        // `next` pops from the back, so the last element is served first: make
        // sure it is not the one we just did.
        if ids.len() > 1 {
            if let Some(last) = &self.last_id {
                if ids.last() == Some(last) {
                    let len = ids.len();
                    ids.swap(0, len - 1);
                }
            }
        }

        self.upcoming = ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(ids: &[&str]) -> Vec<Exercise> {
        ids.iter()
            .map(|id| exercise(id, id, "instructions", 30, &[]))
            .collect()
    }

    #[test]
    fn empty_library_yields_nothing() {
        let mut queue = Queue::default();
        assert!(queue.next(&[]).is_none());
    }

    #[test]
    fn disabled_exercises_are_never_served() {
        let mut library = library(&["a", "b"]);
        library[0].enabled = false;

        let mut queue = Queue::default();
        for _ in 0..10 {
            assert_eq!(
                queue.next(&library).map(|item| item.id).as_deref(),
                Some("b")
            );
        }
    }

    #[test]
    fn serves_every_exercise_before_repeating_any() {
        let library = library(&["a", "b", "c", "d"]);
        let mut queue = Queue::default();

        let mut seen: Vec<String> = (0..4)
            .filter_map(|_| queue.next(&library).map(|item| item.id))
            .collect();
        seen.sort();

        assert_eq!(seen, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn does_not_repeat_across_a_reshuffle() {
        let library = library(&["a", "b", "c"]);

        // Run enough cycles that a naive reshuffle would eventually collide.
        for _ in 0..50 {
            let mut queue = Queue::default();
            let mut previous: Option<String> = None;
            for _ in 0..9 {
                let id = queue
                    .next(&library)
                    .map(|item| item.id)
                    .expect("an exercise");
                assert_ne!(previous.as_deref(), Some(id.as_str()));
                previous = Some(id);
            }
        }
    }

    #[test]
    fn survives_the_library_changing_under_it() {
        let mut queue = Queue::default();
        let first = library(&["a", "b", "c"]);
        queue.next(&first);

        // The whole library is replaced between calls.
        let replaced = library(&["x", "y"]);
        let chosen = queue.next(&replaced).expect("an exercise");
        assert!(["x", "y"].contains(&chosen.id.as_str()));
    }

    #[test]
    fn defaults_are_well_formed() {
        let defaults = defaults();
        assert!(defaults.len() >= 8);
        assert!(defaults.iter().all(|item| item.enabled));
        assert!(defaults.iter().all(|item| !item.instructions.is_empty()));

        let mut ids: Vec<&str> = defaults.iter().map(|item| item.id.as_str()).collect();
        ids.sort();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "exercise ids must be unique");
    }
}
