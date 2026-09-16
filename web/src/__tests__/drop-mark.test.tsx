// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 一部を破棄した日のセルの印（ST04 / tasks 4.1 / 深掘り Q3 / design D10）。
 *
 * **丸ごと覆わない破棄は状態を決めない**（ST02 の判定順）ので、印が無いと長い圏外の両端の日と
 * 上限を小さくして溢れさせた日の破棄が画面から消える。印は**色ではなく形**（右下を三角に欠く）。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CoverageGrid } from "../CoverageGrid";
import { contrastRatio, relativeLuminance } from "../contrast";
import { bandOf, dropMarkLightness, type DayCell, type DayState } from "../coverage";
import { BAND, HUE, SAT } from "../tokens";
import { days, source } from "./fixtures";

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

function withDrop(cell: DayCell, count: number): DayCell {
  return {
    ...cell,
    dropped_count: count,
    dropped_ranges: count > 0 ? [{ from: "10:00", to: "13:00", count }] : [],
  };
}

/** 2026-05-03（日）からの 1 週。`i` 日目だけ破棄を持たせる。 */
function weekWithDropOn(i: number, state: DayState, count: number) {
  const cells = days("2026-05-03", 7, ["recorded"]);
  cells[i] = withDrop({ ...cells[i], state }, count);
  return source("c01-location", "携帯端末の位置", cells);
}

const markOf = (day: string): Element | null =>
  document.querySelector(`[data-day="${day}"] [data-drop-mark]`);

describe("破棄の印", () => {
  // Scenario: 一部を破棄した日のセルに形の印が付く
  it("一部を破棄して記録が残った日は、記録ありの段で右下を三角に欠く", () => {
    render(<CoverageGrid source={weekWithDropOn(2, "recorded", 180)} />);
    const cell = document.querySelector('[data-day="2026-05-05"]') as HTMLElement;
    expect(cell.getAttribute("data-band")).toBe("recorded");
    const mark = markOf("2026-05-05") as HTMLElement;
    expect(mark, "印が無い").not.toBeNull();
    // **形で持たせる**: 右下に置いた、幅と高さの無い箱の枠線で描く三角
    expect(mark.style.position).toBe("absolute");
    expect(mark.style.right).toBe("0px");
    expect(mark.style.bottom).toBe("0px");
    expect(mark.style.width).toBe("0px");
    expect(mark.style.height).toBe("0px");
    expect(mark.style.borderStyle).toBe("solid");
    // 見えるのは下辺だけ（左・上・右は透明）—— 右下の三角
    expect(mark.style.borderTopColor).toBe("transparent");
    expect(mark.style.borderLeftColor).toBe("transparent");
    expect(mark.style.borderRightColor).toBe("transparent");
    expect(mark.style.borderBottomColor).not.toBe("transparent");
    expect(mark.getAttribute("aria-hidden")).toBe("true");
    // 印はその日だけ
    expect(document.querySelectorAll("[data-drop-mark]")).toHaveLength(1);
    // 画面にはそれ以外の破棄の表示が出ていない（件数の文字は週の詳細だけ）
    expect(screen.queryByText(/件を破棄/)).toBeNull();
  });

  // Scenario: 丸ごと覆う破棄の日には印が付かない
  it("丸ごと覆う破棄の日（破棄された期間）には印を付けない", () => {
    render(<CoverageGrid source={weekWithDropOn(2, "dropped", 1440)} />);
    expect(markOf("2026-05-05")).toBeNull();
    expect(document.querySelectorAll("[data-drop-mark]")).toHaveLength(0);
  });

  // Scenario: 破棄の無い日には印が付かない
  it("破棄の報告が無い日には印を付けない", () => {
    render(<CoverageGrid source={weekWithDropOn(2, "recorded", 0)} />);
    expect(document.querySelectorAll("[data-drop-mark]")).toHaveLength(0);
  });

  it("欄を返さない古いサーバでも落ちず、印を付けない", () => {
    const cells = days("2026-05-03", 7, ["recorded"]).map((c) => {
      const old: Partial<DayCell> = { ...c };
      delete old.dropped_count;
      delete old.dropped_ranges;
      return old as DayCell;
    });
    render(<CoverageGrid source={source("c01-location", "携帯端末の位置", cells)} />);
    expect(document.querySelectorAll("[data-drop-mark]")).toHaveLength(0);
  });

  // Scenario: 印はどの段の上でも 3:1 以上
  it("印とセルの相対輝度比が、3 段のどの上でも 3:1 以上", () => {
    for (const band of Object.keys(BAND) as (keyof typeof BAND)[]) {
      const r = contrastRatio(lum(dropMarkLightness(band)), lum(BAND[band]));
      expect(r, `${band} の上の印の比が ${r.toFixed(3)}`).toBeGreaterThanOrEqual(3);
    }
  });

  it("描いた印の色は、その段のために決めた明るさ（測る色と描く色をずらさない）", () => {
    const states: DayState[] = ["recorded", "alive_no_record", "outage"];
    for (const state of states) {
      const cells = days("2026-05-03", 7, ["recorded"]);
      cells[0] = withDrop({ ...cells[0], state }, 5);
      const { unmount } = render(<CoverageGrid source={source("c01-location", "携帯端末の位置", cells)} />);
      const mark = markOf("2026-05-03") as HTMLElement;
      const probe = document.createElement("span");
      probe.style.color = `hsl(${HUE} ${SAT}% ${dropMarkLightness(bandOf(state))}%)`;
      expect(mark.style.borderBottomColor, state).toBe(probe.style.color);
      unmount();
    }
  });
});
