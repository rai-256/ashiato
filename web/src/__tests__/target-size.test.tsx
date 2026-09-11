// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 360 px 幅での操作対象（tasks 8.5 / NFR-19 / 深掘り 第 4 回 Q16, 第 6 回 Q25）。
 *
 * **セルは操作対象にしない。** 1 週 7 日を横に並べると 1 セルが 24 px に満たない幅に
 * なりうる。選ぶ単位を週にすれば、セルの大きさに関わらず 24 × 24 CSS px を満たせる。
 *
 * **jsdom は実際のレイアウトを計算しない。** ここが見るのは
 * 「**24 px 以上を要求する指定が、操作対象すべてに実際に付いているか**」と
 * 「**セルにイベントハンドラが付いていないか**」—— 実寸の目視は人間の確認待ちに残す。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { MIN_TARGET_PX } from "../tokens";
import { days, source } from "./fixtures";

describe("360 px 幅の操作対象", () => {
  // Scenario: 360 px 幅でも操作対象が 24 px を割らない
  it("操作できるものはすべて 24 × 24 CSS px 以上を要求している", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 28, ["recorded"]));
    const { container } = render(<CoverageGrid source={s} />);
    const targets = container.querySelectorAll("button, a, [role='button']");
    expect(targets.length).toBeGreaterThan(0);
    for (const t of targets) {
      const style = (t as HTMLElement).style;
      expect(parseFloat(style.minHeight), `${t.tagName} の高さ`).toBeGreaterThanOrEqual(MIN_TARGET_PX);
      expect(parseFloat(style.minWidth), `${t.tagName} の幅`).toBeGreaterThanOrEqual(MIN_TARGET_PX);
    }
  });

  it("格子のセルは操作対象になっていない", () => {
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 28, ["recorded"]));
    render(<CoverageGrid source={s} />);
    const cells = screen.getByTestId("grid-c01-location").querySelectorAll("[role='gridcell']");
    expect(cells.length).toBe(4 * 7);
    for (const cell of cells) {
      expect(cell.tagName, "セルがボタンになっている").toBe("SPAN");
      // **押せる印を持たない。** 持たせると 24 px 未満の操作対象が生まれる
      expect(cell.getAttribute("onclick")).toBeNull();
      expect(cell.getAttribute("tabindex")).toBeNull();
      expect((cell as HTMLElement).style.cursor).not.toBe("pointer");
    }
  });

  it("格子が横スクロールなしで収まる指定になっている", () => {
    // **縦長にしたので横は 7 日ぶんしか並ばない**（第 6 回 Q25）。
    // 固定幅を持たせると 360 px を超えうるので、帯は幅いっぱいを取る
    const s = source("c01-location", "携帯端末の位置", days("2026-05-03", 28, ["recorded"]));
    render(<CoverageGrid source={s} />);
    const row = screen.getByRole("row", { name: "2026-05-03 の週" }) as HTMLElement;
    expect(row.style.width).toBe("100%");
    // セルは 7 等分（固定幅にしない）
    const cell = row.querySelector("[role='gridcell']") as HTMLElement;
    expect(cell.style.width).toBe("");
    expect(cell.style.flex).not.toBe("");
  });
});
