// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 「書く」のフォームと、種類を足す・名前を変える（ST19 / tasks 4.3 / design D9）。
 *
 * **入力を変えずに押し直したら同じ原文を送る**（spec）—— 組み直すと乱数が変わり、
 * サーバは畳めずに同じ主張が 2 件になる。
 *
 * `fetch` は差し替え、時刻は差し込む。**送った原文を読んで確かめる**（画面の文字だけを見ない）。
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MasterView } from "../MasterView";
import type { AttributesView, Claim, KindView } from "../attributes";

const ok = (body: unknown): Response =>
  ({ ok: true, status: 200, json: () => Promise.resolve(body) }) as Response;
const accepted = (): Response => ok([{ id: "x", duplicate: false, accepted: true, error: null }]);
/** **1 件だけ送って断られると `/ingest` は HTTP 400 を返す**（1 件も受け付けなかったとき）。 */
const refused = (kind: string): Response =>
  ({
    ok: false,
    status: 400,
    json: () => Promise.resolve([{ id: null, duplicate: false, accepted: false, error: kind }]),
  }) as Response;

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

const ADDRESS = "k-address";

/**
 * 「いつから」の新しい順と、**書いた順が違う** 2 件。
 * 取り消す主張の既定は「主張した日時が最も新しい」＝ `OLDER_BUT_WRITTEN_LAST`。
 */
const OLDER_BUT_WRITTEN_LAST = claim({
  id: "c-old",
  value: "古い住所",
  valid_from: { precision: "year", date: "2013" },
  asserted_at: "2026-09-10T10:00:00+09:00",
});
const NEWER_BUT_WRITTEN_FIRST = claim({
  id: "c-new",
  value: "新しい住所",
  valid_from: { precision: "year", date: "2023" },
  asserted_at: "2026-09-01T10:00:00+09:00",
});

function withClaims(): AttributesView {
  return {
    today: "2026-09-15",
    kinds: [
      kind({
        id: ADDRESS,
        name: "住所",
        current: NEWER_BUT_WRITTEN_FIRST,
        claims: [NEWER_BUT_WRITTEN_FIRST, OLDER_BUT_WRITTEN_LAST],
      }),
    ],
  };
}

function empty(): AttributesView {
  return { today: "2026-09-15", kinds: [kind({ id: ADDRESS, name: "住所" })] };
}

let posts: { path: string; body: unknown }[] = [];
let reads = 0;

/** `/api/attributes` は `body` を返し、書き込みは `reply()` の応答を返す。 */
function serve(body: AttributesView, reply: () => Response): void {
  posts = [];
  reads = 0;
  vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
    if (path === "/api/attributes") {
      reads += 1;
      return Promise.resolve(ok(body));
    }
    posts.push({ path, body: JSON.parse(String(init?.body ?? "null")) });
    return Promise.resolve(reply());
  });
}

/** 送った 1 件の原文を読む。 */
function sentRaw(i = 0): Record<string, unknown> {
  const item = (posts[i].body as { raw: string }[])[0];
  return JSON.parse(item.raw) as Record<string, unknown>;
}

async function openForm(): Promise<void> {
  render(<MasterView />);
  await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "書く" }));
  expect(screen.getByTestId("write-form")).toBeTruthy();
}

