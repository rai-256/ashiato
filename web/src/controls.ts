// SPDX-License-Identifier: AGPL-3.0-only
import type { CSSProperties } from "react";
import { controlStyle, primaryStyle } from "./surface";
import { SCHEMES, type Scheme } from "./tokens";

/**
 * 操作対象の見た目。**24 × 24 CSS px 以上**（NFR-19）。マスタ管理の個人属性と場所が共有する。
 * 形は `surface.ts`、色は `tokens.ts` からだけ引く（ここでは明暗を選ぶだけ）。
 */
export function control(scheme: Scheme): CSSProperties {
  return controlStyle(SCHEMES[scheme]);
}

/** 主ボタン（書く・積む）。**アクセントで塗る**（本人の決定 2026-10-12）。 */
export function primary(scheme: Scheme): CSSProperties {
  return primaryStyle(SCHEMES[scheme]);
}
