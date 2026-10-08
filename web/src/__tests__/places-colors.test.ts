// SPDX-License-Identifier: AGPL-3.0-only
/** 場所の画面の色の出所（ST21 / tasks 8.3 / design D12）。静的な検査で、`master-view-limits.test.tsx` と同じ網。 */
import { describe, expect, it } from "vitest";
import bandSource from "../PlaceBand.tsx?raw";
import formsSource from "../PlaceForms.tsx?raw";
import placesSource from "../PlacesView.tsx?raw";
import controlsSource from "../controls.ts?raw";
import modelSource from "../places.ts?raw";

/**
 * **場所の画面を描く全ファイル**（review/code.md R19。2 つだけ見ていたときは、帯の部品に `#ff0000` を書いても緑だった）。
 * 足したら、ここにも足す。
 */
const SOURCES = [
  ["PlacesView.tsx", placesSource],
  ["PlaceBand.tsx", bandSource],
  ["PlaceForms.tsx", formsSource],
  ["controls.ts", controlsSource],
  ["places.ts", modelSource],
] as const;

/** 色を持つ欄の値に出てよい、色でない語（`border: "none"` など）。これ以外の語は名前付き色とみなす */
const NOT_COLORS = new Set(["none", "transparent", "solid", "dashed", "dotted", "double", "inherit", "initial", "unset", "px", "em", "rem"]);

/** 色を持つ欄の、文字列の値（`"…"` と、`${…}` を除いた `` `…` ``）に出る名前付き色 */
function namedColors(src: string): string[] {
  const found: string[] = [];
  const re = /\b(?:background|color|outline|fill|stroke|border)\w*\s*:\s*(?:"([^"]*)"|`([^`]*)`)/g;
  for (const m of src.matchAll(re)) {
    const value = (m[1] ?? m[2] ?? "").replace(/\$\{[^}]*\}/g, " ");
    for (const word of value.match(/[a-zA-Z]+/g) ?? []) {
      if (!NOT_COLORS.has(word.toLowerCase())) found.push(`${m[0]} (${word})`);
    }
  }
  return found;
}

describe("places-colors", () => {
  // Scenario: 場所の画面は確定した色だけを使う
  /** **色は `tokens.ts`（`ui-direction.md` の確定値）からだけ引く**（design D12）。 */
  it("場所の画面のコードに色の直書きが無く、色は tokens から引いている", () => {
    for (const [name, src] of SOURCES) {
      const literals = src.match(/#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/g) ?? [];
      expect(literals, `${name} に色が直書きされている: ${literals.join(" / ")}`).toHaveLength(0);
      const named = namedColors(src);
      expect(named, `${name} に名前付き色が直書きされている: ${named.join(" / ")}`).toHaveLength(0);
    }
    expect(placesSource).toContain('from "./tokens"');
    expect(placesSource).toContain("tone(");
    expect(placesSource).not.toMatch(/\bstyle=\{\{[^}]*\bbackground:\s*"/);
  });

  it("名前付き色の判定は色でない語（none / transparent / solid）を色と数えず、色の名前は数える", () => {
    expect(namedColors('{ border: "none", background: "transparent" }')).toEqual([]);
    expect(namedColors("{ border: `1px solid ${tone(c.muted)}` }")).toEqual([]);
    expect(namedColors('{ color: "red" }')).toHaveLength(1);
    expect(namedColors("{ border: `1px solid red` }")).toHaveLength(1);
  });
});
