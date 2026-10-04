// SPDX-License-Identifier: AGPL-3.0-only
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Root } from "./Root";
import { Gate } from "./session";

const root = document.getElementById("root");
if (!root) throw new Error("#root が無い");
createRoot(root).render(
  <StrictMode>
    <Gate><Root /></Gate>
  </StrictMode>,
);
