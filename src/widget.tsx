import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { WidgetApp } from "./app/WidgetApp";
import "./styles/global.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Missing widget root element");
}

createRoot(root).render(
  <StrictMode>
    <WidgetApp />
  </StrictMode>,
);
