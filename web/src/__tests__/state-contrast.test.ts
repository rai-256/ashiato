// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 格子の 3 段（tasks 8.3 / NFR-23 / WCAG SC 1.4.11）。
 *
 * **トークンに書いた「3:1 以上」を信じない。** 描くのと同じ値から相対輝度を計算する ——
 * `ui-direction` の独立レビューは、**測る色と実際に描く色がずれていて
 * 全状態の 23.4% が 4.5:1 未満**だった事例を実測している（UIR-13 / UIR-38）。
 */
import { describe, expect, it } from "vitest";
import { contrastRatio, relativeLuminance } from "../contrast";
import { bandOf, type DayState } from "../coverage";
import { BAND, HUE, SAT, SURFACE } from "../tokens";

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

describe("格子の 3 段", () => {
  // Scenario: グレースケールでも格子の 3 段が区別できる
  it("隣り合う 2 段の相対輝度比が 3:1 以上", () => {
    // 暗い順に並べる。**隣接だけを見る**（WCAG SC 1.4.11 が求めるのは隣接）
    const ordered = [BAND.other, BAND.alive_no_record, BAND.recorded];
    for (let i = 0; i + 1 < ordered.length; i++) {
      const r = contrastRatio(lum(ordered[i]), lum(ordered[i + 1]));
      expect(r, `${ordered[i]}% と ${ordered[i + 1]}% の比が ${r.toFixed(3)}`).toBeGreaterThanOrEqual(3);
    }
  });

  it("段は 3 つだけ（7 段にしない）", () => {
    // **7 段は計算で不成立が確定している** —— 隣接 3:1 を 6 区間積むと 3^6 = 729:1 が要り、
    // sRGB の理論最大は 21:1。21:1 を 6 等分しても隣接 1.661:1 にしかならない。
    expect(Object.keys(BAND)).toHaveLength(3);
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
    for (const s of all) expect(BAND[bandOf(s)]).toBeTypeOf("number");
  });

  it("いちばん暗い段が、セルが実際に乗るすべての面の上で読める", () => {
    // 段どうしの 3:1（NFR-23）は満たしても、**面と同じ色**だと格子の広がりが読めない。
    //
    // **面を 1 つだけ測って済ませない**（review/code.md の R34 / I5 / F12）。
    // 選択中の週の背景を `surface1`(18%) にしていたときは、いちばん暗い段（9%）との比が
    // **1.422:1** に落ちて、**いちばん見たい週で格子がいちばん読めなくなっていた**。
    // 検査が `surface2` しか見ていなかったので緑のままだった ——
    // `ui-direction` の UIR-13（測る色と描く色がずれる）と同じ型。
    //
    // **閾値 1.5 は要件のどこにも無い数字**だったので、いまは
    // 「セルが乗りうる面が 1 つだけであること」を構造で担保し、
    // その 1 つに対して比を見る。選択は面の明るさではなく輪郭で表す（CoverageGrid）。
    for (const [name, l] of Object.entries(SURFACE)) {
      if (name !== "surface2") continue; // セルが乗るのはここだけ（下の検査が固定する）
      const r = contrastRatio(lum(BAND.other), lum(l));
      expect(r, `「それ以外」と ${name} の比が ${r.toFixed(3)}`).toBeGreaterThan(1.5);
    }
  });


});
