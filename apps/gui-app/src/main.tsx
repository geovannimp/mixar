import { scan } from "react-scan"; // must be imported before React and React DOM
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app";
import { configureAppLogging } from "@/lib/logging";
import "./index.css";

scan({
  enabled: import.meta.env.DEV,
});

void configureAppLogging().then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
