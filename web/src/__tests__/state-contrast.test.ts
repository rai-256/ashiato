// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 格子の 3 段（tasks 8.3 / NFR-23 / WCAG SC 1.4.11）と、状態を表すしるし（NFR-23）。ライト・ダークの両方。
 *
 * **トークンに書いた「3:1 以上」を信じない。** 描くのと同じ値から相対輝度を計算する ——
 * `ui-direction` の独立レビューは、**測る色と実際に描く色がずれていて
 * 全状態の 23.4% が 4.5:1 未満**だった事例を実測している（UIR-13 / UIR-38）。
 */
import { describe, expect, it } from "vitest";
import { contrastRatio, relativeLuminance, toneContrast } from "../contrast";
import { bandOf, type DayState } from "../coverage";
import { BAND_NAMES, SCHEMES, type Scheme } from "../tokens";

/** NFR-23 の数値を**リテラルで持つ**（トークンと一緒に動かない）。 */
const NFR23_NON_TEXT = 3;
const SCHEME_NAMES = Object.keys(SCHEMES) as Scheme[];

describe("格子の 3 段", () => {
  // Scenario: グレースケールでも格子の 3 段が区別できる
  it("隣り合う 2 段の相対輝度比が 3:1 以上（ライト・ダーク）", () => {
    for (const scheme of SCHEME_NAMES) {
      const cell = SCHEMES[scheme].cell;
      // 段の並び（それ以外 → 動いていた → 記録あり）。**隣接だけを見る**（WCAG SC 1.4.11 が求めるのは隣接）
      const ordered = [cell.other, cell.alive_no_record, cell.recorded];
      for (let i = 0; i + 1 < ordered.length; i++) {
        const r = toneContrast(ordered[i], ordered[i + 1]);
        expect(r, `${scheme} の ${BAND_NAMES[i]} と ${BAND_NAMES[i + 1]} の比が ${r.toFixed(3)}`).toBeGreaterThanOrEqual(
          NFR23_NON_TEXT,
        );
      }
    }
  });

  it("段は 3 つだけ（7 段にしない）", () => {
    // **7 段は計算で不成立が確定している** —— 隣接 3:1 を 6 区間積むと 3^6 = 729:1 が要り、
    // sRGB の理論最大は 21:1。21:1 を 6 等分しても隣接 1.661:1 にしかならない。
    expect(BAND_NAMES).toHaveLength(3);
    for (const scheme of SCHEME_NAMES) expect(Object.keys(SCHEMES[scheme].cell)).toHaveLength(3);
    const seven = 3 ** 6;
    const srgbMax = contrastRatio(relativeLuminance(0, 0, 100), relativeLuminance(0, 0, 0));
    expect(srgbMax).toBeLessThan(seven);
    expect(srgbMax).toBeCloseTo(21, 0);
  });

  it("8 状態がこの 3 段に畳まれる", () => {
    const all: DayState[] = [
      "recorded",
      "alive_no_record",
      "alive_not_capturable",
      "stopped",
      "dropped",
      "outage",
      "before_start",
    ];
    expect(all.map(bandOf)).toEqual([
      "recorded",
      "alive_no_record",
      "other",
      "other",
      "other",
      "other",
      "other",
    ]);
    // **畳んだ先が 3 段のどれかであること**（新しい状態を足しても段が増えない）
    for (const s of all) expect(BAND_NAMES).toContain(bandOf(s));
  });

  it("「それ以外」のセルの広がりが、縁で読める（縁がセルにも節の面にも 3:1 以上）", () => {
    // 段どうしの 3:1（NFR-23）は満たしても、**面と同じ色**だと格子の広がりが読めない。
    //
    // **面を 1 つだけ測って済ませない**（review/code.md の R34 / I5 / F12）。
    // 2026-10-12 の表面では、ライトの「それ以外」（明るさ 95）が節の面（91）とほぼ同じになる。
    // だから「それ以外」のセルには**控えめな文字の色の 1 px の縁**を付け（`CoverageGrid`）、
    // その縁がセルの色にも、セルが乗る唯一の面（`surface2`）にも 3:1 以上あることを見る
    // （playground の「格子 その他の縁」と同じ組）。選択は面の明るさではなく輪郭で表す（CoverageGrid）。
    for (const scheme of SCHEME_NAMES) {
      const c = SCHEMES[scheme];
      const onCell = toneContrast(c.muted, c.cell.other);
      expect(onCell, `${scheme} の縁と「それ以外」の比が ${onCell.toFixed(3)}`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
      const onFace = toneContrast(c.muted, c.surface2);
      expect(onFace, `${scheme} の縁と節の面の比が ${onFace.toFixed(3)}`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
    }
  });
});

describe("状態を表すしるしとフォーカスの輪郭", () => {
  it("種類のしるし（控えめな文字の色・移動は差し色）が面の 3 段すべてに 3:1 以上", () => {
    // しるしは**文字に添えるだけ**（色だけで種類を区別しない）。それでも見えないしるしは置かない
    for (const scheme of SCHEME_NAMES) {
      const c = SCHEMES[scheme];
      for (const face of ["ground", "surface1", "surface2"] as const) {
        for (const mark of ["muted", "spark"] as const) {
          const r = toneContrast(c[mark], c[face]);
          expect(r, `${scheme} の ${mark} のしるしが ${face} の上で ${r.toFixed(3)}`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
        }
      }
    }
  });

  it("フォーカスの輪郭（本文の色）が面の 3 段すべてに 3:1 以上（NFR-22）", () => {
    for (const scheme of SCHEME_NAMES) {
      const c = SCHEMES[scheme];
      for (const face of ["ground", "surface1", "surface2"] as const) {
        const r = toneContrast(c.text, c[face]);
        expect(r, `${scheme} の輪郭が ${face} の上で ${r.toFixed(3)}`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
      }
    }
  });
});
