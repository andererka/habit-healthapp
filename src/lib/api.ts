import { invoke } from "@tauri-apps/api/core";

import type {
  Answer,
  Exercise,
  Media,
  Reminder,
  RuleKind,
  Settings,
  Status,
} from "../types";

export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) =>
  invoke<void>("set_settings", { settings });

export const getStatus = () => invoke<Status>("get_status");

export const remindNow = (kind: RuleKind) => invoke<void>("remind_now", { kind });
export const pause = (minutes: number) => invoke<void>("pause", { minutes });
export const resume = () => invoke<void>("resume");

export const getExercises = () => invoke<Exercise[]>("get_exercises");
export const setExercises = (exercises: Exercise[]) =>
  invoke<void>("set_exercises", { exercises });

export const exportLibrary = (path: string) =>
  invoke<void>("export_library", { path });
export const importLibrary = (path: string) =>
  invoke<Exercise[]>("import_library", { path });

export const takePendingReminder = () =>
  invoke<Reminder | null>("take_pending_reminder");
export const answerReminder = (kind: RuleKind, answer: Answer) =>
  invoke<void>("answer_reminder", { kind, answer });

export const attachMedia = (path: string) =>
  invoke<Media>("attach_media", { path });
export const removeMedia = (media: Media) =>
  invoke<void>("remove_media", { media });
