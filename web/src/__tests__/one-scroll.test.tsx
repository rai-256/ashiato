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
import { retiredLast, type SourceCoverage } from "../coverage";
import { achievement, days, fiveSources, source } from "./fixtures";
import { bottomOf, declaredHeight, px } from "./layout";
import { ARCHIVE_BOX_MAX_PX } from "../LatestArchive";

/**
 * **予算から除くのは「箱の宣言の高さ」だが、除ける量には上限がある**
 * （`collection-coverage` の予算の文 / 第 2 回 Q9）。
 *
 * `ONE_SCROLL_PX + 160` を予算にしていたときは、箱が 100 px しか無くても
 * 160 px ぶん甘くなっていた。逆に箱が 200 px に伸びたら、除けるのは 160 px まで。
 */
function excludedBoxPx(): number {
  const box = document.querySelector('[data-testid="latest-archive"]');
  if (!(box instanceof HTMLElement)) return 0;
  return Math.min(declaredHeight(box), ARCHIVE_BOX_MAX_PX);
}

afterEach(() => {
  vi.unstubAllGlobals();
});

async function renderPage(extra: SourceCoverage[] = []): Promise<HTMLElement> {
  // **いちばん高くなる形で測る** —— 合否が暫定のときは「確定まであと N 日」の 1 行が増える。
  // 収集開始から 365 日が経つまではこちらが常態なので、確定した形で測ると勘定が甘くなる。
  const body = {
    // **退役を先頭で返す**（サーバは並び順を約束しない）。画面が末尾へ回さなければ
    // Must の 5 本が押し出されて予算を超える —— そこが R63 の眼目
    "/api/coverage": [...extra, ...fiveSources("2026-01-04", 371)],
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
    expect(document.querySelectorAll("section[data-source]")).toHaveLength(5 + extra.length);
  });
  return document.querySelector("main") as HTMLElement;
}

/** Must の 5 本の節（退役したものは末尾に回るので、勘定の対象から外す）。 */
function mustSections(): HTMLElement[] {
  return [...document.querySelectorAll("section[data-source]")].filter(
    (s) => (s.getAttribute("data-retired") ?? "") === "",
  ) as HTMLElement[];
}

describe("ひとスクロールの勘定", () => {
  // Scenario: ひとスクロールで 5 ソースすべてが見える
  it("5 ソースすべてが 2 画面ぶん以内に収まる", async () => {
    const main = await renderPage();
    const last = mustSections().at(-1) as HTMLElement;
    const bottom = bottomOf(main, last);
    expect(
      bottom,
      `5 本目の下端が ${Math.round(bottom)} px で、ひとスクロール（${ONE_SCROLL_PX} px）に収まっていない`,
    ).toBeLessThanOrEqual(ONE_SCROLL_PX + excludedBoxPx());
  });

  // Scenario: 開いた直後に 2 ソース以上の直近 1 か月が同時に見える
  it("開いた直後に 2 ソース以上の直近 4 週が 1 画面に収まる", async () => {
    const main = await renderPage();
    // **測るのは格子（週の行）の下端**。spec が言っているのは
    // 「2 ソース以上について**直近 4 週以上が**同時に見えている」で、
    // 「1 年ぶんを見る」のボタンまで見えていることは求めていない
    const grids = [...document.querySelectorAll('[data-role="grid"]')] as HTMLElement[];
    const visible = grids.filter((g) => bottomOf(main, g) <= VIEWPORT_H_PX + excludedBoxPx()).length;
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

  // Scenario: ひとスクロールで 5 ソースすべてが見える
  it("退役したソースが増えても、Must の 5 本はひとスクロール以内に残る", async () => {
    // **R63 が想定したのはこの形**（退役は 1 本きりではなく増える）。
    // 退役が上に並ぶと Must の 5 本が押し出されるので、末尾へ回して畳んである。
    const retired = [1, 2, 3].map((i) =>
      source(`c02-window-old${i}`, `PC のウィンドウ（旧${i}）`, days("2026-01-04", 371, ["recorded"]), "2026-05-10"),
    );
    const main = await renderPage(retired);
    expect(retiredLast(retired).length, "退役の並べ替えが効いていない").toBe(3);
    const last = mustSections().at(-1) as HTMLElement;
    const bottom = bottomOf(main, last);
    expect(
      bottom,
      `退役 3 本を足すと Must の 5 本目が ${Math.round(bottom)} px まで下がった`,
    ).toBeLessThanOrEqual(ONE_SCROLL_PX + excludedBoxPx());
  });
});
