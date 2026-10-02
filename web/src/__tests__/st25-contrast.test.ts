// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 要約・中身・畳んだ行・取れていない時間の行・記録なしの事情の文字の対比（ST25 / tasks 8.3 / NFR-18）。
 * 組は画面が描くときに引く表（`TIMELINE_PAIRS`）から取る —— 測る色と描く色がずれない。
 */
import { describe, expect, it } from "vitest";
import { contrastRatio, relativeLuminance } from "../contrast";
import { TIMELINE_PAIRS } from "../timeline";
import { HUE, SAT, SCHEMES } from "../tokens";

/** NFR-18 の数値をリテラルで持つ（トークンと一緒に動かない）。 */
const NFR18_TEXT = 4.5;
const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

describe("1 日の画面の新しい文字の対比", () => {
  // Scenario: 要約と中身の文字はライトでもダークでも 4.5:1 を下回らない
  it("どの組もライトとダークの両方で 4.5:1 以上", () => {
    expect(Object.keys(TIMELINE_PAIRS).length).toBeGreaterThan(0);
    for (const [scheme, c] of Object.entries(SCHEMES)) {
      for (const [role, pair] of Object.entries(TIMELINE_PAIRS)) {
        const r = contrastRatio(lum(c[pair.fg]), lum(c[pair.bg]));
        expect(r, `${scheme} の ${role}（${pair.fg} / ${pair.bg}）が ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(NFR18_TEXT);
      }
    }
  });
});
