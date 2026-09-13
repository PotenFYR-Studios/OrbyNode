import React from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import App from "./App";
import "./styles/app.css";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root element");

const app = React.createElement(React.StrictMode, null, React.createElement(App));
if (root.hasChildNodes()) hydrateRoot(root, app);
else createRoot(root).render(app);
