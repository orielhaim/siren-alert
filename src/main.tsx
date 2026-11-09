import "./i18n";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { WarmTooltipGroup } from "./components/WarmTooltip";

const root = document.getElementById("root");
if (!root) throw new Error("Missing application root");
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <WarmTooltipGroup delay={350} warmWindow={400}>
      <App />
    </WarmTooltipGroup>
  </React.StrictMode>,
);
