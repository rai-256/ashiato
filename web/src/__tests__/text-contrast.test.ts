// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 文字の対比（tasks 8.7 / NFR-18）。
 *
 * 掛かるのは**各格子に添えたソース名**と、**週を選んだときに出る状態名**。
 * **セルには掛からない** —— 本文でも文字画像でもないため（design D10）。
 */
import { describe, expect, it } from "vitest";
import { contrastRatio, relativeLuminance } from "../contrast";
import { HUE, SAT, SURFACE, TEXT } from "../tokens";

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

describe("文字の対比", () => {
  it("文字が乗るすべての面で 4.5:1 以上", () => {
    // **文字が実際に乗る面を列挙する。** 1 つだけ測って済ませると、
    // 別の面に乗った文字が割れていても気付かない（ui-direction の UIR-13 と同じ型）
    const surfaces = Object.entries(SURFACE);
    for (const [name, l] of surfaces) {
      for (const [kind, t] of Object.entries(TEXT)) {
        const r = contrastRatio(lum(t), lum(l));
        expect(r, `${kind} の文字が ${name} の上で ${r.toFixed(3)}:1`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("弱い文字（日付・単位）も 4.5:1 を割らない", () => {
    // 「弱い」は明るさの差であって、**読めなくてよいという意味ではない**
    const r = contrastRatio(lum(TEXT.muted), lum(SURFACE.surface2));
    expect(r).toBeGreaterThanOrEqual(4.5);
  });
});
