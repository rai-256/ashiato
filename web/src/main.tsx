// SPDX-License-Identifier: AGPL-3.0-only
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Root } from "./Root";
import { Gate } from "./session";

const root = document.getElementById("root");
// 画面に出す文ではない（index.html が壊れたときの開発者向けの例外）ので文言の表に入れない
// eslint-disable-next-line no-restricted-syntax
if (!root) throw new Error("#root が無い");
createRoot(root).render(
  <StrictMode>
    <Gate><Root /></Gate>
  </StrictMode>,
);
