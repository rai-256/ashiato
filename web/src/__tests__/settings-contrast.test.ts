// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 設定画面（ST33 / tasks 8.3）が使う色の対比（NFR-18 / NFR-23。ライト・ダーク）。
 *
 * 画面は `tokens.ts` の値だけを使う（`surface.ts` の部品経由）。描くのと同じ値から `contrast.ts` で計算する。
 * 画面の Scenario の印は 9.5 の e2e に置く。
 */
import { describe, expect, it } from "vitest";
import { toneContrast } from "../contrast";
import settingsSource from "../SettingsView.tsx?raw";
import { SCHEMES, type Scheme } from "../tokens";

const TEXT = 4.5;
const NON_TEXT = 3;
const SCHEME_NAMES = Object.keys(SCHEMES) as Scheme[];

describe("設定画面の色", () => {
  it("節の面（surface1）の上の本文・控えめ・アクセントの文字が 4.5:1 以上", () => {
    for (const s of SCHEME_NAMES) {
      const c = SCHEMES[s];
      for (const t of ["text", "muted", "accent"] as const) {
        const r = toneContrast(c[t], c.surface1);
        expect(r, `${s} の ${t} が surface1 の上で ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(TEXT);
      }
    }
  });

  it("地（ground）の上の「次の設定」の注記（muted）と戻るリンク（accent）が 4.5:1 以上", () => {
    for (const s of SCHEME_NAMES) {
      const c = SCHEMES[s];
      for (const t of ["muted", "accent"] as const) {
        expect(toneContrast(c[t], c.ground), `${s} の ${t}`).toBeGreaterThanOrEqual(TEXT);
      }
    }
  });

  it("入力欄・副ボタン（surface2）の文字が 4.5:1、枠（muted）が 3:1 以上", () => {
    for (const s of SCHEME_NAMES) {
      const c = SCHEMES[s];
      expect(toneContrast(c.text, c.surface2), `${s} の入力の文字`).toBeGreaterThanOrEqual(TEXT);
      expect(toneContrast(c.muted, c.surface1), `${s} の枠`).toBeGreaterThanOrEqual(NON_TEXT);
    }
  });

  it("主ボタン（書き出す）の塗りの上の文字が 4.5:1 以上", () => {
    for (const s of SCHEME_NAMES) {
      const c = SCHEMES[s];
      expect(toneContrast(c.onAccent, c.accentFill), `${s} の主ボタン`).toBeGreaterThanOrEqual(TEXT);
    }
  });

  it("SettingsView は色の直書きをしない（tokens の値だけ）", () => {
    expect(settingsSource).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(/);
  });
});
