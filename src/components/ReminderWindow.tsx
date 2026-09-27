import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import * as api from "../lib/api";
import type { Answer, Reminder } from "../types";
import { MediaView } from "./MediaView";
import "./ReminderWindow.css";

export function ReminderWindow() {
  const [reminder, setReminder] = useState<Reminder | null>(null);
  const [remaining, setRemaining] = useState<number | null>(null);

  const load = useCallback((next: Reminder | null) => {
    setReminder(next);
    setRemaining(next?.durationSeconds ?? null);
  }, []);

  // Collect the payload parked for us at creation, then stay listening: a
  // second reminder can arrive while this window is still open.
  useEffect(() => {
    api.takePendingReminder().then(load);
    const unlisten = listen<Reminder>("reminder", (event) => load(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, [load]);

  // The countdown is a suggestion, not a deadline — it stops at zero and waits
  // for you rather than dismissing itself.
  useEffect(() => {
    if (remaining === null || remaining <= 0) return;
    const timer = setTimeout(() => setRemaining((value) => (value ?? 1) - 1), 1000);
    return () => clearTimeout(timer);
  }, [remaining]);

  if (!reminder) return null;

  const answer = (choice: Answer) => api.answerReminder(reminder.kind, choice);

  const total = reminder.durationSeconds ?? 0;
  const progress = total > 0 && remaining !== null ? 1 - remaining / total : 0;

  return (
    <main className="reminder">
      <header data-tauri-drag-region>
        <span className="kind">
          {reminder.kind === "posture" ? "Position change" : "Exercise"}
        </span>
      </header>

      <h1>{reminder.title}</h1>
      {reminder.media && <MediaView media={reminder.media} />}
      <p className="instructions">{reminder.body}</p>

      {remaining !== null && (
        <div className="timer">
          <div className="bar">
            <div className="fill" style={{ width: `${Math.min(progress, 1) * 100}%` }} />
          </div>
          <span className="count">
            {remaining > 0 ? `${remaining}s` : "Take your time"}
          </span>
        </div>
      )}

      <div className="actions">
        <button type="button" className="primary" onClick={() => answer("done")}>
          Done
        </button>
        <button type="button" onClick={() => answer("snooze")}>
          Snooze
        </button>
        <button type="button" onClick={() => answer("skip")}>
          Skip
        </button>
      </div>
    </main>
  );
}
