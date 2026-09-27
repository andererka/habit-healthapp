import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import App from "./App";
import { ReminderWindow } from "./components/ReminderWindow";
import "./App.css";

// Both windows load the same bundle; the window label decides what renders.
const isReminder = getCurrentWindow().label === "reminder";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isReminder ? <ReminderWindow /> : <App />}</React.StrictMode>,
);
