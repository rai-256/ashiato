// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 週を選ぶと 7 日ぶんが 8 状態の名前で文字で出る（tasks 8.4 / 深掘り 第 5 回 Q20 / Q21）。
 *
 * **格子は 3 段しか担わない**ので、8 状態の区別の担い手はこの文字。
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
  // Scenario: 週を選ぶと 7 日ぶんが 8 状態の名前で出る
  it("7 日ぶんが 8 状態それぞれの名前で出る", () => {
    // 2026-05-03 は日曜。1 週ちょうどに 8 状態を 1 つずつ置く
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);

    expect(screen.queryByTestId("week-detail")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));

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
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const detail = screen.getByTestId("week-detail");
    const names = [...detail.querySelectorAll("[data-state]")].map((e) => e.textContent);
    expect(new Set(names).size).toBe(7);
  });

  it("もう一度押すと閉じる", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);
    const row = screen.getByRole("button", { name: "2026-05-03 の週" });
    fireEvent.click(row);
    expect(screen.getByTestId("week-detail")).toBeTruthy();
    fireEvent.click(row);
    expect(screen.queryByTestId("week-detail")).toBeNull();
  });

  /**
   * **選択で面の明るさを変えない**（review/code.md の R34 / I5 / F12）。
   *
   * 選択中の背景を `surface1`(18%) にしていたときは、いちばん暗い段（9%）との比が
   * **1.422:1** に落ちて、**いちばん見たい週で格子がいちばん読めなくなっていた**。
   * 検査は `surface2` しか測っていなかったので緑のまま ——
   * `ui-direction` の UIR-13（測る色と描く色がずれる）と同じ型。
   */
  it("選択しても、セルが乗る面の明るさが変わらない", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 7, ALL));
    render(<CoverageGrid source={s} />);
    const row = screen.getByRole("button", { name: "2026-05-03 の週" }) as HTMLElement;
    const before = row.style.background;
    fireEvent.click(row);
    expect(row.style.background, "選択で面の明るさが変わっている").toBe(before);
    // 選択の印は輪郭で表す（段の色に触らない）
    expect(row.style.outline, "選択の印が無い").not.toBe("");
    expect(row.getAttribute("aria-pressed")).toBe("true");
  });

  /**
   * **畳み戻したら詳細も閉じる**（review/code.md の R37 / I11）。
   * `weeks` から選んだ週を探していたときは、伸ばして選んでから畳み戻すと
   * **行は消えるのに日付リストだけ残り**、閉じる手段が無くなった。
   */
  it("伸ばして選んだ週を畳み戻すと、詳細も閉じる", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-01-04", 53 * 7, ["recorded"]));
    render(<CoverageGrid source={s} />);
    fireEvent.click(screen.getByRole("button", { name: "1 年ぶんを見る" }));
    const rows = screen.getByTestId("grid-c01-location").querySelectorAll("[data-week]");
    fireEvent.click(rows[30] as HTMLElement);
    expect(screen.getByTestId("week-detail")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "直近だけにする" }));
    expect(
      screen.queryByTestId("week-detail"),
      "見えていない週の詳細が残っている",
    ).toBeNull();
  });
});