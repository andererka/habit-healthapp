/** Mirrors the serde shapes in src-tauri/src/settings.rs and scheduler.rs. */

export type RuleKind = "exercise" | "posture";
export type Posture = "sitting" | "standing";

export type ReminderRule = {
  kind: RuleKind;
  enabled: boolean;
  intervalMinutes: number;
};

export type WorkingHours = {
  enabled: boolean;
  /** "HH:MM", local time. */
  start: string;
  end: string;
  /** Monday = 1 … Sunday = 7. */
  days: number[];
};

export type Settings = {
  rules: ReminderRule[];
  workingHours: WorkingHours;
  posture: Posture;
  snoozeMinutes: number;
  popupStyle: PopupStyle;
};

export type Status = {
  paused: boolean;
  posture: Posture;
  nextExerciseSeconds: number | null;
  nextPostureSeconds: number | null;
  withinWorkingHours: boolean;
};

/** `src` is a bare filename inside the app's media directory. */
export type Media =
  | { kind: "image"; src: string }
  | { kind: "video"; src: string };

export type Exercise = {
  id: string;
  title: string;
  instructions: string;
  durationSeconds?: number;
  media?: Media;
  tags: string[];
  enabled: boolean;
};

export type PopupStyle = "notification" | "window";

export type Reminder = {
  kind: RuleKind;
  title: string;
  body: string;
  durationSeconds: number | null;
  media: Media | null;
};

export type Answer = "done" | "snooze" | "skip";
