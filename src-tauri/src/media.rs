//! Images and videos attached to exercises.
//!
//! Files are copied into `media/` inside the app data directory and referenced
//! by filename alone. Copying rather than pointing at wherever you found the
//! file means an exercise cannot break later because you tidied your Downloads
//! folder — and it keeps everything the app needs in one directory you can back
//! up or move wholesale.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::exercises::Media;

/// Formats the webview can actually display. Anything else is rejected at
/// attach time, which is a far better moment to find out than when a reminder
/// fires and shows a blank box.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "avif"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "webm"];

pub fn media_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("media");

    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

/// Decide what kind of media a path holds, by extension.
pub fn classify(path: &Path) -> Option<fn(String) -> Media> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())?
        .to_lowercase();

    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(|src| Media::Image { src })
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(|src| Media::Video { src })
    } else {
        None
    }
}

/// Strip anything that could escape the media directory or upset a filesystem.
/// The stem is cosmetic — it only exists so the folder stays browsable by hand.
fn sanitise(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "media".to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn unique_name(source: &Path) -> Result<String, String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .ok_or("that file has no extension, so I cannot tell what it is")?
        .to_lowercase();

    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("media");

    // Two files called `stretch.png` must not collide, so the name carries a
    // timestamp rather than trusting the original.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    Ok(format!("{}-{stamp}.{extension}", sanitise(stem)))
}

/// Copy a file the user picked into the media directory.
pub fn attach(app: &AppHandle, source: &Path) -> Result<Media, String> {
    let build = classify(source).ok_or_else(|| {
        format!(
            "unsupported file type — images: {}, videos: {}",
            IMAGE_EXTENSIONS.join(", "),
            VIDEO_EXTENSIONS.join(", ")
        )
    })?;

    if !source.is_file() {
        return Err("that file does not exist".to_string());
    }

    let name = unique_name(source)?;
    let destination = media_dir(app)?.join(&name);

    std::fs::copy(source, &destination).map_err(|error| format!("could not copy: {error}"))?;

    Ok(build(name))
}

/// Where the bundled starter images live inside the app bundle.
const BUNDLED_DIR: &str = "resources/exercises";

/// Find the file behind a media reference.
///
/// Two places can hold one: the user's own media directory, and the images
/// shipped inside the app bundle for the default exercises. User files win, so
/// replacing a bundled illustration with your own just works.
pub fn resolve(app: &AppHandle, src: &str) -> Result<PathBuf, String> {
    // `src` arrives from the frontend, so a value like `../../settings.json`
    // must never be joined onto a directory.
    if Path::new(src).components().count() != 1 {
        return Err("unexpected media reference".to_string());
    }

    let user = media_dir(app)?.join(src);
    if user.is_file() {
        return Ok(user);
    }

    let bundled = app
        .path()
        .resolve(format!("{BUNDLED_DIR}/{src}"), BaseDirectory::Resource)
        .map_err(|error| error.to_string())?;
    if bundled.is_file() {
        return Ok(bundled);
    }

    Err(format!("no media file called {src}"))
}

/// Delete a media file. Missing is fine — the point is that it is gone.
///
/// Only the user's own files are deletable: a bundled image belongs to the app
/// bundle, and removing it would break the default library for good.
pub fn remove(app: &AppHandle, media: &Media) -> Result<(), String> {
    let name = media.src();

    // Refuse anything that is not a plain filename: `src` comes back through
    // the frontend, and a path like `../../settings.json` must not reach
    // `remove_file`.
    if Path::new(name).components().count() != 1 {
        return Err("unexpected media reference".to_string());
    }

    let path = media_dir(app)?.join(name);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_images_and_videos_case_insensitively() {
        assert!(matches!(
            classify(Path::new("a.PNG")).map(|f| f("x".into())),
            Some(Media::Image { .. })
        ));
        assert!(matches!(
            classify(Path::new("a.mp4")).map(|f| f("x".into())),
            Some(Media::Video { .. })
        ));
    }

    #[test]
    fn rejects_formats_the_webview_cannot_show() {
        assert!(classify(Path::new("notes.pdf")).is_none());
        assert!(classify(Path::new("clip.avi")).is_none());
        assert!(classify(Path::new("no-extension")).is_none());
    }

    #[test]
    fn sanitises_names_that_could_escape_the_directory() {
        assert_eq!(sanitise("../../etc/passwd"), "etc-passwd");
        assert_eq!(sanitise("hello world!"), "hello-world");
        assert_eq!(sanitise("***"), "media");
        assert_eq!(sanitise(""), "media");
    }

    #[test]
    fn generated_names_keep_the_extension_and_differ() {
        let first = unique_name(Path::new("/tmp/Shoulder Rolls.PNG")).unwrap();
        assert!(first.ends_with(".png"), "{first}");
        assert!(first.starts_with("Shoulder-Rolls-"), "{first}");
    }

    #[test]
    fn long_names_are_truncated() {
        let long = "a".repeat(200);
        assert!(sanitise(&long).len() <= 40);
    }
}
