// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 個人属性の画面（ST19 / tasks 4.2 / design D9 / D13）。
 *
 * **構造は本人が proto で決めた**（Q2 の逐語。`deep.md`）—— 種類ごとのカードに
 * **積んだ主張を常に全部**・「いつから」の新しい順・**書いた日時は押したときだけ**・
 * 訂正で取り消した主張は畳む・カードごとに「書く」1 つ。
 * **画面が長い（10 年後で 4.3 画面）のは本人が承知で選んだ。畳む形に戻さない。**
 *
 * **応答を固定して描画を見る。** 何を返すかは Rust の結合テストが見る。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import type { AttributesView, Claim, KindView } from "../attributes";

const ok = (body: unknown): Response =>
  ({ ok: true, status: 200, json: () => Promise.resolve(body) }) as Response;

function claim(over: Partial<Claim> & Pick<Claim, "id">): Claim {
  return {
    value: "東京都 目黒区",
    valid_from: { precision: "month", date: "2019-10" },
    asserted_at: "2026-09-01T10:00:00+09:00",
    ingested_at: "2026-09-01T01:00:00Z",
    supersedes: null,
    superseded_by: null,
    note: null,
    ...over,
  };
}

function kind(over: Partial<KindView> & Pick<KindView, "id" | "name">): KindView {
  return { current: null, upcoming: [], claims: [], superseded: [], ...over };
}

function view(kinds: KindView[]): AttributesView {
  return { today: "2026-09-15", kinds };
}

/** 住所に主張が 8 件ある状態（spec の Scenario と同じ数）。 */
function eightClaims(): AttributesView {
  const claims = Array.from({ length: 8 }, (_, i) =>
    claim({
      id: `c${i}`,
      value: `住所 ${i}`,
      valid_from: { precision: "year", date: String(2026 - i) },
      asserted_at: `2026-0${(i % 9) + 1}-01T10:00:00+09:00`,
    }),
  );
  return view([kind({ id: "k-address", name: "住所", current: claims[0], claims })]);
}

let calls: { path: string; init?: RequestInit }[] = [];

function serve(body: AttributesView | number): void {
  calls = [];
  vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
    calls.push({ path, init });
    if (typeof body === "number") {
      return Promise.resolve({ ok: false, status: body, json: () => Promise.resolve([]) } as Response);
    }
    return Promise.resolve(ok(body));
  });
}

beforeEach(() => {
  window.location.hash = "#/master";
});
afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

/** 画面を開いて読み出しが終わるまで待つ。 */
async function open(body: AttributesView | number): Promise<void> {
  serve(body);
  render(<Root />);
  await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
}

function card(name: string): HTMLElement {
  const found = screen
    .getAllByTestId("kind-card")
    .find((el) => within(el).queryByRole("button", { name }) !== null);
  if (found === undefined) throw new Error(`「${name}」のカードが無い`);
  return found;
}

