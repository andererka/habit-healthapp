//! Notification delivery.
//!
//! macOS needs its own implementation. `tauri-plugin-notification` delegates to
//! `mac-notification-sys`, which only speaks the `NSUserNotification` API that
//! Apple removed in macOS 26 — it reports success and delivers nothing, never
//! prompts for permission, and never registers the app in System Settings.
//! So on macOS we talk to `UNUserNotificationCenter` directly.
//!
//! Windows and Linux keep the plugin, where it works (WinRT toasts, D-Bus).

use tauri::AppHandle;

/// Ask the OS for permission to post notifications. Safe to call on every
/// launch: the prompt only appears the first time.
pub fn request_authorization(app: &AppHandle) {
    imp::request_authorization(app);
}

pub fn show(app: &AppHandle, title: &str, body: &str) {
    imp::show(app, title, body);
}

#[cfg(target_os = "macos")]
mod imp {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSBundle, NSError, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest,
        UNUserNotificationCenter,
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    use tauri::AppHandle;

    /// `UNUserNotificationCenter::currentNotificationCenter` raises an
    /// Objective-C exception — which aborts the process — when the executable is
    /// not inside an app bundle. `tauri dev` runs exactly such a bare binary, so
    /// this guard is what keeps development from crashing on every reminder.
    fn is_bundled() -> bool {
        NSBundle::mainBundle().bundleIdentifier().is_some()
    }

    pub fn request_authorization(_app: &AppHandle) {
        if !is_bundled() {
            eprintln!(
                "HabitHealthApp: unbundled binary (tauri dev) — skipping notification \
                 authorization. Build a bundle to test notifications."
            );
            return;
        }

        let center = UNUserNotificationCenter::currentNotificationCenter();
        let handler = RcBlock::new(|granted: Bool, error: *mut NSError| {
            if granted.as_bool() {
                eprintln!("HabitHealthApp: notification authorization granted");
            } else {
                eprintln!("HabitHealthApp: notification authorization DENIED");
            }
            if !error.is_null() {
                let message = unsafe { (*error).localizedDescription() };
                eprintln!("HabitHealthApp: authorization error: {message}");
            }
        });

        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &handler,
        );
    }

    pub fn show(_app: &AppHandle, title: &str, body: &str) {
        if !is_bundled() {
            eprintln!("HabitHealthApp: unbundled binary — would notify: {title} / {body}");
            return;
        }

        let center = UNUserNotificationCenter::currentNotificationCenter();

        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));

        // Identifiers must be unique or a later notification silently replaces
        // the earlier one in Notification Center.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let identifier = NSString::from_str(&format!("HabitHealthApp-{nanos}"));

        // `trigger: None` means deliver immediately.
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &content,
            None,
        );

        let completion = RcBlock::new(|error: *mut NSError| {
            if error.is_null() {
                eprintln!("HabitHealthApp: notification delivered");
            } else {
                let message = unsafe { (*error).localizedDescription() };
                eprintln!("HabitHealthApp: notification FAILED: {message}");
            }
        });

        center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use tauri::AppHandle;
    use tauri_plugin_notification::NotificationExt;

    /// Windows and Linux grant notification access without a runtime prompt.
    pub fn request_authorization(_app: &AppHandle) {}

    pub fn show(app: &AppHandle, title: &str, body: &str) {
        let result = app.notification().builder().title(title).body(body).show();
        if let Err(error) = result {
            eprintln!("HabitHealthApp: notification FAILED: {error}");
        }
    }
}
