import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";

import * as api from "../lib/api";
import type { Exercise } from "../types";
import { MediaView } from "./MediaView";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "avif"];
const VIDEO_EXTENSIONS = ["mp4", "m4v", "mov", "webm"];

function blankExercise(): Exercise {
  return {
    // crypto.randomUUID is available in the Tauri webview on all target
    // platforms; ids only need to be unique within one library.
    id: crypto.randomUUID(),
    title: "",
    instructions: "",
    durationSeconds: 30,
    tags: [],
    enabled: true,
  };
}

export function ExerciseLibrary() {
  const [exercises, setExercises] = useState<Exercise[] | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    api.getExercises().then(setExercises);
  }, []);

  if (!exercises) {
    return <p className="muted">Loading…</p>;
  }

  const persist = (next: Exercise[]) => {
    setExercises(next);
    api.setExercises(next);
  };

  const update = (id: string, patch: Partial<Exercise>) =>
    persist(exercises.map((item) => (item.id === id ? { ...item, ...patch } : item)));

  const remove = (id: string) => {
    persist(exercises.filter((item) => item.id !== id));
    if (editingId === id) setEditingId(null);
  };

  const add = () => {
    const created = blankExercise();
    persist([...exercises, created]);
    setEditingId(created.id);
  };

  // The file is copied into the app's media folder, so the original can be
  // moved or deleted afterwards without breaking the exercise.
  const attach = async (id: string) => {
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Image or video",
          extensions: [...IMAGE_EXTENSIONS, ...VIDEO_EXTENSIONS],
        },
      ],
    });
    if (typeof path !== "string") return;

    try {
      const media = await api.attachMedia(path);
      update(id, { media });
      setMessage(null);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const doExport = async () => {
    const path = await save({
      defaultPath: "exercises.json",
      filters: [{ name: "Exercise library", extensions: ["json"] }],
    });
    if (!path) return;
    await api.exportLibrary(path);
    setMessage(`Exported ${exercises.length} exercises. Send that file to a colleague.`);
  };

  const doImport = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Exercise library", extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    try {
      const imported = await api.importLibrary(path);
      setExercises(imported);
      setMessage(`Imported ${imported.length} exercises, replacing your library.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const enabledCount = exercises.filter((item) => item.enabled).length;

  return (
    <>
      <section className="card">
        <div className="card-head">
          <strong>
            {enabledCount} of {exercises.length} in rotation
          </strong>
          <button type="button" onClick={add}>
            Add exercise
          </button>
        </div>
        <p className="muted small">
          Reminders cycle through every enabled exercise before repeating any.
          Export carries the text, not attached images or videos.
        </p>
        <div className="row">
          <button type="button" onClick={doExport}>
            Export library…
          </button>
          <button type="button" onClick={doImport}>
            Import library…
          </button>
        </div>
        {message && <p className="muted small">{message}</p>}
      </section>

      {enabledCount === 0 && (
        <p className="warning small">
          Nothing is enabled, so exercise reminders will tell you to pick some.
        </p>
      )}

      {exercises.map((exercise) => (
        <section key={exercise.id} className="card">
          <div className="card-head">
            <label className="toggle">
              <input
                type="checkbox"
                checked={exercise.enabled}
                onChange={(event) =>
                  update(exercise.id, { enabled: event.target.checked })
                }
              />
              <span>{exercise.title || "Untitled exercise"}</span>
            </label>
            <button
              type="button"
              onClick={() =>
                setEditingId(editingId === exercise.id ? null : exercise.id)
              }
            >
              {editingId === exercise.id ? "Done" : "Edit"}
            </button>
          </div>

          {editingId === exercise.id ? (
            <>
              <label className="field stacked">
                Title
                <input
                  type="text"
                  value={exercise.title}
                  onChange={(event) =>
                    update(exercise.id, { title: event.target.value })
                  }
                />
              </label>
              <label className="field stacked">
                Instructions
                <textarea
                  rows={4}
                  value={exercise.instructions}
                  onChange={(event) =>
                    update(exercise.id, { instructions: event.target.value })
                  }
                />
              </label>
              <label className="field">
                Takes about
                <input
                  type="number"
                  min={5}
                  max={600}
                  value={exercise.durationSeconds ?? 30}
                  onChange={(event) =>
                    update(exercise.id, {
                      durationSeconds: Math.max(5, Number(event.target.value)),
                    })
                  }
                />
                seconds
              </label>
              <label className="field stacked">
                Tags, comma separated
                <input
                  type="text"
                  value={exercise.tags.join(", ")}
                  onChange={(event) =>
                    update(exercise.id, {
                      tags: event.target.value
                        .split(",")
                        .map((tag) => tag.trim())
                        .filter(Boolean),
                    })
                  }
                />
              </label>
              <div className="media-field">
                {exercise.media ? (
                  <>
                    <MediaView media={exercise.media} />
                    <button
                      type="button"
                      onClick={async () => {
                        const attached = exercise.media;
                        update(exercise.id, { media: undefined });
                        if (attached) await api.removeMedia(attached);
                      }}
                    >
                      Remove {exercise.media.kind}
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={() => attach(exercise.id)}
                  >
                    Add image or video…
                  </button>
                )}
              </div>

              <div className="row">
                <button type="button" className="danger" onClick={() => remove(exercise.id)}>
                  Delete
                </button>
              </div>
            </>
          ) : (
            <p className="muted small">{exercise.instructions || "No instructions yet."}</p>
          )}
        </section>
      ))}
    </>
  );
}
