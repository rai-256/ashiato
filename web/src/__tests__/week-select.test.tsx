// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 週を選ぶと 7 日ぶんが 7 状態の名前で文字で出る（tasks 8.4 / 深掘り 第 5 回 Q20 / Q21）。
 *
 * **格子は 3 段しか担わない**ので、7 状態の区別の担い手はこの文字。
 * ここが無いと、③④⑤⑥⑦ の区別が画面のどこにも出ない。
 */
import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { STATE_NAME, type DayState } from "../coverage";
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

describe("週を選ぶ", () => {
  // Scenario: 週を選ぶと 7 日ぶんが 7 状態の名前で出る
  it("7 日ぶんが 7 状態それぞれの名前で出る", () => {
    // 2026-05-03 は日曜。1 週ちょうどに 7 状態を 1 つずつ置く
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);

    expect(screen.queryByTestId("week-detail")).toBeNull();
    fireEvent.click(screen.getByRole("row", { name: "2026-05-03 の週" }));

    const detail = screen.getByTestId("week-detail");
    for (const state of ALL) {
      expect(
        within(detail).getByText(STATE_NAME[state]),
        `${state} の名前が出ていない`,
      ).toBeTruthy();
    }
  });

  it("格子で「それ以外」に畳まれた日も、どの状態だったか分かる", () => {
    const folded: DayState[] = [
      "alive_not_capturable",
      "stopped",
      "dropped",
      "outage",
      "before_start",
      "recorded",
      "alive_no_record",
    ];
    const s = source("c01-photo", "端末に保存された写真", days("2026-05-03", 7, folded));
    render(<CoverageGrid source={s} />);

    // 格子の側では 5 日ぶんが同じ段に畳まれている
    const cells = screen.getByTestId("grid-c01-photo").querySelectorAll('[data-band="other"]');
    expect(cells).toHaveLength(5);

    // 文字の側では 5 つが別々の名前で出る
    fireEvent.click(screen.getByRole("row", { name: "2026-05-03 の週" }));
    const detail = screen.getByTestId("week-detail");
    const names = [...detail.querySelectorAll("[data-state]")].map((e) => e.textContent);
    expect(new Set(names).size).toBe(7);
  });

  it("もう一度押すと閉じる", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);
    const row = screen.getByRole("row", { name: "2026-05-03 の週" });
    fireEvent.click(row);
    expect(screen.getByTestId("week-detail")).toBeTruthy();
    fireEvent.click(row);
    expect(screen.queryByTestId("week-detail")).toBeNull();
  });
});