describe("個人属性の画面", () => {
  // Scenario: 種類ごとのカードにいまの値と積んだ主張が全部出る
  it("**積んだ主張を常に全部**、押さずに見せる（本人が 10 年後の量で選んだ側）", async () => {
    await open(eightClaims());
    const c = card("住所");
    expect(within(c).getByTestId("current-value").textContent).toBe("住所 0");
    const rows = within(c).getAllByTestId("claim-row");
    expect(rows, "積んだ主張が全部出ていない（畳む形に戻っている）").toHaveLength(8);
    for (let i = 0; i < 8; i++) {
      expect(rows[i].textContent, `${i} 件目の値が押さずに見えない`).toContain(`住所 ${i}`);
      expect(rows[i].textContent, `${i} 件目の「いつから」が押さずに見えない`).toContain(`${2026 - i} 年から`);
    }
  });

  // Scenario: 積んだ主張はいつからの新しい順に出る
  /** **並べ替えはサーバが持つ**（画面は受け取った順に描く）—— 規則が 2 か所に割れない。 */
  it("サーバが返した順（「いつから」の新しい順）にそのまま並ぶ", async () => {
    const claims = [
      claim({ id: "c1", value: "C", valid_from: { precision: "day", date: "2023-03-18" } }),
      claim({ id: "c2", value: "B", valid_from: { precision: "month", date: "2017-04" } }),
      claim({ id: "c3", value: "A", valid_from: { precision: "month", date: "2013-04" } }),
    ];
    await open(view([kind({ id: "k", name: "住所", current: claims[0], claims })]));
    const rows = within(card("住所")).getAllByTestId("claim-row");
    expect(rows.map((r) => r.textContent?.slice(0, 1))).toEqual(["C", "B", "A"]);
    expect(rows[0].textContent).toContain("2023 年 3 月 18 日から");
    expect(rows[1].textContent).toContain("2017 年 4 月から");
  });

  // Scenario: 補足は押さずに見える
  /** **本人が押したときだけにしたのは主張した日時だけ**（Q2）。補足はそれに合わせない（D13（仮））。 */
  it("補足は押す前から見えている", async () => {
    const c = claim({ id: "c1", note: "転職に合わせて引っ越した" });
    await open(view([kind({ id: "k", name: "住所", current: c, claims: [c] })]));
    expect(screen.getByTestId("claim-note").textContent).toContain("転職に合わせて引っ越した");
  });

  // Scenario: 書いた日時は主張を押したときだけ出る
  it("主張した日時は押す前は見えず、押すと見える", async () => {
    const c = claim({ id: "c1", asserted_at: "2026-09-01T10:00:00+09:00" });
    await open(view([kind({ id: "k", name: "住所", current: c, claims: [c] })]));
    const row = screen.getByTestId("claim-row");
    expect(screen.queryByTestId("asserted-at"), "押す前に書いた日時が見えている").toBeNull();
    expect(row.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(row);
    expect(screen.getByTestId("asserted-at").textContent).toContain("に書いた");
    expect(row.getAttribute("aria-expanded")).toBe("true");
  });

  // Scenario: 訂正で取り消した主張は畳まれる
  // Scenario: 畳んだ取り消しは押すと出る
  it("取り消された主張は「訂正で取り消した 1 件」に畳まれ、押すと出る", async () => {
    const live = claim({ id: "c1", value: "新しい値" });
    const gone = claim({ id: "c0", value: "取り消された値", superseded_by: "c1" });
    await open(view([kind({ id: "k", name: "住所", current: live, claims: [live], superseded: [gone] })]));

    const fold = screen.getByRole("button", { name: "訂正で取り消した 1 件" });
    expect(screen.queryByText(/取り消された値/), "畳む前に取り消された値が見えている").toBeNull();
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(fold);
    expect(screen.getByTestId("superseded").textContent).toContain("取り消された値");
  });

  // Scenario: 予定の主張は予定の文字とともに出る
  it("予定は「予定」の文字つきで出て、いまの値の欄はその値ではない", async () => {
    const now = claim({ id: "c1", value: "いまの住所" });
    const soon = claim({ id: "c2", value: "新居", valid_from: { precision: "day", date: "2026-12-01" } });
    await open(view([kind({ id: "k", name: "住所", current: now, upcoming: [soon], claims: [soon, now] })]));

    const c = card("住所");
    expect(within(c).getByTestId("upcoming").textContent).toContain("新居");
    expect(within(c).getByTestId("upcoming").textContent).toContain("予定");
    expect(within(c).getByTestId("current-value").textContent, "予定がいまの値になっている").toBe("いまの住所");
  });

  // Scenario: いまの値が無い種類はまだ書いていないと出る
  /** **「まだ書いていない」と「なし」を分ける**（深掘り C10）。 */
  it("主張を持たない種類は「まだ書いていない」、「なし」の主張は「なし」", async () => {
    const none = claim({ id: "c1", value: null });
    await open(
      view([
        kind({ id: "k1", name: "住所" }),
        kind({ id: "k2", name: "副業", current: none, claims: [none] }),
      ]),
    );
    expect(within(card("住所")).getByTestId("current-value").textContent).toBe("まだ書いていない");
    expect(within(card("副業")).getByTestId("current-value").textContent).toBe("なし");
  });

  // Scenario: 人物と場所のタブは無い
  /** **押しても何も無いタブを置かない** —— 人物（ST20）と場所（ST21）が中身とともに足す。 */
  it("タブは「個人属性」の 1 つだけ", async () => {
    await open(eightClaims());
    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(1);
    expect(tabs[0].textContent).toBe("個人属性");
    expect(screen.queryByRole("tab", { name: "人物" })).toBeNull();
    expect(screen.queryByRole("tab", { name: "場所" })).toBeNull();
  });

  // Scenario: 編集する操作が無い
  /** **主張は書き換えられない**（FR-44 / Q1）—— 書き換えの欄も、送る操作も画面に置かない。 */
  it("主張の行を全部押しても、「書く」のフォームの外に入力欄が出ない", async () => {
    await open(eightClaims());
    for (const row of screen.getAllByTestId("claim-row")) fireEvent.click(row);
    expect(screen.queryByTestId("write-form"), "フォームが開いている（前提が違う）").toBeNull();
    expect(
      document.querySelectorAll("input, textarea"),
      "主張を押すと入力欄が出る（書き換えの経路になる）",
    ).toHaveLength(0);
  });

  // Scenario: 稼働状況の画面からマスタ管理へ行ける
  it("稼働状況の入口を押すと個人属性の画面が開く", async () => {
    window.location.hash = "";
    vi.stubGlobal("fetch", (path: string) => {
      if (path.includes("/attributes")) return Promise.resolve(ok(eightClaims()));
      if (path.includes("achievement")) {
        return Promise.resolve(ok({ sources: [], days: 0, needed: 0, passed: false, provisional: true }));
      }
      return Promise.resolve(ok([]));
    });
    render(<Root />);
    const to = await screen.findByTestId("to-master");
    expect(to.getAttribute("href")).toBe("#/master");

    // hashchange は jsdom が自動では撃たないので、行き先を変えて撃つ
    window.location.hash = "#/master";
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    expect(await screen.findByTestId("master-view")).toBeTruthy();
  });

  // Scenario: 読み出しの失敗と主張が無いことを区別する
  /** **混ぜると、サーバが落ちている間ずっと「属性が 1 つも無い」と読める。** */
  it("読み出しに失敗したら失敗と出し、「まだ書いていない」とは出さない", async () => {
    await open(500);
    expect(screen.getByTestId("master-failed").textContent).toContain("読み出せませんでした");
    expect(screen.queryByText("まだ書いていない"), "失敗を「まだ書いていない」と出している").toBeNull();
    expect(screen.queryByTestId("kind-card")).toBeNull();
  });

  it("読み出しは `/api/attributes` を引く", async () => {
    await open(eightClaims());
    expect(calls.map((c) => c.path)).toContain("/api/attributes");
  });
});

describe("独立レビューで足したもの（review/code.md）", () => {
  // Scenario: 予定の主張は予定の文字とともに出る
  /**
   * **R10**: 積んだ主張の行の「（予定）」は、**サーバが返した `upcoming`** で決まること。
   * 画面が `valid_from.date > today` を文字列比較で組み直していたときは、この assertion が
   * 無かったので `const future = false` に変えても web 114 件が全部緑だった。
   */
  it("行の「（予定）」はサーバの `upcoming` に従う（画面で判定し直さない）", async () => {
    const now = claim({ id: "c1", value: "いまの住所" });
    // **「いつから」は今日より前なのに、サーバは予定だと言っている**（規則が画面にあれば食い違う）
    const odd = claim({
      id: "c2",
      value: "サーバが予定と言う主張",
      valid_from: { precision: "year", date: "2000" },
    });
    await open(view([kind({ id: "k", name: "住所", current: now, upcoming: [odd], claims: [odd, now] })]));

    const rows = within(card("住所")).getAllByTestId("claim-row");
    const marked = rows.filter((r) => r.textContent?.includes("（予定）"));
    expect(marked, "サーバが予定と言った主張に印が付いていない").toHaveLength(1);
    expect(marked[0].textContent).toContain("サーバが予定と言う主張");
  });

  // Scenario: 読み出しの失敗と主張が無いことを区別する
  /** **R28**: 200 でも**形が違えば失敗として出す**（描画で落ちて画面が白くなるのを防ぐ）。 */
  it("200 で形の違う応答も、失敗として出す", async () => {
    serve({ today: "2026-09-15" } as unknown as AttributesView);
    render(<Root />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    expect(screen.getByTestId("master-failed").textContent).toContain("unexpected_shape");
    expect(screen.queryByText("まだ書いていない")).toBeNull();
  });
});
