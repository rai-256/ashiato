// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 個人属性の画面の下限（ST19 / tasks 4.4 / NFR-17〜19 / NFR-22 / NFR-23）。
 *
 * **jsdom は実寸も `:focus-visible` も計算しない。** ここが見るのは、
 * 値から計算したコントラスト比と、24 px 以上を要求する指定と、
 * フォーカスの輪郭の規則が操作対象に掛かっていることと、色の出所。
 *
 * 測り方は `day-view-limits.test.tsx` と同じ（ST16）。
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FOCUS_ATTR, focusRule } from "../DayView";
import { MasterView } from "../MasterView";
import masterSource from "../MasterView.tsx?raw";
import { contrastRatio, relativeLuminance } from "../contrast";
import { HUE, SAT, SCHEMES } from "../tokens";
import type { AttributesView, Claim, KindView } from "../attributes";

/** NFR-18 / NFR-19 / NFR-23 の数値を**リテラルで持つ**（トークンと一緒に動かない。ST02 の review R4）。 */
const NFR18_TEXT = 4.5;
const NFR19_MIN_PX = 24;
const NFR23_NON_TEXT = 3;

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

function claim(over: Partial<Claim> & Pick<Claim, "id">): Claim {
  return {
    value: "東京都 目黒区",
    valid_from: { precision: "month", date: "2019-10" },
    asserted_at: "2026-09-01T10:00:00+09:00",
    ingested_at: "2026-09-01T01:00:00Z",
    supersedes: null,
    superseded_by: null,
    note: "補足つき",
    ...over,
  };
}

function kind(over: Partial<KindView> & Pick<KindView, "id" | "name">): KindView {
  return { current: null, upcoming: [], claims: [], superseded: [], ...over };
}

/** いまの値・予定・積んだ主張・取り消された主張が全部あるカード。 */
function fullView(): AttributesView {
  const now = claim({ id: "c1" });
  const soon = claim({ id: "c2", value: "新居", valid_from: { precision: "day", date: "2026-12-01" } });
  const gone = claim({ id: "c0", value: "取り消された", superseded_by: "c1" });
  return {
    today: "2026-09-15",
    kinds: [kind({ id: "k", name: "住所", current: now, upcoming: [soon], claims: [soon, now], superseded: [gone] })],
  };
}

const ok = (body: unknown): Response =>
  ({ ok: true, status: 200, json: () => Promise.resolve(body) }) as Response;

function stubScheme(light: boolean | null): void {
  vi.stubGlobal("fetch", () => Promise.resolve(ok(fullView())));
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

async function draw(): Promise<void> {
  render(<MasterView />);
  await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("個人属性の画面の下限", () => {
  // Scenario: 個人属性の画面は文字のコントラストの下限を満たす
  it("文字が乗る面すべてで、ライトとダークの両方 4.5:1 以上", () => {
    for (const [scheme, c] of Object.entries(SCHEMES)) {
      for (const [face, bg] of [["ground", c.ground], ["surface1", c.surface1], ["surface2", c.surface2]] as const) {
        for (const [what, fg] of [["text", c.text], ["muted", c.muted]] as const) {
          const r = contrastRatio(lum(fg), lum(bg));
          expect(r, `${scheme} の ${what} が ${face} の上で ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(NFR18_TEXT);
        }
      }
    }
  });

  // Scenario: 個人属性の画面は OS の明暗に追従する
  it("OS の明暗に追従し、取得できないときはダーク", async () => {
    stubScheme(true);
    await draw();
    expect(screen.getByTestId("master-view").getAttribute("data-scheme")).toBe("light");

    vi.unstubAllGlobals();
    stubScheme(null);
    await draw();
    const views = screen.getAllByTestId("master-view");
    expect(views[views.length - 1].getAttribute("data-scheme")).toBe("dark");
  });

  // Scenario: 個人属性の画面は触れる対象の下限を満たす
  /** **数え漏らしで緑にしない** —— 画面にある操作対象を全部測る（「書く」を開いた状態も）。 */
  it("ボタン・主張の行・タブ・入力の選択肢が 24 × 24 CSS px 以上", async () => {
    stubScheme(false);
    await draw();
    // 「書く」を開いて、フォームの選択肢まで出す
    fireEvent.click(screen.getByRole("button", { name: "書く" }));
    fireEvent.click(screen.getByLabelText("前の書き込みが間違っていた"));

    const targets = [...document.querySelectorAll("button, a, input, select")];
    expect(targets.length, "測る対象が見つからない").toBeGreaterThan(5);
    for (const el of targets) {
      const s = (el as HTMLElement).style;
      const name = el.textContent ?? el.getAttribute("aria-label") ?? el.tagName;
      expect(parseFloat(s.minHeight), `${name} の高さ指定（${s.minHeight}）`).toBeGreaterThanOrEqual(NFR19_MIN_PX);
      expect(parseFloat(s.minWidth), `${name} の幅指定（${s.minWidth}）`).toBeGreaterThanOrEqual(NFR19_MIN_PX);
    }
  });

  // Scenario: 個人属性の画面はフォーカスの輪郭が見える
  it("操作対象すべてに輪郭の規則が掛かり、輪郭は地に対して 3:1 以上", async () => {
    for (const light of [false, true]) {
      stubScheme(light);
      const { unmount } = render(<MasterView />);
      await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
      fireEvent.click(screen.getByRole("button", { name: "書く" }));

      const scheme = light ? "light" : "dark";
      const rule = [...document.querySelectorAll("style")].map((s) => s.textContent ?? "").join("\n");
      expect(rule).toContain(focusRule(scheme));
      expect(rule).toMatch(new RegExp(`\\[${FOCUS_ATTR}\\]:focus-visible \\{ outline: 3px solid`));
      for (const el of document.querySelectorAll("button, a, input, select")) {
        const name = el.textContent ?? el.getAttribute("aria-label") ?? el.tagName;
        expect(el.hasAttribute(FOCUS_ATTR), `${name} に輪郭の印が無い`).toBe(true);
        expect((el as HTMLElement).style.outline, `${name} が輪郭を消している`).toBe("");
      }
      const c = SCHEMES[scheme];
      const r = contrastRatio(lum(c.text), lum(c.ground));
      expect(r, `${scheme} の輪郭が地に対して ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
      unmount();
      vi.unstubAllGlobals();
    }
  });

  // Scenario: 個人属性の画面は確定した色だけを使う
  /**
   * **色は `tokens.ts`（`ui-direction.md` の確定値）からだけ引く**（design D9）——
   * 画面ごとに色を直書きすると、ST20 / ST21 が別のトークン系を持つ。
   */
  it("`MasterView.tsx` に色の直書きが無い", () => {
    const literals = masterSource.match(/#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(/g) ?? [];
    expect(literals, `色が直書きされている: ${literals.join(" / ")}`).toHaveLength(0);
    // 色は `tone` と `SCHEMES` からだけ引いている
    expect(masterSource).toContain('from "./tokens"');
    expect(masterSource).toContain("tone(");
  });
});