/** 値と「いつから」を入れる。 */
function fill(value: string, year: string, month?: string): void {
  fireEvent.change(screen.getByLabelText("値"), { target: { value } });
  fireEvent.change(screen.getByLabelText("いつから（年）"), { target: { value: year } });
  if (month !== undefined) {
    fireEvent.change(screen.getByLabelText("いつから（月）"), { target: { value: month } });
  }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-09-15T02:00:00Z"));
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("「書く」のフォーム", () => {
  // Scenario: 変わったを選んで主張を積める
  it("「変わった」で値と精度「年月」の「いつから」を入れて積める", async () => {
    serve(empty(), accepted);
    await openForm();
    fill("東京都 世田谷区", "2026", "9");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));

    await waitFor(() => expect(posts).toHaveLength(1));
    expect(posts[0].path).toBe("/api/ingest");
    const raw = sentRaw();
    expect(raw.kind).toBe(ADDRESS);
    expect(raw.value).toBe("東京都 世田谷区");
    expect(raw.valid_from).toEqual({ precision: "month", date: "2026-09" });
    expect(raw.supersedes, "「変わった」なのに取り消し先が入っている").toBeNull();
  });

  // Scenario: 間違いを直すと取り消す主張を選べる
  // Scenario: 取り消す主張は最も新しく書いた主張が選ばれている
  /**
   * **既定は「主張した日時が最も新しい」**（spec）—— 「いつから」の新しい順で並ぶ
   * `claims` の先頭とは限らない。ここでは古い「いつから」の主張が最後に書かれている。
   */
  it("「間違っていた」で最も新しく書いた主張が既定で選ばれ、原文の取り消し先に入る", async () => {
    serve(withClaims(), accepted);
    await openForm();
    fireEvent.click(screen.getByLabelText("前の書き込みが間違っていた"));

    const select = screen.getByLabelText<HTMLSelectElement>("取り消す主張");
    expect(select.value, "「いつから」の新しい主張が既定で選ばれている").toBe(OLDER_BUT_WRITTEN_LAST.id);

    fill("直した住所", "2014");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));
    expect(sentRaw().supersedes).toBe(OLDER_BUT_WRITTEN_LAST.id);
  });

  it("取り消す主張を選び直すと、選んだ主張が原文に入る", async () => {
    serve(withClaims(), accepted);
    await openForm();
    fireEvent.click(screen.getByLabelText("前の書き込みが間違っていた"));
    fireEvent.change(screen.getByLabelText("取り消す主張"), {
      target: { value: NEWER_BUT_WRITTEN_FIRST.id },
    });
    fill("直した住所", "2024");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));
    expect(sentRaw().supersedes).toBe(NEWER_BUT_WRITTEN_FIRST.id);
  });

  // Scenario: 精度に年を選ぶと年の欄だけが出る
  // Scenario: 精度に分からないを選ぶと日付の欄が出ない
  /** **精度を先に選ぶ**（Q2。本人が proto で決めた）。 */
  it("選んだ精度の欄だけが出る", async () => {
    serve(empty(), accepted);
    await openForm();
    // 既定は「年月」
    expect(screen.getByLabelText("いつから（年）")).toBeTruthy();
    expect(screen.getByLabelText("いつから（月）")).toBeTruthy();
    expect(screen.queryByLabelText("いつから（日）")).toBeNull();

    fireEvent.click(screen.getByRole("radio", { name: "年" }));
    expect(screen.getByLabelText("いつから（年）")).toBeTruthy();
    expect(screen.queryByLabelText("いつから（月）"), "精度「年」で月の欄が出ている").toBeNull();
    expect(screen.queryByLabelText("いつから（日）")).toBeNull();

    fireEvent.click(screen.getByRole("radio", { name: "年月日" }));
    expect(screen.getByLabelText("いつから（日）")).toBeTruthy();

    fireEvent.click(screen.getByRole("radio", { name: "分からない" }));
    for (const f of ["いつから（年）", "いつから（月）", "いつから（日）"]) {
      expect(screen.queryByLabelText(f), `精度「分からない」で ${f} が出ている`).toBeNull();
    }
  });

  it("精度「分からない」で積むと、原文の「いつから」は日付を持たない", async () => {
    serve(empty(), accepted);
    await openForm();
    fireEvent.change(screen.getByLabelText("値"), { target: { value: "実家" } });
    fireEvent.click(screen.getByRole("radio", { name: "分からない" }));
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));
    expect(sentRaw().valid_from).toEqual({ precision: "unknown", date: null });
  });

  // Scenario: なしを選んで積める
  /** **「なし」は値が無いことではない**（深掘り C10）—— 置き換える値が無い終わり方。 */
  it("「なし」を選ぶと原文の値が null になる", async () => {
    serve(empty(), accepted);
    await openForm();
    fireEvent.click(screen.getByLabelText("なし（その属性が終わった）"));
    fill("", "2025");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));
    expect(sentRaw().value).toBeNull();
  });

  // Scenario: 主張した日時は入力させない
  /** **ずらせると、2 つの時刻を分けた意味（後から直したことが分かる）が消える**（深掘り C4）。 */
  it("主張した日時の欄は無く、送られるのは押した時刻", async () => {
    serve(empty(), accepted);
    await openForm();
    for (const name of ["主張した日時", "書いた日時", "書いた日"]) {
      expect(screen.queryByLabelText(name), `${name} の欄がある`).toBeNull();
    }
    fill("東京都", "2026", "9");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));
    const item = (posts[0].body as { event_time: string }[])[0];
    expect(item.event_time).toBe("2026-09-15T02:00:00.000Z");
  });

  // Scenario: 2 回押しても送る原文は 1 つ
  /** **組み直すと乱数が変わり、サーバは畳めずに同じ主張が 2 件になる。** */
  it("1 回目が届かなくても、押し直しで送る原文は 1 回目と同じ", async () => {
    let attempt = 0;
    posts = [];
    vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
      if (path === "/api/attributes") return Promise.resolve(ok(empty()));
      posts.push({ path, body: JSON.parse(String(init?.body ?? "null")) });
      attempt += 1;
      // 1 回目は届かない
      if (attempt === 1) return Promise.reject(new Error("offline"));
      return Promise.resolve(accepted());
    });

    render(<MasterView />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "書く" }));
    fill("東京都 世田谷区", "2026", "9");

    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(screen.getByTestId("write-problem")).toBeTruthy());
    // **入力を変えずに**押し直す
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(2));

    expect(sentRaw(0), "押し直しで原文が組み直されている（同じ主張が 2 件になる）").toEqual(sentRaw(1));
  });

  it("入力を 1 か所でも変えたら原文を組み直す（同じ `id` で別の原文は `id_reused` になる）", async () => {
    serve(empty(), () => refused("invalid_valid_from"));
    await openForm();
    fill("東京都", "2026", "9");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(1));

    fireEvent.change(screen.getByLabelText("値"), { target: { value: "大阪府" } });
    fireEvent.click(screen.getByRole("button", { name: "積む" }));
    await waitFor(() => expect(posts).toHaveLength(2));
    expect(sentRaw(0).claim, "値を変えたのに同じ識別子で送っている").not.toBe(sentRaw(1).claim);
    expect(sentRaw(1).value).toBe("大阪府");
  });

  // Scenario: 積めたらフォームが閉じて読み直す
  it("受理ならフォームが閉じ、個人属性をもう一度読み出す", async () => {
    serve(empty(), accepted);
    await openForm();
    expect(reads).toBe(1);
    fill("東京都", "2026", "9");
    fireEvent.click(screen.getByRole("button", { name: "積む" }));

    await waitFor(() => expect(screen.queryByTestId("write-form")).toBeNull());
    expect(reads, "受理の後に読み直していない").toBe(2);
  });

  // Scenario: 断られたとき入力が残り理由が出る
  /** **入力を捨てると、本人は打ち直しになる。** HTTP 400 と 1 件ごとの結果で作る。 */
  it("断られたら理由が出て、入力（「いつから」と補足）が残る", async () => {
    serve(empty(), () => refused("invalid_claim_value"));
    await openForm();
    fill("   ", "2026", "9");
    fireEvent.change(screen.getByLabelText("補足"), { target: { value: "引っ越した" } });
    fireEvent.click(screen.getByRole("button", { name: "積む" }));

    await waitFor(() => expect(screen.getByTestId("write-problem")).toBeTruthy());
    expect(screen.getByTestId("write-problem").textContent).toContain("値が空です");
    expect(screen.getByTestId("write-form"), "断られたのにフォームが閉じている").toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>("いつから（年）").value).toBe("2026");
    expect(screen.getByLabelText<HTMLInputElement>("いつから（月）").value).toBe("9");
    expect(screen.getByLabelText<HTMLInputElement>("補足").value).toBe("引っ越した");
  });

  // Scenario: 届かなかったとき入力が残り届かなかったと出る
  /** **断られたときの文と違うもの**（spec）—— 混ぜると、本人は入力を直そうとして直らない。 */
  it("届かなかったら、断られたときとは違う文が出て入力が残る", async () => {
    vi.stubGlobal("fetch", (path: string) => {
      if (path === "/api/attributes") return Promise.resolve(ok(empty()));
      return Promise.reject(new Error("offline"));
    });
    render(<MasterView />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "書く" }));
    fill("東京都", "2026", "9");
    fireEvent.change(screen.getByLabelText("補足"), { target: { value: "引っ越した" } });
    fireEvent.click(screen.getByRole("button", { name: "積む" }));

    await waitFor(() => expect(screen.getByTestId("write-problem")).toBeTruthy());
    const why = screen.getByTestId("write-problem").textContent ?? "";
    expect(why).toContain("届きませんでした");
    expect(why, "断られたときと同じ文が出ている").not.toContain("値が空です");
    expect(screen.getByLabelText<HTMLInputElement>("値").value).toBe("東京都");
    expect(screen.getByLabelText<HTMLInputElement>("いつから（年）").value).toBe("2026");
    expect(screen.getByLabelText<HTMLInputElement>("補足").value).toBe("引っ越した");
  });

  it("「やめる」で閉じ、送らない", async () => {
    serve(empty(), accepted);
    await openForm();
    fill("東京都", "2026", "9");
    fireEvent.click(screen.getByRole("button", { name: "やめる" }));
    expect(screen.queryByTestId("write-form")).toBeNull();
    expect(posts).toHaveLength(0);
  });
});

