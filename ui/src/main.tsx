import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import "./i18n";
import { watchSystemTheme } from "./lib/theme";
import "./styles/base.css";
import "./styles/tokens.css";

watchSystemTheme();

const root = document.getElementById("root");
if (!root) {
  throw new Error("index.html has no #root element");
}
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
