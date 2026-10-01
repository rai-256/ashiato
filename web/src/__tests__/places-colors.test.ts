// SPDX-License-Identifier: AGPL-3.0-only
/** 場所の画面の色の出所（ST21 / tasks 8.3 / design D12）。静的な検査で、`master-view-limits.test.tsx` と同じ網。 */
import { describe, expect, it } from "vitest";
import placesSource from "../PlacesView.tsx?raw";
import modelSource from "../places.ts?raw";

describe("places-colors", () => {
  // Scenario: 場所の画面は確定した色だけを使う
  /** **色は `tokens.ts`（`ui-direction.md` の確定値）からだけ引く**（design D12）。 */
  it("場所の画面のコードに色の直書きが無く、色は tokens から引いている", () => {
    for (const [name, src] of [["PlacesView.tsx", placesSource], ["places.ts", modelSource]] as const) {
      const literals = src.match(/#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/g) ?? [];
      expect(literals, `${name} に色が直書きされている: ${literals.join(" / ")}`).toHaveLength(0);
      const named = src.match(/\b(?:background|color|outline|fill|stroke|border)\s*:\s*"(?!transparent")[a-zA-Z]+"/g) ?? [];
      expect(named, `${name} に名前付き色が直書きされている: ${named.join(" / ")}`).toHaveLength(0);
    }
    expect(placesSource).toContain('from "./tokens"');
    expect(placesSource).toContain("tone(");
    expect(placesSource).not.toMatch(/\bstyle=\{\{[^}]*\bbackground:\s*"/);
  });
});
