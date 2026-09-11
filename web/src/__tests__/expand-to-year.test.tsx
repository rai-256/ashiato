// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 年ぶんは伸ばして見る（tasks 8.5c / 深掘り 第 7 回 Q28）。
 */
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { INITIAL_WEEKS, YEAR_WEEKS } from "../tokens";
import { days, source } from "./fixtures";

describe("伸ばして見る", () => {
  // Scenario: 1 年ぶんは伸ばして見る
  it("伸ばすと 1 年ぶん（53 週）が読める", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-01-04", 53 * 7, ["recorded"]));
    render(<CoverageGrid source={s} />);
    const grid = screen.getByTestId("grid-c01-location");
    expect(Number(grid.getAttribute("data-weeks"))).toBe(INITIAL_WEEKS);

    fireEvent.click(screen.getByRole("button", { name: "1 年ぶんを見る" }));
    expect(Number(grid.getAttribute("data-weeks"))).toBe(YEAR_WEEKS);
    expect(grid.querySelectorAll("[role='row']")).toHaveLength(YEAR_WEEKS);
  });

  it("戻すと直近だけになる", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-01-04", 53 * 7, ["recorded"]));
    render(<CoverageGrid source={s} />);
    fireEvent.click(screen.getByRole("button", { name: "1 年ぶんを見る" }));
    fireEvent.click(screen.getByRole("button", { name: "直近だけにする" }));
    expect(Number(screen.getByTestId("grid-c01-location").getAttribute("data-weeks"))).toBe(
      INITIAL_WEEKS,
    );
  });
});
