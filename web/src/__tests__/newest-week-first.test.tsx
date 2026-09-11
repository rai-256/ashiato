// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 新しい週が上（tasks 8.5b / 深掘り 第 6 回 Q25）。
 *
 * 開いた直後に出る直近 4〜5 週が**新しい側**になるようにするため。
 * **「53 週が並んでいる」ことは開いた直後には求めない**（8.5c を参照）。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { foldIntoWeeks } from "../coverage";
import { days, source } from "./fixtures";

describe("週の並び", () => {
  // Scenario: 格子は 1 行 1 週で、直近の週が一番上に来る
  it("1 行が 1 週で、新しい週から古い週へ上から下に並ぶ", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 35, ["recorded"]));
    render(<CoverageGrid source={s} />);
    const rows = [...screen.getByTestId("grid-c01-location").querySelectorAll("[role='row']")];
    const starts = rows.map((r) => r.getAttribute("data-week"));
    expect(starts).toEqual([...starts].sort().reverse());
    // 1 行は 7 日ぶん
    for (const r of rows) expect(r.querySelectorAll("[role='gridcell']")).toHaveLength(7);
  });

  it("直近の週が一番上にある", () => {
    const cells = days("2026-05-03", 35, ["recorded"]);
    const last = cells[cells.length - 1].day;
    const s = source("c01-location", "携帯端末の位置", cells);
    render(<CoverageGrid source={s} />);
    const first = screen.getByTestId("grid-c01-location").querySelector("[role='row']");
    const daysInRow = [...(first?.querySelectorAll("[data-day]") ?? [])].map((e) =>
      e.getAttribute("data-day"),
    );
    expect(daysInRow).toContain(last);
  });

  it("端の週も 7 つの枠を保つ（格子の形が崩れない）", () => {
    // 2026-05-06 は水曜。最初の週は日〜火が無い
    const s = source("c01-location", "携帯端末の位置", days("2026-05-06", 10, ["recorded"]));
    const weeks = foldIntoWeeks(s.days);
    for (const w of weeks) expect(w.days).toHaveLength(7);
    render(<CoverageGrid source={s} />);
    const rows = [...screen.getByTestId("grid-c01-location").querySelectorAll("[role='row']")];
    for (const r of rows) expect(r.querySelectorAll("[role='gridcell']")).toHaveLength(7);
  });
});
