// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 部品の見た目（2026-10-12 の表面）。**値は `tokens.ts` だけから引き、ここでは組むだけ。**
 *
 * 画面ごとに同じ形（ボタン・リンク・題・節）を別々に書いていたので、表面を変えると
 * 画面ごとに少しずつずれた。playground（`docs/ui-surface-playground.html`）の `.btn` / `.btn.pri` /
 * `.lnk2` / `h1` / `.sec` / `.mk` と同じ形をここに 1 つずつ置く。
 */
import type { CSSProperties } from "react";
import {
  DESTRUCTIVE_TARGET_PX,
  font,
  MARK_PX,
  MIN_TARGET_PX,
  RADIUS,
  SECTION_GAP_PX,
  SECTION_PAD_PX,
  SPACE,
  TYPE,
  tone,
  type Hsl,
  type SchemeColors,
} from "./tokens";

/** 画面の地（`<main>`）。 */
export function pageStyle(c: SchemeColors): CSSProperties {
  return {
    background: tone(c.ground),
    color: tone(c.text),
    font: font(TYPE.body),
    minHeight: "100vh",
    // **横スクロールを出さない**（NFR-19 / 完了の判定）
    maxWidth: "100%",
    overflowX: "hidden",
    padding: SPACE.page,
    boxSizing: "border-box",
  };
}

/** 題（h1）。**アクセントは見出しまで**（本人の決定 2026-10-12）。 */
export function titleStyle(c: SchemeColors): CSSProperties {
  return { font: font(TYPE.h1, 600, 1.3), color: tone(c.accent), margin: `0 0 ${SPACE.underTitle}px` };
}

/** 画面の行き来のリンク。アクセントの文字に下線（色だけでリンクと分からせない）。 */
export function linkStyle(c: SchemeColors): CSSProperties {
  return {
    color: tone(c.accent),
    font: font(TYPE.caption),
    textDecoration: "underline",
    textUnderlineOffset: 3,
    minHeight: MIN_TARGET_PX,
    minWidth: MIN_TARGET_PX,
    display: "inline-flex",
    alignItems: "center",
    borderRadius: 4,
  };
}

/** リンクの見た目の `<button>`（その画面の中で開く・閉じる）。button の既定の面と枠を消す。 */
export function linkButtonStyle(c: SchemeColors): CSSProperties {
  return { ...linkStyle(c), background: "transparent", border: "none" };
}

/** 副ボタン（面と枠）。**24 × 24 CSS px 以上**（NFR-19）。 */
export function controlStyle(c: SchemeColors): CSSProperties {
  return {
    minHeight: MIN_TARGET_PX,
    minWidth: MIN_TARGET_PX,
    padding: "4px 10px",
    font: font(TYPE.caption),
    color: tone(c.text),
    background: tone(c.surface2),
    border: `1px solid ${tone(c.muted)}`,
    borderRadius: RADIUS.control,
    boxSizing: "border-box",
  };
}

/** 主ボタン（書く・積む・ログイン）。**アクセントで塗る**（本人の決定 2026-10-12）。 */
export function primaryStyle(c: SchemeColors): CSSProperties {
  return { ...controlStyle(c), color: tone(c.onAccent), background: tone(c.accentFill), border: "1px solid transparent" };
}

/** 消す・止める（NFR-20。44 × 44 CSS px 以上）。 */
export function destructiveStyle(c: SchemeColors): CSSProperties {
  return { ...controlStyle(c), minHeight: DESTRUCTIVE_TARGET_PX, minWidth: DESTRUCTIVE_TARGET_PX, padding: "8px 14px" };
}

/** 入力欄。 */
export function inputStyle(c: SchemeColors): CSSProperties {
  return { ...controlStyle(c), font: font(TYPE.body), padding: "3px 8px" };
}

/** 節・カード（角 12 px、区切りの線）。`face` は節を置く面。 */
export function sectionStyle(c: SchemeColors, face: Hsl): CSSProperties {
  return {
    background: tone(face),
    border: `1px solid ${tone(c.line)}`,
    borderRadius: RADIUS.section,
    padding: SECTION_PAD_PX,
    marginBottom: SECTION_GAP_PX,
    boxSizing: "border-box",
  };
}

/**
 * 種類のしるし（線の輪の中に記号）。**文字（滞在・移動・記録なし）に添えるだけ** ——
 * 色だけで種類を区別しない。輪と記号は同じ色で、面の 3 段すべてに 3:1 以上（NFR-23）。
 */
export function markStyle(color: Hsl): CSSProperties {
  return {
    flex: "none",
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: MARK_PX,
    height: MARK_PX,
    borderRadius: "50%",
    border: `1.5px solid ${tone(color)}`,
    color: tone(color),
    font: font(round1(TYPE.body * 0.78), 400, 1),
    boxSizing: "border-box",
  };
}

function round1(n: number): number {
  return Math.round(n * 10) / 10;
}
