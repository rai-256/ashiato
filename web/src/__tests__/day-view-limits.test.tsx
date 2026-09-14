// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の一覧の下限（ST16 / tasks 6.2 / NFR-17〜19 / NFR-22 / NFR-23）。
 *
 * **jsdom は実寸も `:focus-visible` も計算しない。** ここが見るのは、
 * 値から計算したコントラスト比と、24 px 以上を要求する指定と、フォーカスの輪郭の規則が操作対象に掛かっていること。
 */
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DayView, FOCUS_ATTR, focusRule } from "../DayView";
import { contrastRatio, relativeLuminance } from "../contrast";
import { HUE, SAT, SCHEMES } from "../tokens";

/** NFR-18 / NFR-19 の数値を**リテラルで持つ**（トークンと一緒に動かない。ST02 の review R4）。 */
const NFR18_TEXT = 4.5;
const NFR19_MIN_PX = 24;
const NFR23_NON_TEXT = 3;

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

afterEach(() => {
  vi.unstubAllGlobals();
});

function stubScheme(light: boolean | null): void {
  vi.stubGlobal("fetch", () => new Promise<Response>(() => {}));
  if (light === null) {
    vi.stubGlobal("matchMedia", undefined);
    return;
  }
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: q.includes("light") ? light : !light,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

describe("一覧の下限", () => {
  // Scenario: 一覧の文字はライトでもダークでも 4.5:1 を下回らない
  it("文字が乗る面すべてで、ライトとダークの両方 4.5:1 以上", () => {
    for (const [scheme, c] of Object.entries(SCHEMES)) {
      for (const [face, bg] of [["ground", c.ground], ["surface1", c.surface1], ["surface2", c.surface2]] as const) {
        for (const [kind, fg] of [["text", c.text], ["muted", c.muted]] as const) {
          const r = contrastRatio(lum(fg), lum(bg));
          expect(r, `${scheme} の ${kind} が ${face} の上で ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(NFR18_TEXT);
        }
      }
    }
  });

  it("OS の明暗に追従し、取得できないときはダーク", () => {
    stubScheme(true);
    const { unmount } = render(<DayView date="2026-07-01" />);
    expect(screen.getByTestId("day-view").getAttribute("data-scheme")).toBe("light");
    unmount();
    stubScheme(false);
    const second = render(<DayView date="2026-07-01" />);
    expect(screen.getByTestId("day-view").getAttribute("data-scheme")).toBe("dark");
    second.unmount();
    stubScheme(null);
    render(<DayView date="2026-07-01" />);
    expect(screen.getByTestId("day-view").getAttribute("data-scheme")).toBe("dark");
  });

  // Scenario: 日を移る操作は 24 px を下回らない
  it("前の日・次の日・日付の指定・稼働状況へは 24 × 24 CSS px 以上を要求している", () => {
    stubScheme(false);
    render(<DayView date="2026-07-01" />);
    const targets = [
      screen.getByRole("button", { name: "前の日" }),
      screen.getByRole("button", { name: "次の日" }),
      screen.getByLabelText("日付を指定"),
      screen.getByRole("link", { name: "稼働状況へ" }),
    ];
    for (const t of targets) {
      expect(parseFloat(t.style.minHeight), `${t.textContent} の高さ`).toBeGreaterThanOrEqual(NFR19_MIN_PX);
      expect(parseFloat(t.style.minWidth), `${t.textContent} の幅`).toBeGreaterThanOrEqual(NFR19_MIN_PX);
    }
    // 操作できるものは上の 4 つだけ（数え漏れで緑にしない）
    expect(document.querySelectorAll("button, a, input")).toHaveLength(targets.length);
  });

  // Scenario: キーボードで移るとフォーカスの位置が見える
  it("操作対象すべてにフォーカスの輪郭の規則が掛かり、輪郭は地に対して 3:1 以上", () => {
    for (const light of [false, true]) {
      stubScheme(light);
      const { unmount } = render(<DayView date="2026-07-01" />);
      const scheme = light ? "light" : "dark";
      const rule = [...document.querySelectorAll("style")].map((s) => s.textContent ?? "").join("\n");
      expect(rule).toContain(focusRule(scheme));
      expect(rule).toMatch(new RegExp(`\\[${FOCUS_ATTR}\\]:focus-visible \\{ outline: 3px solid`));
      for (const el of document.querySelectorAll("button, a, input")) {
        expect(el.hasAttribute(FOCUS_ATTR), `${el.textContent} に輪郭の印が無い`).toBe(true);
        expect((el as HTMLElement).style.outline, `${el.textContent} が輪郭を消している`).toBe("");
      }
      const c = SCHEMES[scheme];
      const r = contrastRatio(lum(c.text), lum(c.ground));
      expect(r, `${scheme} の輪郭が地に対して ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
      unmount();
    }
  });
});
