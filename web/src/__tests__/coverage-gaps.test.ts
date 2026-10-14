// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 取りこぼしの期間が週の文字に出る（ST14 / tasks 10.2 / design D23（仮））。
 * ここは文字の組み立て。本物の画面の文字は `web/e2e/coverage-gaps.spec.ts` が見る。
 */
import { describe, expect, it } from "vitest";
import { gapNotes, type DayCell, type GapSpan } from "../coverage";
import { days } from "./fixtures";

// 2026-09-01 08:00 +09:00 〜 2026-09-03 08:00 +09:00
const BEGIN = "2026-08-31T23:00:00Z";
const END = "2026-09-02T23:00:00Z";
const SPAN = "2026-09-01 08:00〜2026-09-03 08:00";

function cellWith(gaps: GapSpan[]): DayCell {
  return { ...days("2026-09-02", 1, ["alive_not_capturable"])[0], gaps };
}

describe("取りこぼしの文字", () => {
  // Scenario: 取得元に残っていない期間が週の文字に出る
  it("理由 retention は「取得元に残っていない（期間）」で出る", () => {
    expect(gapNotes(cellWith([{ reason: "retention", begin: BEGIN, end: END }]))).toEqual([
      `取得元に残っていない（${SPAN}）`,
    ]);
  });

  // Scenario: 時計のずれで取れていない期間は別の文字で出る
  it("理由 clock_skew_abandoned は「時計のずれで取れていない（期間）」で出て、取得元の文字は出ない", () => {
    const notes = gapNotes(cellWith([{ reason: "clock_skew_abandoned", begin: BEGIN, end: END }]));
    expect(notes).toEqual([`時計のずれで取れていない（${SPAN}）`]);
    expect(notes.join("")).not.toContain("取得元に残っていない");
  });

  // Scenario: 2 つの理由が同じ日に重なると 2 つとも出る
  it("理由の違う 2 つは畳まれず 1 つずつ出る", () => {
    const notes = gapNotes(
      cellWith([
        { reason: "retention", begin: BEGIN, end: END },
        { reason: "clock_skew_abandoned", begin: "2026-09-02T01:00:00Z", end: "2026-09-02T05:00:00Z" },
      ]),
    );
    expect(notes).toHaveLength(2);
    expect(notes[0]).toMatch(/^取得元に残っていない（/);
    expect(notes[1]).toBe("時計のずれで取れていない（2026-09-02 10:00〜2026-09-02 14:00）");
  });

  // Scenario: 知らない理由の取りこぼしは理由の文字を付けずに出る
  it("知らない理由は「取りこぼし（期間）」で、理由の文字を付けない", () => {
    const notes = gapNotes(cellWith([{ reason: "unknown_reason", begin: BEGIN, end: END }]));
    expect(notes).toEqual([`取りこぼし（${SPAN}）`]);
  });

  it("取りこぼしが無い日・欄を返さないサーバでは何も出ない", () => {
    expect(gapNotes(cellWith([]))).toEqual([]);
    const old = { ...cellWith([]) } as Partial<DayCell>;
    delete old.gaps;
    expect(gapNotes(old as DayCell)).toEqual([]);
  });
});
