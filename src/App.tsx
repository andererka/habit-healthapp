import { useCallback, useEffect, useState } from "react";

import { ExerciseLibrary } from "./components/ExerciseLibrary";
import * as api from "./lib/api";
import type { ReminderRule, RuleKind, Settings, Status } from "./types";
import "./App.css";

const DAYS = [
  { value: 1, label: "Mon" },
  { value: 2, label: "Tue" },
  { value: 3, label: "Wed" },
  { value: 4, label: "Thu" },
  { value: 5, label: "Fri" },
  { value: 6, label: "Sat" },
  { value: 7, label: "Sun" },
];

const RULE_LABELS: Record<RuleKind, { title: string; hint: string }> = {
  exercise: {
    title: "Exercise reminder",
    hint: "A short exercise to get you out of your chair.",
  },
  posture: {
    title: "Position change",
    hint: "Switch between sitting and standing.",
  },
};

function countdown(seconds: number | null): string {
  if (seconds === null) return "off";
  if (seconds < 60) return "under a minute";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

type Tab = "reminders" | "exercises";

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [tab, setTab] = useState<Tab>("reminders");

  useEffect(() => {
    api.getSettings().then(setSettings);
  }, []);

  // The tray menu and this window show the same countdowns, so keep them in
  // step rather than letting the window go stale while it sits open.
  useEffect(() => {
    const refresh = () => api.getStatus().then(setStatus);
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => clearInterval(timer);
  }, []);

  // Every edit persists immediately. A settings window with a Save button is a
  // settings window you can lose work in.
  const update = useCallback((next: Settings) => {
    setSettings(next);
    api.setSettings(next);
  }, []);

  if (!settings) {
    return (
      <main className="container">
        <p className="muted">Loading…</p>
      </main>
    );
  }

  const updateRule = (kind: RuleKind, patch: Partial<ReminderRule>) =>
    update({
      ...settings,
      rules: settings.rules.map((rule) =>
        rule.kind === kind ? { ...rule, ...patch } : rule,
      ),
    });

  const toggleDay = (day: number) => {
    const days = settings.workingHours.days.includes(day)
      ? settings.workingHours.days.filter((d) => d !== day)
      : [...settings.workingHours.days, day].sort((a, b) => a - b);
    update({ ...settings, workingHours: { ...settings.workingHours, days } });
  };

  return (
    <main className="container">
      <header>
        <h1>HabitHealthApp</h1>
        <p className="muted">
          {status?.paused
            ? "Reminders are paused."
            : status && !status.withinWorkingHours
              ? "Outside working hours — reminders are quiet."
              : `Next exercise in ${countdown(status?.nextExerciseSeconds ?? null)}, position change in ${countdown(status?.nextPostureSeconds ?? null)}.`}
        </p>
      </header>

      <nav className="tabs">
        <button
          type="button"
          className={tab === "reminders" ? "tab on" : "tab"}
          onClick={() => setTab("reminders")}
        >
          Reminders
        </button>
        <button
          type="button"
          className={tab === "exercises" ? "tab on" : "tab"}
          onClick={() => setTab("exercises")}
        >
          Exercises
        </button>
      </nav>

      {tab === "exercises" && <ExerciseLibrary />}

      {tab === "reminders" && settings.rules.map((rule) => (
        <section key={rule.kind} className="card">
          <div className="card-head">
            <label className="toggle">
              <input
                type="checkbox"
                checked={rule.enabled}
                onChange={(event) =>
                  updateRule(rule.kind, { enabled: event.target.checked })
                }
              />
              <span>{RULE_LABELS[rule.kind].title}</span>
            </label>
            <button type="button" onClick={() => api.remindNow(rule.kind)}>
              Test
            </button>
          </div>
          <p className="muted">{RULE_LABELS[rule.kind].hint}</p>
          <label className="field">
            Every
            <input
              type="number"
              min={1}
              max={480}
              value={rule.intervalMinutes}
              disabled={!rule.enabled}
              onChange={(event) =>
                updateRule(rule.kind, {
                  intervalMinutes: Math.max(1, Number(event.target.value)),
                })
              }
            />
            minutes
          </label>
        </section>
      ))}

      {tab === "reminders" && (
      <section className="card">
        {status && !status.withinWorkingHours && settings.workingHours.enabled && (
          <p className="quiet-now">
            Quiet right now — it is outside your working hours, so no reminders
            will fire. Testing a reminder above still works.
          </p>
        )}
        <label className="toggle">
          <input
            type="checkbox"
            checked={settings.workingHours.enabled}
            onChange={(event) =>
              update({
                ...settings,
                workingHours: {
                  ...settings.workingHours,
                  enabled: event.target.checked,
                },
              })
            }
          />
          <span>Only during working hours</span>
        </label>

        <div className="row">
          <label className="field">
            From
            <input
              type="time"
              value={settings.workingHours.start}
              disabled={!settings.workingHours.enabled}
              onChange={(event) =>
                update({
                  ...settings,
                  workingHours: {
                    ...settings.workingHours,
                    start: event.target.value,
                  },
                })
              }
            />
          </label>
          <label className="field">
            to
            <input
              type="time"
              value={settings.workingHours.end}
              disabled={!settings.workingHours.enabled}
              onChange={(event) =>
                update({
                  ...settings,
                  workingHours: {
                    ...settings.workingHours,
                    end: event.target.value,
                  },
                })
              }
            />
          </label>
        </div>

        <div className="days">
          {DAYS.map((day) => (
            <button
              key={day.value}
              type="button"
              className={
                settings.workingHours.days.includes(day.value) ? "day on" : "day"
              }
              disabled={!settings.workingHours.enabled}
              onClick={() => toggleDay(day.value)}
            >
              {day.label}
            </button>
          ))}
        </div>
      </section>
      )}

      {tab === "reminders" && (
      <section className="card">
        <label className="field">
          Snooze length
          <input
            type="number"
            min={1}
            max={120}
            value={settings.snoozeMinutes}
            onChange={(event) =>
              update({
                ...settings,
                snoozeMinutes: Math.max(1, Number(event.target.value)),
              })
            }
          />
          minutes
        </label>

        <label className="field">
          Remind me with
          <select
            value={settings.popupStyle}
            onChange={(event) =>
              update({
                ...settings,
                popupStyle: event.target.value as Settings["popupStyle"],
              })
            }
          >
            <option value="window">a window that waits for me</option>
            <option value="notification">a notification</option>
          </select>
        </label>
        <p className="muted small">
          Notification banners disappear after a few seconds whether or not you
          saw them.
        </p>

        <div className="row">
          {status?.paused ? (
            <button type="button" onClick={() => api.resume()}>
              Resume reminders
            </button>
          ) : (
            <>
              <button type="button" onClick={() => api.pause(30)}>
                Pause 30 min
              </button>
              <button type="button" onClick={() => api.pause(60)}>
                Pause 1 hour
              </button>
            </>
          )}
        </div>
      </section>
      )}

      <p className="muted small">
        HabitHealthApp keeps running in the menu bar when you close this window.
      </p>
    </main>
  );
}

export default App;
