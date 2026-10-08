// SPDX-License-Identifier: AGPL-3.0-only
import { MIN_TARGET_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** 操作対象の見た目。**24 px 以上を要求する**（NFR-19）。マスタ管理の個人属性と場所が共有する。 */
export function control(scheme: Scheme): React.CSSProperties {
  const c = SCHEMES[scheme];
  return {
    minHeight: MIN_TARGET_PX,
    minWidth: MIN_TARGET_PX,
    padding: "4px 10px",
    font: "400 15px/1.6 system-ui, sans-serif",
    color: tone(c.text),
    background: tone(c.surface2),
    border: `1px solid ${tone(c.muted)}`,
    borderRadius: 8,
    boxSizing: "border-box",
  };
}

export function link(scheme: Scheme): React.CSSProperties {
  return {
    ...control(scheme),
    background: "transparent",
    border: "none",
    display: "inline-flex",
    alignItems: "center",
  };
}
