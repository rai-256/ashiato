// SPDX-License-Identifier: AGPL-3.0-only
/**
 * **ひとスクロールで 5 ソースすべてが見える**（深掘り 第 8 回 Q30 / tasks 13.2）。
 *
 * 第 7 回 Q28 は「開いた直後に 5 ソースがスクロールせずに同時に見える」で決着していたが、
 * **その根拠にした 600 px は `5 ソース × 5 行 × 24 px` でセルだけを積んだ勘定**だった ——
 * 見出しも、節の余白も、「1 年ぶんを見る」のボタンも、達成の表も数えていない。
 * 実際に宣言されている箱を積むと **約 1,491 px** で、640 px の画面には到底入らない。
 * 本人の答え（第 8 回 Q30）は「**同時に見える」を 1 スクロール以内に緩める**」。
 *
 * **ここでやっている勘定の作り方**（design D27「検査は定数と定数を突き合わせない」）:
 *
 * - 積むのは `element.style` に**実際に入っている値**。定数を読み直すのではない ——
 *   `SECTION_PAD_PX` を 40 にすれば、この検査が落ちる
 * - 突き合わせる相手は**固定の予算**（640 / 1,280 px）。両側が一緒に動く形にしない
 * - jsdom は実寸を測らないので、これは**指定の勘定**であって実寸ではない。
 *   実寸は `tasks.md` の「人間の確認待ち」が持つ（design D23）
 */
