# Developing HabitHealthApp

Requires [Node](https://nodejs.org) 20+ and [Rust](https://rustup.rs).

```sh
npm install
npm run tauri dev    # settings UI with hot reload
```

## Notifications and the popup cannot be tested with `tauri dev` on macOS

Dev runs an unbundled binary, and macOS refuses to register such a binary with
Notification Center — `UNUserNotificationCenter` raises an exception outside an
app bundle, which `notifier.rs` guards against. Build a real bundle instead:

```sh
npm run app          # debug build, bundled and ad-hoc signed
open src-tauri/target/debug/bundle/macos/HabitHealthApp.app
```

The bundle must be signed, even ad-hoc, or macOS will not register it for
notifications at all. `bundle.macOS.signingIdentity: "-"` in `tauri.conf.json`
handles that; without it Tauri leaves the linker's ad-hoc identity with
`Info.plist` unbound and notifications fail silently.

## Checks

```sh
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
```

## Where things live

| Path | What |
|---|---|
| `src-tauri/src/scheduler.rs` | Wall-clock reminder scheduling, working hours, pause/snooze |
| `src-tauri/src/notifier.rs` | Notifications — `UNUserNotificationCenter` on macOS, the Tauri plugin elsewhere |
| `src-tauri/src/reminder.rs` | The always-on-top popup window |
| `src-tauri/src/exercises.rs` | Exercise library, persistence, and the shuffled round-robin queue |
| `src-tauri/src/media.rs` | Attaching images/videos, and resolving them from app data or the bundle |
| `src-tauri/src/settings.rs` | Settings model and persistence |
| `src-tauri/src/tray.rs` | Menu bar icon and menu |
| `src-tauri/resources/exercises/` | Illustrations bundled with the default exercises |
| `src/` | React settings UI and reminder popup |

Settings and exercises are plain JSON in the app data directory
(`~/Library/Application Support/io.github.andererka.habithealthapp/` on macOS).

## Why the timer lives in Rust

A `setInterval` in a hidden or backgrounded webview gets throttled by the OS,
which for a reminder app means silently missed reminders. `scheduler.rs`
compares wall-clock times instead of counting ticks, so suspending the machine
does not drift the schedule, and waking produces one catch-up reminder rather
than a burst of everything missed overnight.

## Media

Attached files are copied into `media/` in the app data directory and referenced
by bare filename. Resolution prefers the user's directory and falls back to
`src-tauri/resources/exercises/`, so a user file transparently overrides a
bundled illustration.

Bundled illustrations are ~900px JPEGs. They are displayed at roughly 380px
(760px on retina), so anything larger is wasted bytes in every installer.

## The menu bar icon

`src-tauri/icons/tray.png` is a template image: macOS ignores its colour and
uses only the alpha channel, tinting the shape to match the menu bar.
Regenerate it with:

```sh
python3 scripts/make_tray_icon.py
```

## Copying app bundles

Use `ditto`, never `cp -R`. `cp -R` brings Finder metadata that invalidates the
code signature, after which macOS refuses to register the app for notifications.

```sh
ditto path/to/HabitHealthApp.app /Applications/HabitHealthApp.app
xattr -cr /Applications/HabitHealthApp.app       # if a signature check fails
codesign --verify --strict /Applications/HabitHealthApp.app
```

## Releasing

Push a tag; CI builds all three platforms and opens a **draft** release for you
to check before publishing.

```sh
npm version patch
git push --follow-tags
```

The version lives only in `package.json` — `tauri.conf.json` inherits it, so
`npm version` is the single place it changes.

Tauri cannot cross-compile, so every installer is built on its own runner; see
`.github/workflows/release.yml`.

Building a `.dmg` locally needs Automation permission for Finder (System
Settings → Privacy & Security → Automation → your terminal → Finder). Without
it `bundle_dmg.sh` fails on an AppleScript step. CI is unaffected.
