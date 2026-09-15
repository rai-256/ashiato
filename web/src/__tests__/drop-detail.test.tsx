// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 週の詳細に破棄の件数と時刻を文字で出す（ST04 / tasks 4.2 / 深掘り Q3 / design D10）。
 *
 * **期間と件数は週の詳細の文字だけ**。ソースの見出しの下の一覧は作らない ——
 * 一覧の行を足すと開いた直後の高さが伸び、ひとスクロールの予算（`one-scroll.test.tsx`）に効く。
 */
import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { type DayCell } from "../coverage";
import { days, source } from "./fixtures";

function sourceWith(i: number, patch: Partial<DayCell>) {
  const cells = days("2026-05-03", 7, ["recorded"]);
  cells[i] = { ...cells[i], ...patch };
  return source("c01-location", "携帯端末の位置", cells);
}

const rowOf = (day: string): HTMLElement => {
  const detail = screen.getByTestId("week-detail");
  const dt = within(detail).getByText(day);
  return dt.parentElement as HTMLElement;
};

describe("週の詳細の破棄", () => {
  // Scenario: 週を選ぶと破棄の件数と時刻が文字で出る
  it("一部を破棄した日の行に、状態名と「うち 180 件を破棄（10:00〜13:00）」が出る", () => {
    render(
      <CoverageGrid
        source={sourceWith(2, {
          state: "recorded",
          dropped_count: 180,
          dropped_ranges: [{ from: "10:00", to: "13:00", count: 180 }],
        })}
      />,
    );
    // 選ぶ前は文字が無い
    expect(screen.queryByText(/件を破棄/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const row = rowOf("2026-05-05");
    expect(row.textContent).toContain("記録あり");
    expect(row.textContent).toContain("うち 180 件を破棄（10:00〜13:00）");
    // 破棄の無い日の行には出ない
    expect(rowOf("2026-05-04").textContent).not.toContain("破棄");
  });

  it("1 日に 2 つの区間があれば、区間ごとに出る", () => {
    render(
      <CoverageGrid
        source={sourceWith(0, {
          state: "recorded",
          dropped_count: 7,
          dropped_ranges: [
            { from: "00:00", to: "03:00", count: 5 },
            { from: "22:00", to: "24:00", count: 2 },
          ],
        })}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const text = rowOf("2026-05-03").textContent ?? "";
    expect(text).toContain("うち 5 件を破棄（00:00〜03:00）");
    expect(text).toContain("うち 2 件を破棄（22:00〜24:00）");
  });

  // Scenario: 件数を持たない区間は件数を添えずに時刻だけが出る
  it("件数 0 の区間は「うち 0 件」と出さず、時刻だけを出す", () => {
    render(
      <CoverageGrid
        source={sourceWith(2, {
          state: "recorded",
          dropped_count: 2,
          dropped_ranges: [
            { from: "10:00", to: "10:01", count: 2 },
            { from: "10:30", to: "10:31", count: 0 },
          ],
        })}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const text = rowOf("2026-05-05").textContent ?? "";
    expect(text).toContain("うち 2 件を破棄（10:00〜10:01）");
    expect(text).toContain("破棄（10:30〜10:31）");
    expect(text).not.toContain("うち 0 件");
  });

  // Scenario: 丸ごと覆う破棄の日は件数が添えられる
  it("丸ごと覆う破棄の日は「破棄された期間」と件数 1,440 が出る", () => {
    render(
      <CoverageGrid
        source={sourceWith(2, {
          state: "dropped",
          event_count: 0,
          dropped_count: 1440,
          dropped_ranges: [{ from: "00:00", to: "24:00", count: 1440 }],
        })}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const text = rowOf("2026-05-05").textContent ?? "";
    expect(text).toContain("破棄された期間");
    expect(text).toContain("1,440 件");
    // 丸ごとの日に「うち」は付けない（その日の全部が破棄）
    expect(text).not.toContain("うち");
  });

  // Scenario: 破棄の一覧は出ない
  it("格子と週の詳細のほかに、破棄を並べた一覧が無い", () => {
    const cells = days("2026-05-03", 7, ["recorded"]).map((c) => ({
      ...c,
      dropped_count: 60,
      dropped_ranges: [{ from: "10:00", to: "11:00", count: 60 }],
    }));
    const { container } = render(<CoverageGrid source={source("c01-location", "携帯端末の位置", cells)} />);
    // 開いた直後: 破棄の文字は 1 つも出ていない（印だけ）
    expect(container.textContent).not.toContain("破棄");
    expect(container.querySelectorAll("ul, ol, table")).toHaveLength(0);
    // 週を選んでも、破棄の文字は週の詳細の中にだけある
    fireEvent.click(screen.getByRole("button", { name: "2026-05-03 の週" }));
    const detail = screen.getByTestId("week-detail");
    const all = [...container.querySelectorAll("*")].filter(
      (e) => e.children.length === 0 && (e.textContent ?? "").includes("破棄"),
    );
    expect(all.length).toBeGreaterThan(0);
    for (const e of all) expect(detail.contains(e), "週の詳細の外に破棄の文字がある").toBe(true);
    expect(container.querySelectorAll("ul, ol, table")).toHaveLength(0);
  });
});