describe("種類を足す・名前を変える", () => {
  // Scenario: 画面から種類を足して名前を変えられる
  it("「副業」を足し、その名前を「副収入」に変えると、カードが入れ替わる", async () => {
    // 読み直しのたびに次の状態を返す（足す → 名前を変える）
    const states: AttributesView[] = [
      empty(),
      { today: "2026-09-15", kinds: [kind({ id: ADDRESS, name: "住所" }), kind({ id: "k-side", name: "副業" })] },
      { today: "2026-09-15", kinds: [kind({ id: ADDRESS, name: "住所" }), kind({ id: "k-side", name: "副収入" })] },
    ];
    let read = 0;
    posts = [];
    vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
      if (path === "/api/attributes") {
        const body = states[Math.min(read, states.length - 1)];
        read += 1;
        return Promise.resolve(ok(body));
      }
      posts.push({ path, body: JSON.parse(String(init?.body ?? "null")) });
      return Promise.resolve({ ok: true, status: 204, json: () => Promise.resolve({}) } as Response);
    });

    render(<MasterView />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());

    // 足す
    fireEvent.click(screen.getByRole("button", { name: "種類を足す" }));
    fireEvent.change(screen.getByLabelText("種類を足す"), { target: { value: "副業" } });
    fireEvent.click(screen.getByRole("button", { name: "決める" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "副業" })).toBeTruthy());
    expect(posts[0].path).toBe("/api/attributes/kinds");
    expect(posts[0].body).toEqual({ name: "副業" });

    // 名前を変える（種類の名前を押すと欄が出る）
    fireEvent.click(screen.getByRole("button", { name: "副業" }));
    fireEvent.change(screen.getByLabelText("「副業」の名前を変える"), { target: { value: "副収入" } });
    fireEvent.click(screen.getByRole("button", { name: "決める" }));

    await waitFor(() => expect(screen.getByRole("button", { name: "副収入" })).toBeTruthy());
    expect(screen.queryByRole("button", { name: "副業" }), "前の名前のカードが残っている").toBeNull();
    // **識別子ではなく名前の台帳へ足す口を叩く**（名前を変えても識別子は変わらない）
    expect(posts[1].path).toBe("/api/attributes/kinds/k-side/names");
    expect(posts[1].body).toEqual({ name: "副収入" });
  });

  it("断られたら理由が出て、欄は閉じない", async () => {
    vi.stubGlobal("fetch", (path: string) => {
      if (path === "/api/attributes") return Promise.resolve(ok(empty()));
      return Promise.resolve({
        ok: false,
        status: 400,
        json: () => Promise.resolve({ error: "duplicate_name" }),
      } as Response);
    });
    render(<MasterView />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "種類を足す" }));
    fireEvent.change(screen.getByLabelText("種類を足す"), { target: { value: "住所" } });
    fireEvent.click(screen.getByRole("button", { name: "決める" }));

    await waitFor(() => expect(screen.getByRole("alert")).toBeTruthy());
    expect(screen.getByTestId("name-form")).toBeTruthy();
  });
});
