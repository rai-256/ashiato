// SPDX-License-Identifier: AGPL-3.0-only
/**
 * **格子のセルに実際に塗られた色**が状態と結びついていること
 * （review/code.md の R3 / F4）。
 *
 * `state-contrast.test.ts` が見るのは `BAND` という**定数**の相対輝度だけ、
 * `week-select.test.tsx` が見るのは `data-band` **属性**だけで、
 * **`background` に実際に入る値を読む検査がどこにも無かった** ——
 * 全セルを `tone(BAND.recorded)` で塗っても 26/26 緑だった（実測）。
 *
 * これは `ui-direction` の独立レビューが名指しした型そのもの
 * （UIR-13 / UIR-38: 測る色と実際に描く色がずれていて全状態の 23.4% が 4.5:1 未満）。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { bandOf, type DayState } from "../coverage";
import { BAND, tone } from "../tokens";
import { days, source } from "./fixtures";

const ALL: DayState[] = [
  "recorded",
  "alive_no_record",
  "alive_not_capturable",
  "stopped",
  "dropped",
  "outage",
  "before_start",
];

/** jsdom が色をどう正規化するかに依らず比べるための足場。 */
function normalize(color: string): string {
  const probe = document.createElement("div");
  probe.style.background = color;
  return probe.style.background;
}

describe("セルに塗られる色", () => {
  it("段ごとに違う色が実際に塗られている", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);
    const cells = [...screen.getByTestId("grid-c01-location").querySelectorAll("[data-day]")];
    const painted = new Map<string, Set<string>>();
    for (const cell of cells) {
      const day = cell.getAttribute("data-day");
      if (day === null || day === "") continue;
      const state = ALL[days("2026-05-03", 7, ALL).findIndex((d) => d.day === day)];
      const band = bandOf(state);
      const bg = (cell as HTMLElement).style.background;
      (painted.get(band) ?? painted.set(band, new Set()).get(band))?.add(bg);
    }
    // 3 段ぶんの色が出ていて、**段ごとに 1 色に決まっている**
    expect([...painted.keys()].sort()).toEqual(["alive_no_record", "other", "recorded"]);
    for (const [band, colors] of painted) {
      expect(colors.size, `${band} に複数の色が塗られている`).toBe(1);
      // jsdom は色を `rgb(...)` に正規化するので、**同じ経路で正規化した期待値**と比べる
      expect([...colors][0], `${band} の色がトークンと違う`).toBe(
        normalize(tone(BAND[band as keyof typeof BAND])),
      );
    }
    // **3 段が別々の色である**（1 色に潰れていない）
    const all = [...painted.values()].map((v) => [...v][0]);
    expect(new Set(all).size, "段が同じ色に潰れている").toBe(3);
  });

  it("日の無い枠には色を塗らない", () => {
    // 2026-05-06 は水曜。最初の週の日〜火は日が無い
    const s = source("c01-location", "携帯端末の位置", days("2026-05-06", 10, ["recorded"]));
    render(<CoverageGrid source={s} />);
    const empty = [...screen.getByTestId("grid-c01-location").querySelectorAll("[data-band='empty']")];
    expect(empty.length).toBeGreaterThan(0);
    for (const cell of empty) {
      expect((cell as HTMLElement).style.background).toBe("transparent");
    }
  });
});
