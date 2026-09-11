// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 退役したソースは**後ろに置き、既定で畳む**（tasks 15.6 / ST03 の R63）。
 *
 * ST03 の運用では退役は 1 本きりではなく増える。退役した格子が上に並ぶと
 * **Must の 5 本が 1 画面から押し出され**、第 8 回 Q30 で決めた
 * 「開いた直後に 2〜3 本、ひとスクロールで 5 本すべて」が成り立たなくなる。
 *
 * **消しはしない** —— 退役したことも稼働状況の一部で、開けば同じ格子が出る。
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { CoverageGrid } from "../CoverageGrid";
import { retiredLast, STATE_NAME, type SourceCoverage } from "../coverage";
import { achievement, days, fiveSources, source } from "./fixtures";

afterEach(() => {
  vi.unstubAllGlobals();
});

const retired = source(
  "c02-window-old",
  "PC のウィンドウ（旧）",
  days("2026-01-04", 371, ["recorded", "retired"]),
  "2026-05-10",
);

describe("退役したソース", () => {
  // Scenario: 退役したソースは後ろで畳まれている
  it("Must の 5 本より後ろに回り、既定で週の行を出さない", () => {
    // **並び順**: サーバが先頭で返しても後ろへ回る
    const ordered = retiredLast([retired, ...fiveSources("2026-01-04", 371)]);
    expect(ordered.at(-1)?.logical_source, "退役したソースが末尾にいない").toBe("c02-window-old");
    expect(ordered.filter((s) => s.retired_on === null)).toHaveLength(5);

    render(
      <div>
        {ordered.map((s) => (
          <CoverageGrid key={s.logical_source} source={s} />
        ))}
      </div>,
    );
    // **既定で週の行を出さない**（Must の 5 本を押し出さない）
    const grid = screen.getByTestId("grid-c02-window-old");
    expect(grid.getAttribute("data-weeks"), "退役したソースが既定で行を出している").toBe("0");
    // 生きている 5 本は出したまま
    expect(screen.getByTestId("grid-c01-location").getAttribute("data-weeks")).not.toBe("0");
  });

  it("開けば同じ格子が出る（消してはいない）", () => {
    render(<CoverageGrid source={retired} />);
    expect(screen.getByTestId("grid-c02-window-old").getAttribute("data-weeks")).toBe("0");
    fireEvent.click(screen.getByRole("button", { name: "退役したソースを見る" }));
    expect(
      Number(screen.getByTestId("grid-c02-window-old").getAttribute("data-weeks")),
      "開いても格子が出ない",
    ).toBeGreaterThan(0);
    // **退役した日が文字で読める**（色は意味の担い手にしない）
    expect(screen.getByTestId("retired-c02-window-old").textContent).toContain("2026-05-10");
  });

  // Scenario: 週を選ぶと 7 日ぶんが 8 状態の名前で出る
  it("⑧「退役」が週の詳細に名前で出る", () => {
    render(<CoverageGrid source={retired} />);
    fireEvent.click(screen.getByRole("button", { name: "退役したソースを見る" }));
    const week = document.querySelector("[data-week]") as HTMLElement;
    fireEvent.click(week);
    const detail = screen.getByTestId("week-detail");
    expect(detail.querySelector('[data-state="retired"]'), "⑧ が名前で出ていない").toBeTruthy();
    expect(detail.textContent).toContain(STATE_NAME.retired);
  });

  // Scenario: 退役したソースは後ろで畳まれている
  it("画面を開いたときにも後ろに回る（App 経由）", async () => {
    // **`retiredLast` を直接呼ばない**（review/code-r2.md の R4 / G5）——
    // 検査側で並べ替えてから `CoverageGrid` に渡していたときは、
    // `App.tsx` から `retiredLast` を外しても 43 件すべて緑のままだった。
    // Scenario の WHEN は「画面を開く」なので、画面から見る。
    const body: SourceCoverage[] = [retired, ...fiveSources("2026-01-04", 371)];
    vi.stubGlobal("fetch", (path: string) =>
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve(path.startsWith("/api/coverage?") ? body : achievement()),
      } as Response),
    );
    render(<App />);
    await waitFor(() => {
      expect(document.querySelectorAll("section[data-source]")).toHaveLength(6);
    });
    const order = [...document.querySelectorAll("section[data-source]")].map((s) =>
      s.getAttribute("data-source"),
    );
    expect(order.at(-1), "退役したソースが末尾に回っていない").toBe("c02-window-old");
    // **生きている 5 本の並びは定数の順のまま**（design D19。並べ替えが安定であること）
    expect(order.slice(0, 5)).toEqual([
      "c01-location",
      "c01-app-usage",
      "c01-photo",
      "c02-window",
      "c02-browser-history",
    ]);
  });

  it("欄そのものが返らないサーバでも、全部が退役にはならない", () => {
    // `retired_on` が `undefined` のとき `!== null` は真になり、
    // **5 本すべてが畳まれて格子が全部消える**（review/code-r2.md の M-1）
    const legacy = fiveSources("2026-01-04", 371).map((s) => {
      // **欄そのものを落とす**（古いサーバの応答の形）
      const rest: Record<string, unknown> = { ...s };
      delete rest.retired_on;
      return rest as unknown as SourceCoverage;
    });
    render(
      <div>
        {retiredLast(legacy).map((s) => (
          <CoverageGrid key={s.logical_source} source={s} />
        ))}
      </div>,
    );
    for (const s of legacy) {
      expect(
        screen.getByTestId(`grid-${s.logical_source}`).getAttribute("data-weeks"),
        `${s.logical_source} が退役扱いで畳まれている`,
      ).not.toBe("0");
    }
  });
});