import { render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { ONE_SCROLL_PX, VIEWPORT_H_PX } from "../tokens";
import { achievement, fiveSources } from "./fixtures";

afterEach(() => {
  vi.unstubAllGlobals();
});

const px = (v: string): number => {
  const n = Number.parseFloat(v);
  return Number.isFinite(n) ? n : 0;
};

/**
 * その要素に効いている行の高さ。`font` の短縮記法（`600 15px/1.3 ...`）から引き、
 * 宣言が無ければ先祖をたどる（CSS の継承と同じ向き）。
 */
function lineHeight(el: HTMLElement): number {
  for (let e: HTMLElement | null = el; e !== null; e = e.parentElement) {
    const m = /(\d+(?:\.\d+)?)px\s*\/\s*(\d+(?:\.\d+)?)/.exec(e.style.font);
    if (m !== null) return Number(m[1]) * Number(m[2]);
  }
  return 0;
}

/** 直接の子（要素ではないもの）に文字があるか。あれば少なくとも 1 行ぶんの高さを取る。 */
function hasOwnText(el: HTMLElement): boolean {
  return [...el.childNodes].some(
    (n) => n.nodeType === Node.TEXT_NODE && (n.textContent ?? "").trim() !== "",
  );
}

/**
 * **宣言されている箱の高さ**を積む。
 *
 * 横に並ぶもの（`display: flex` で縦並びでないもの。週の帯の中のセル）は
 * 足さずに**いちばん高いもの**を取る。それ以外は上から下へ積む。
 */
function declaredHeight(el: HTMLElement): number {
  const s = el.style;
  const chrome =
    px(s.paddingTop) +
    px(s.paddingBottom) +
    px(s.borderTopWidth) +
    px(s.borderBottomWidth) +
    px(s.marginTop) +
    px(s.marginBottom);
  const kids = [...el.children].filter((c): c is HTMLElement => c instanceof HTMLElement);
  // **横に並ぶもの**: `display: flex` で縦並びでないもの（週の帯の中のセル）と、
  // 表の行（`<tr>` の中のセルは横に並ぶ。足すと 1 行が 4 行ぶんの高さになる）
  const row = (s.display === "flex" && s.flexDirection !== "column") || el.tagName === "TR";
  const stacked =
    kids.length === 0
      ? 0
      : row
        ? Math.max(...kids.map(declaredHeight))
        : kids.reduce((a, k) => a + declaredHeight(k), 0);
  const text = kids.length === 0 || hasOwnText(el) ? lineHeight(el) : 0;
  return chrome + Math.max(px(s.minHeight), stacked, text);
}

/** `root` の内容の上端から数えた、`target` の下端までの距離。 */
function bottomWithin(root: HTMLElement, target: HTMLElement): number {
  let y = 0;
  for (const child of [...root.children]) {
    if (!(child instanceof HTMLElement)) continue;
    if (child === target) return y + declaredHeight(child);
    if (child.contains(target)) {
      // その子の枠（上の余白・枠線・内側の余白）を足してから中へ降りる
      return (
        y +
        px(child.style.marginTop) +
        px(child.style.borderTopWidth) +
        px(child.style.paddingTop) +
        bottomWithin(child, target)
      );
    }
    y += declaredHeight(child);
  }
  throw new Error("その要素が見つからない");
}

/** 画面の上端から、その要素の下端までの高さ。 */
function bottomOf(main: HTMLElement, target: HTMLElement): number {
  return px(main.style.paddingTop) + bottomWithin(main, target);
}

async function renderPage(): Promise<HTMLElement> {
  // **いちばん高くなる形で測る** —— 合否が暫定のときは「確定まであと N 日」の 1 行が増える。
  // 収集開始から 365 日が経つまではこちらが常態なので、確定した形で測ると勘定が甘くなる。
  const body = {
    "/api/coverage": fiveSources("2026-01-04", 371),
    "/api/coverage/achievement": achievement({ confirmed: false, days_until_confirmed: 200 }),
  };
  vi.stubGlobal("fetch", (path: string) =>
    Promise.resolve({
      ok: true,
      json: () => Promise.resolve(path.startsWith("/api/coverage?") ? body["/api/coverage"] : body["/api/coverage/achievement"]),
    } as Response),
  );
  render(<App />);
  await waitFor(() => {
    expect(document.querySelectorAll("section[data-source]")).toHaveLength(5);
  });
  return document.querySelector("main") as HTMLElement;
}

describe("ひとスクロールの勘定", () => {
  // Scenario: ひとスクロールで 5 ソースすべてが見える
  it("5 ソースすべてが 2 画面ぶん以内に収まる", async () => {
    const main = await renderPage();
    const last = [...document.querySelectorAll("section[data-source]")].at(-1) as HTMLElement;
    const bottom = bottomOf(main, last);
    expect(
      bottom,
      `5 本目の下端が ${Math.round(bottom)} px で、ひとスクロール（${ONE_SCROLL_PX} px）に収まっていない`,
    ).toBeLessThanOrEqual(ONE_SCROLL_PX);
  });

  // Scenario: 開いた直後に 2 ソース以上の直近 1 か月が同時に見える
  it("開いた直後に 2 ソース以上の直近 4 週が 1 画面に収まる", async () => {
    const main = await renderPage();
    // **測るのは格子（週の行）の下端**。spec が言っているのは
    // 「2 ソース以上について**直近 4 週以上が**同時に見えている」で、
    // 「1 年ぶんを見る」のボタンまで見えていることは求めていない
    const grids = [...document.querySelectorAll('[data-role="grid"]')] as HTMLElement[];
    const visible = grids.filter((g) => bottomOf(main, g) <= VIEWPORT_H_PX).length;
    expect(
      visible,
      `1 画面（${VIEWPORT_H_PX} px）に直近 4 週が収まっているのが ${visible} 本しかない`,
    ).toBeGreaterThanOrEqual(2);
  });

  it("勘定が空振りしていない（余白を増やせば落ちる）", async () => {
    const main = await renderPage();
    const first = document.querySelector("section[data-source]") as HTMLElement;
    const before = declaredHeight(first);
    const was = px(first.style.paddingTop);
    first.style.paddingTop = `${was + 200}px`;
    expect(
      declaredHeight(first),
      "余白を 200 px 増やしても勘定が動いていない（宣言を読んでいない）",
    ).toBe(before + 200);
    expect(bottomOf(main, first)).toBeGreaterThan(before);
  });
});
