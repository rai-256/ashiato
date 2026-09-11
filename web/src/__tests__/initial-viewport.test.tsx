// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 開いた直後に**各ソースが直近 4 週以上を出す**（tasks 8.5c / 深掘り 第 7 回 Q28）。
 *
 * **これが無いと完了の判定が 1 本目のソースにしか成立しない**（3 巡目 R4）——
 * 縦長の格子は 1 ソースで 53 行 × 24 px ≒ 1,300 px あり、2 本目以降の直近週は
 * それだけ下にある。「新しい週を上に置けば満たす」は 1 本目にしか成立していなかった。
 *
 * **「何本が 1 画面に収まるか」はここでは見ない**（第 8 回 Q30）——
 * 高さの勘定は `one-scroll.test.tsx` が、宣言された箱を DOM から積んで確かめる。
 * この検査が見るのは「各ソースが最初から 1 年ぶんを出していないこと」だけ。
 *
 * **`5 ソース × 5 行 × 24 px ≒ 600 px` という勘定はここから消した** ——
 * 定数どうしを突き合わせていただけで（design D27）、実際の高さを 1 px も見ていなかった。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { FIVE, fiveSources } from "./fixtures";

describe("開いた直後", () => {
  it("5 ソースすべてが直近 4 週以上を出す", () => {
    const sources = fiveSources("2026-01-04", 371);
    render(
      <div>
        {sources.map((s) => (
          <CoverageGrid key={s.logical_source} source={s} />
        ))}
      </div>,
    );
    for (const [id] of FIVE) {
      const grid = screen.getByTestId(`grid-${id}`);
      const shown = Number(grid.getAttribute("data-weeks"));
      expect(shown, `${id} が ${shown} 週しか出していない`).toBeGreaterThanOrEqual(4);
      // **定数と突き合わせない。** `INITIAL_WEEKS` を 53 にしても通ってしまい、
      // 「最初から 1 年ぶん出している」という当の回帰を見逃す（実測）
      expect(shown, `${id} が最初から 1 年ぶん出している`).toBeLessThanOrEqual(6);
    }
  });

  // Scenario: ソースごとに格子が分かれる
  it("ソースごとに格子が分かれ、それぞれにソース名の文字がある", () => {
    const sources = fiveSources("2026-01-04", 371);
    render(
      <div>
        {sources.map((s) => (
          <CoverageGrid key={s.logical_source} source={s} />
        ))}
      </div>,
    );
    for (const [, name] of FIVE) {
      // **色ではなく文字**がソースの区別を担う（ui-direction の宿題 1 / 第 4 回 Q15）
      expect(screen.getByRole("heading", { name }), `${name} の文字が無い`).toBeTruthy();
    }
    expect(screen.getAllByRole("heading")).toHaveLength(5);
  });
});
