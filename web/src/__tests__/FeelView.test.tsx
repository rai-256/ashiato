// SPDX-License-Identifier: AGPL-3.0-only
/**
 * S-3 主観入力の画面（ST17 / tasks 8.1 / design D12）—— 押した瞬間に保存する。
 *
 * `fetch` は差し替え、時刻は `Date` だけ固定する。**送った原文を読んで確かめる**（画面の文字だけを見ない）。
 * 画面の Scenario は `web/e2e/feel.spec.ts`（本物のブラウザ）が担保する。ここは文字の組み立てを細かく撃つ追加。
 */
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FeelView } from "../FeelView";
import type { Feeling } from "../feelings";
import { Root } from "../Root";

/** JST 2026-10-08 12:00 */
const NOON = "2026-10-08T03:00:00Z";

const json = (status: number, body: unknown): Response => ({ ok: status < 400, status, json: () => Promise.resolve(body) }) as Response;
const accepted = (): Response => json(200, [{ id: "x", duplicate: false, accepted: true, error: null }]);

let seq = 0;
function rec(v: number, over: Partial<Feeling> & { e?: number; phase?: Feeling["phase"] } = {}): Feeling {
  seq += 1;
  const { e, ...rest } = over;
  return {
    id: `f-${seq}`,
    scales: [{ id: "valence", version: 1, value: v }, ...(e === undefined ? [] : [{ id: "energy", version: 1, value: e }])],
    note: null,
    written_at: "2026-10-08T11:00:00+09:00",
    ingested_at: "2026-10-08T02:00:00Z",
    phase: "before",
    target_stay: null,
    ...rest,
  };
}

/** 記録（新しい順）から `GET /feelings` の 1 日の分を作る。最新は前/後ごとの先頭。 */
function view(date: string, records: Feeling[]): unknown {
  const first = (p: Feeling["phase"]): Feeling | null => records.find((r) => r.phase === p) ?? null;
  return {
    today: "2026-10-08",
    first_day: null,
    days: [{ date, status: "recorded", day: { latest: { before: first("before"), after: first("after") }, records }, stays: [], stays_with_records: 0, hidden_stay_records: 0 }],
  };
}

let posts: { raw: string; body: Record<string, unknown> }[] = [];
let ingest: () => Response | Promise<Response> = accepted;
let records: Feeling[] = [];
let gets: string[] = [];
let feelingsFail = false;
/** `/api/stays` が返す滞在の行（既定は無し）と、その滞在に付く記録 */
let stayEntries: Record<string, unknown>[] = [];
let stayRecords: Feeling[] = [];

beforeEach(() => {
  seq = 0;
  posts = [];
  gets = [];
  feelingsFail = false;
  stayEntries = [];
  stayRecords = [];
  records = [];
  ingest = accepted;
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(NOON));
  window.location.hash = "";
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string, init?: RequestInit) => {
      if (init?.method === "POST") {
        return Promise.resolve(ingest()).then((r) => {
          const items = JSON.parse(init.body as string) as Record<string, unknown>[];
          posts.push({ raw: items[0].raw as string, body: JSON.parse(items[0].raw as string) as Record<string, unknown> });
          return r;
        });
      }
      if (url.startsWith("/api/stays")) return Promise.resolve(json(200, { date: "", criteria: [], entries: stayEntries }));
      if (url.startsWith("/api/feelings/reminder")) return Promise.resolve(json(200, { at: "21:00", recorded: false }));
      gets.push(url);
      if (feelingsFail) return Promise.resolve(json(500, {}));
      const date = /to=([\d-]+)/.exec(url)?.[1] ?? "";
      const v = view(date, records) as { days: { stays: unknown[] }[] };
      if (stayRecords.length > 0) v.days[0].stays = [{ stay: "stay-1", latest: { before: stayRecords[0], after: null }, records: stayRecords }];
      return Promise.resolve(json(200, v));
    }),
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function open(date: string | null = null): Promise<void> {
  render(<FeelView date={date} />);
  await screen.findByTestId("valence-1");
}

const press = (testid: string): void => {
  fireEvent.click(screen.getByTestId(testid));
};
const note = (): HTMLTextAreaElement => screen.getByTestId("feel-note") as HTMLTextAreaElement;
const typeNote = (text: string): void => {
  fireEvent.change(note(), { target: { value: text } });
};
const sent = async (n: number): Promise<void> => {
  await waitFor(() => {
    expect(posts).toHaveLength(n);
  });
};
/** 読み出しと再描画が落ち着くまで待つ（`Date` だけ偽物なので、タイマーは本物で進む）。 */
const settle = async (): Promise<void> => {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 100));
  });
};
const scalesOf = (i: number): { id: string; value: number }[] => posts[i].body.scales as { id: string; value: number }[];

describe("押した瞬間に保存する", () => {
  // Scenario: 通知から開いて 1 回押すと保存される
  it("快を 1 回押すと、確定の操作なしで今日の「日」に快–不快 +1 が 1 件送られ、保存と記入の日時が出る", async () => {
    await open();
    press("valence-1");
    await sent(1);
    expect(posts[0].body.target).toEqual({ kind: "day", date: "2026-10-08" });
    expect(scalesOf(0)).toEqual([{ id: "valence", version: 1, value: 1 }]);
    const ok = await screen.findByTestId("feel-notice-ok");
    expect(ok.textContent).toContain("保存しました");
    expect(ok.textContent).toContain("+1 快");
    expect(ok.textContent).toContain("10 月 8 日 12:00");
  });

  // Scenario: 日付の無いアドレスは夜中も暦の今日を開く
  it("Asia/Tokyo の 2026-10-08 00:30 に日付の無いアドレスを開くと、見出しの日付は 2026-10-08", async () => {
    vi.setSystemTime(new Date("2026-10-07T15:30:00Z"));
    window.location.hash = "#/feel";
    render(<Root />);
    const title = await screen.findByTestId("feel-title");
    expect(title.querySelector("time")?.getAttribute("datetime")).toBe("2026-10-08");
    expect(gets[0]).toContain("to=2026-10-08");
  });

  // Scenario: 暦に無い日付のアドレスは書く操作を出さない
  it("暦に無い日付のアドレスは、日付が読めないことと今日への入口を出し、快–不快のボタンを置かない", () => {
    window.location.hash = "#/feel/2026-13-45";
    render(<Root />);
    expect(screen.getByRole("alert").textContent).toContain("2026-13-45");
    expect(screen.getByRole("link", { name: "今日の気分を書く" }).getAttribute("href")).toBe("#/feel");
    expect(screen.queryByTestId("valence-1")).toBeNull();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  // Scenario: 保存したら対象の日が出る
  it("2026-10-08 に 2026-10-07 の画面で快–不快を押すと、保存と対象の日 2026-10-07 と記入の日時が出る", async () => {
    await open("2026-10-07");
    press("valence-1");
    await sent(1);
    expect(posts[0].body.target).toEqual({ kind: "day", date: "2026-10-07" });
    const ok = await screen.findByTestId("feel-notice-ok");
    expect(ok.textContent).toContain("保存しました");
    expect(ok.textContent).toContain("対象の日 2026-10-07");
    expect(ok.textContent).toContain("記入 10 月 8 日 12:00");
  });

  // Scenario: 昨日の分は前の日へ 1 回で書ける
  it("今日の画面で前の日へを 1 回押して快–不快を押すと、昨日の「日」に 1 件送られる", async () => {
    window.location.hash = "#/feel";
    render(<Root />);
    await screen.findByTestId("valence-1");
    press("feel-prev");
    await waitFor(() => {
      expect(window.location.hash).toBe("#/feel/2026-10-07");
    });
    await waitFor(() => {
      expect(gets.some((u) => u.includes("to=2026-10-07"))).toBe(true);
    });
    await screen.findByTestId("valence-1");
    press("valence-1");
    await sent(1);
    expect(posts[0].body.target).toEqual({ kind: "day", date: "2026-10-07" });
  });

  // Scenario: 次の日へ移って未来の日に書ける
  it("今日の画面で次の日へを押して快–不快を押すと、明日の「日」に 1 件送られる", async () => {
    window.location.hash = "#/feel";
    render(<Root />);
    await screen.findByTestId("valence-1");
    press("feel-next");
    await waitFor(() => {
      expect(gets.some((u) => u.includes("to=2026-10-09"))).toBe(true);
    });
    await screen.findByTestId("valence-1");
    press("valence-1");
    await sent(1);
    expect(posts[0].body.target).toEqual({ kind: "day", date: "2026-10-09" });
  });
});

describe("5 つのボタン・ひとこと・元気さ", () => {
  // Scenario: 快不快のボタンは数値と言葉を持つ
  it("5 つのボタンが -2・-1・0・+1・+2 の数値と言葉を持つ", async () => {
    await open();
    const group = screen.getByRole("group", { name: "快–不快" });
    const names = within(group).getAllByRole("button").map((b) => b.textContent);
    expect(names).toEqual(["-2とても不快", "-1不快", "0どちらでもない", "+1快", "+2とても快"]);
    expect(within(group).getByRole("button", { name: "+2 とても快" })).toBeTruthy();
  });

  // Scenario: ひとことと元気さは押さずに見える
  it("何も押す前から、ひとことの欄と元気さの 5 つのボタンが見えている", async () => {
    await open();
    expect(note()).toBeTruthy();
    const energy = within(screen.getByRole("group", { name: "元気さ" })).getAllByRole("button");
    expect(energy.map((b) => b.textContent)).toEqual(["-2", "-1", "0", "+1", "+2"]);
    expect(posts).toHaveLength(0);
  });

  // Scenario: 前後を選ぶ操作が無い
  it("前に書くか後に書くかを選ぶ操作が無い", async () => {
    await open();
    expect(screen.queryAllByRole("radio")).toHaveLength(0);
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
    expect(screen.queryAllByRole("button", { name: /^(前|後)$|前に書|後に書/ })).toHaveLength(0);
  });
});

describe("積む", () => {
  // Scenario: ひとことを足すと記録がもう 1 件積まれる
  it("快–不快 +1 の後にひとこと「散歩した」を入れて欄を離れると、+1 とひとことの記録がもう 1 件送られる", async () => {
    await open();
    press("valence-1");
    await sent(1);
    typeNote("散歩した");
    fireEvent.blur(note());
    await sent(2);
    expect(scalesOf(1)).toEqual([{ id: "valence", version: 1, value: 1 }]);
    expect(posts[1].body.note).toBe("散歩した");
    expect(posts[0].body.note).toBeNull();
    expect(posts[1].body.feeling).not.toBe(posts[0].body.feeling);
  });

  // Scenario: 元気さを押すと今の快不快と一緒に積まれる
  it("快–不快 -1 の後に元気さ +2 を押すと、-1 と +2 の記録がもう 1 件送られる", async () => {
    await open();
    press("valence--1");
    await sent(1);
    press("energy-2");
    await sent(2);
    expect(scalesOf(1)).toEqual([
      { id: "valence", version: 1, value: -1 },
      { id: "energy", version: 1, value: 2 },
    ]);
  });

  // Scenario: 快不快を選ぶ前は元気さだけを送らない
  it("記録の無い日に元気さだけを押しても送らず、ひとことだけ入れて離れても送らない", async () => {
    await open();
    press("energy-2");
    typeNote("まだ選んでいない");
    fireEvent.blur(note());
    await act(() => Promise.resolve());
    expect(posts).toHaveLength(0);
    // 快–不快を押したとき、持っておいた元気さとひとことが一緒に送られる
    press("valence-1");
    await sent(1);
    expect(scalesOf(0).map((s) => s.value)).toEqual([1, 2]);
    expect(posts[0].body.note).toBe("まだ選んでいない");
  });
});

describe("送れなかったとき", () => {
  // Scenario: 届かなかった記録は同じ原文で送り直す
  it("届かなかった後に「もう一度送る」を押すと、1 回目と同じ原文が送られる", async () => {
    ingest = () => json(503, null);
    await open();
    press("valence-1");
    await sent(1);
    const resend = await screen.findByTestId("feel-resend");
    ingest = accepted;
    fireEvent.click(resend);
    await sent(2);
    expect(posts[1].raw).toBe(posts[0].raw);
    await screen.findByTestId("feel-notice-ok");
  });

  // Scenario: 届かなかった記録は同じ原文で送り直す
  it("ひとことを入れて届かなかった後に「もう一度送る」を押しても、欄のフォーカスを奪わず同じ原文が送られる", async () => {
    ingest = () => json(503, null);
    await open();
    typeNote("雨だった");
    note().focus();
    press("valence-1");
    await sent(1);
    const resend = await screen.findByTestId("feel-resend");
    // 押し込みでフォーカスが移ると欄の blur で別の原文が先に送られる。それを止めているか
    expect(fireEvent.mouseDown(resend)).toBe(false);
    ingest = accepted;
    fireEvent.click(resend);
    await sent(2);
    expect(posts[1].raw).toBe(posts[0].raw);
    await screen.findByTestId("feel-notice-ok");
    expect(posts).toHaveLength(2);
  });

  // Scenario: 届かなかった記録は同じ原文で送り直す
  it("ひとこと付きの記録を送り直して届いた後に欄を離れても、同じひとことをもう一度送らない", async () => {
    ingest = () => json(503, null);
    await open();
    typeNote("雨だった");
    press("valence-1");
    await sent(1);
    const resend = await screen.findByTestId("feel-resend");
    ingest = accepted;
    fireEvent.click(resend);
    await sent(2);
    await screen.findByTestId("feel-notice-ok");
    fireEvent.blur(note());
    fireEvent.click(screen.getByTestId("feel-keep-note"));
    await new Promise((r) => setTimeout(r, 50));
    expect(posts).toHaveLength(2);
  });

  // Scenario: 届かなかったとき選んだ値とひとことが残る
  it("ひとことを入れて、サーバに届かない状態で快–不快を押すと、届かなかった文が出て値とひとことが残る", async () => {
    await open();
    typeNote("雨だった");
    ingest = () => {
      throw new TypeError("network");
    };
    press("valence--2");
    const bad = await screen.findByTestId("feel-notice-bad");
    expect(bad.textContent).toContain("サーバに届きませんでした");
    expect(bad.textContent).not.toContain("受け付けられませんでした");
    expect(note().value).toBe("雨だった");
    expect(screen.getByTestId("valence--2").getAttribute("aria-pressed")).toBe("true");
  });

  // Scenario: 断られたとき理由の文が出る
  it("ひとことが長すぎるとして断られると、その文が出て、入れたひとことが残る", async () => {
    ingest = () => json(400, [{ id: null, duplicate: false, accepted: false, error: "invalid_note" }]);
    await open();
    typeNote("断られる");
    press("valence-1");
    const bad = await screen.findByTestId("feel-notice-bad");
    expect(bad.textContent).toContain("ひとことが長すぎます（2,000 文字まで）");
    expect(bad.textContent).not.toContain("サーバに届きませんでした");
    expect(note().value).toBe("断られる");
    // 日の欄は断られても押した値を残す（本人の決定 Q4。review/code.md R15）
    expect(screen.getByTestId("valence-1").getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByTestId("feel-resend")).toBeNull();
  });
});

describe("断られた理由の文", () => {
  // Scenario: 断られたとき理由の文が出る
  it("制御文字の入ったひとことが断られると、長すぎるとは言わず使えない文字が入っていると出る", async () => {
    ingest = () => json(400, [{ id: null, duplicate: false, accepted: false, error: "invalid_note" }]);
    await open();
    typeNote("ベル\u0007の音");
    press("valence-1");
    const bad = await screen.findByTestId("feel-notice-bad");
    expect(bad.textContent).toContain("使えない文字（制御文字）");
    expect(bad.textContent).not.toContain("長すぎ");
    expect(note().value).toBe("ベル\u0007の音");
  });
});

describe("ひとことの上限", () => {
  const EMOJI = "😀";

  // Scenario: 長すぎるひとことは送る前に知らせる
  it("絵文字 2,001 個で欄の直後に「2,001 / 2,000 文字」と長すぎることが出て、2,000 個に戻すと消える", async () => {
    await open();
    typeNote(EMOJI.repeat(2000));
    expect(screen.queryByTestId("feel-note-over")).toBeNull();
    typeNote(EMOJI.repeat(2001));
    const over = screen.getByTestId("feel-note-over");
    expect(over.textContent).toContain("2,001 / 2,000 文字");
    expect(over.textContent).toContain("長すぎ");
    expect(note().nextElementSibling).toBe(over);
    typeNote(EMOJI.repeat(2000));
    expect(screen.queryByTestId("feel-note-over")).toBeNull();
    expect(posts).toHaveLength(0);
  });

  // Scenario: 長すぎるひとことのまま押しても送らない
  it.each([
    ["快を押す", () => press("valence-1")],
    ["元気さを押す", () => press("energy-1")],
    ["欄を離れる", () => fireEvent.blur(note())],
    ["ひとことを残す", () => press("feel-keep-note")],
  ])("2,001 文字のまま「%s」と、送らず、保存していないことと長すぎることが出て、値とひとことが残る", async (name, operate) => {
    await open();
    const long = EMOJI.repeat(2001);
    typeNote(long);
    operate();
    const bad = await screen.findByTestId("feel-notice-bad");
    expect(bad.textContent).toContain("保存していません");
    expect(bad.textContent).toContain("長すぎ");
    expect(bad.textContent).not.toContain("サーバに届きませんでした");
    expect(note().value).toBe(long);
    if (name === "快を押す") expect(screen.getByTestId("valence-1").getAttribute("aria-pressed")).toBe("true");
    if (name === "元気さを押す") expect(screen.getByTestId("energy-1").getAttribute("aria-pressed")).toBe("true");
    await act(() => Promise.resolve());
    expect(posts).toHaveLength(0);
  });
});

describe("開いたとき", () => {
  // Scenario: 開くと最新の記録の値が選ばれている
  it("今日の「日」に +1、続けて -1 の記録があると、-1 が押された状態で出る", async () => {
    records = [rec(-1, { note: "さっき" }), rec(1)];
    await open();
    expect(screen.getByTestId("valence--1").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByTestId("valence-1").getAttribute("aria-pressed")).toBe("false");
    expect(note().value).toBe("さっき");
  });

  // Scenario: もう一方の前後の最新は 1 行で添えられる
  it("昨日の「前」の記録 +2 があるとき、翌日に昨日の画面を開くと、押されたボタンは無く「前に書いた」と +2 が 1 行で出る", async () => {
    records = [rec(2, { phase: "before" })];
    await open("2026-10-07");
    for (const v of ["-2", "-1", "0", "1", "2"]) {
      expect(screen.getByTestId(`valence-${v}`).getAttribute("aria-pressed")).toBe("false");
    }
    const line = screen.getByTestId("feel-other");
    expect(line.textContent).toContain("前に書いた");
    expect(line.textContent).toContain("+2");
  });

  // Scenario: 書き直した履歴を開くと全部が出る
  it("記録が 3 件あるとき「書き直した履歴 3 件」を押すと、値と記入の日時が新しい順に出る", async () => {
    records = [
      rec(-1, { written_at: "2026-10-08T11:30:00+09:00" }),
      rec(2, { e: 1, note: "走った", written_at: "2026-10-08T10:15:00+09:00" }),
      rec(0, { written_at: "2026-10-08T09:05:00+09:00" }),
    ];
    await open();
    expect(screen.queryByTestId("feel-history")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "書き直した履歴 3 件" }));
    const rows = screen.getAllByTestId("feel-history-row").map((r) => r.textContent ?? "");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toContain("-1 不快");
    expect(rows[0]).toContain("11:30");
    expect(rows[1]).toContain("+2 とても快");
    expect(rows[1]).toContain("元気さ +1");
    expect(rows[1]).toContain("「走った」");
    expect(rows[1]).toContain("10:15");
    expect(rows[2]).toContain("0 どちらでもない");
    expect(rows[2]).toContain("9:05");
  });

  // Scenario: 主観を書き換える操作が無い
  it("履歴の各行を押しても入力欄は増えず、書き換えや削除の要求を送る操作も無い", async () => {
    records = [rec(-1), rec(2), rec(0)];
    await open();
    const before = screen.getAllByRole("textbox").length;
    fireEvent.click(screen.getByRole("button", { name: "書き直した履歴 3 件" }));
    for (const row of screen.getAllByTestId("feel-history-row")) fireEvent.click(row);
    expect(screen.getAllByRole("textbox")).toHaveLength(before);
    expect(screen.queryAllByRole("button", { name: /書き換|削除|消す|直す|編集/ })).toHaveLength(0);
    expect(within(screen.getByTestId("feel-history")).queryAllByRole("button")).toHaveLength(0);
    expect(posts).toHaveLength(0);
  });
});

describe("主観の読み出しは 30 日の 1 回", () => {
  // Scenario: 読み出しの失敗は日の欄にも出る
  it("読み出しが失敗した状態で開くと、日の欄と暦の両方に失敗が出て、快–不快のボタンはどれも押されていない", async () => {
    feelingsFail = true;
    await open();
    expect((await screen.findByTestId("feel-notice")).textContent).toContain("気分を読み出せませんでした");
    expect((await screen.findByTestId("feel-calendar-failed")).textContent).toContain("暦を読み出せませんでした");
    // ボタンが無ければ落とす（無いまま検査を抜けない）
    for (const v of [-2, -1, 0, 1, 2]) expect(screen.getByTestId(`valence-${v}`).getAttribute("aria-pressed")).toBe("false");
  });

  it("開くと /api/feelings を対象の日で終わる 30 日で 1 回だけ読み、快–不快を押して受理された後に 1 回だけ増える", async () => {
    await open();
    await screen.findByTestId("feel-calendar");
    expect(gets).toHaveLength(1);
    expect(gets[0]).toBe("/api/feelings?from=2026-09-09&to=2026-10-08");
    press("valence-1");
    await sent(1);
    await screen.findByTestId("feel-notice-ok");
    await waitFor(() => {
      expect(gets).toHaveLength(2);
    });
    // 2 回目を待っただけでは、後から来る 3 回目を捕まえない。落ち着くまで待ってから厳密に数える
    await settle();
    expect(gets).toHaveLength(2);
    expect(gets[1]).toBe(gets[0]);
  });
});

describe("滞在の行は断られたら押された印を戻し、届かなかったら残す", () => {
  const STAY = { kind: "stay", id: "stay-1", start: "2026-10-08T10:00:00+09:00", end: "2026-10-08T10:40:00+09:00" };
  const rejected = (): Response => json(400, [{ id: null, duplicate: false, accepted: false, error: "invalid_feeling_target" }]);
  const pressed = (testid: string): string | null => screen.getByTestId(testid).getAttribute("aria-pressed");

  async function openStay(): Promise<void> {
    stayEntries = [STAY];
    await open();
    await screen.findByTestId("feel-stay-row");
  }

  // Scenario: 滞在の行で断られた値は押された状態に残らない
  it("+1 の記録がある滞在の行の「とても不快」が断られると、断られた文が出て、+1 が押された状態に戻り -2 は押されていない", async () => {
    stayRecords = [rec(1, { target_stay: "stay-1" })];
    ingest = rejected;
    await openStay();
    expect(pressed("stay-stay-1-1")).toBe("true");
    press("stay-stay-1--2");
    await sent(1);
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("書けませんでした");
    await waitFor(() => {
      expect(pressed("stay-stay-1--2")).toBe("false");
    });
    expect(pressed("stay-stay-1-1")).toBe("true");
  });

  // Scenario: 記録の無い滞在の行で断られるとどれも押されていない
  it("記録の無い滞在の行の「快」が断られると、断られた文が出て、その行のどのボタンも押されていない", async () => {
    ingest = rejected;
    await openStay();
    press("stay-stay-1-1");
    await sent(1);
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("書けませんでした");
    await waitFor(() => {
      expect(pressed("stay-stay-1-1")).toBe("false");
    });
    for (const v of [-2, -1, 0, 1, 2]) expect(pressed(`stay-stay-1-${v}`)).toBe("false");
  });

  // Scenario: 滞在の行で届かなかった値は押された状態に残る
  it("要求が届かない状態で滞在の行の「快」を押すと、届かなかった文（断られた文と違う）が出て、+1 が押された状態で残る", async () => {
    ingest = () => Promise.reject(new TypeError("network"));
    await openStay();
    press("stay-stay-1-1");
    const bad = await screen.findByTestId("feel-notice-bad");
    expect(bad.textContent).toContain("サーバに届きませんでした");
    expect(bad.textContent).not.toContain("書けませんでした");
    expect(pressed("stay-stay-1-1")).toBe("true");
  });

  // 応答の届く順が押した順と入れ替わっても、後から押した値の印は消えない（review/code.md R4）。
  // -2 を押す → 返事待ちの間に +1 を押す → +1 が「届かなかった」で先に返る → -2 が「断られた」で後に返る。
  // 断られた枝が外すのは、印が断られた値のときだけ（design D5 は断られた値を戻すのであって、後から押した値を消すのではない）
  it("-2 の返事待ちに +1 を押し、+1 が届かず先に返って -2 が断られて後に返ると、+1 の印と送り直しが残る", async () => {
    let refuseFirst: (r: Response) => void = () => undefined;
    const first = new Promise<Response>((resolve) => {
      refuseFirst = resolve;
    });
    let calls = 0;
    ingest = () => {
      calls += 1;
      return calls === 1 ? first : Promise.reject(new TypeError("network"));
    };
    await openStay();
    press("stay-stay-1--2");
    press("stay-stay-1-1");
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("サーバに届きませんでした");
    expect(pressed("stay-stay-1-1")).toBe("true");
    await act(async () => {
      refuseFirst(rejected());
      await first;
    });
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("書けませんでした");
    expect(pressed("stay-stay-1-1")).toBe("true");
    expect(pressed("stay-stay-1--2")).toBe("false");
    // 届かなかった +1 は送り直しを待っている
    ingest = accepted;
    press("feel-resend");
    await sent(2);
    expect(scalesOf(1)).toEqual([{ id: "valence", version: 1, value: 1 }]);
  });

  /** 1 回目の送信だけ返事を止めておき、2 回目からは `later` の順に返す。止めた 1 回目は戻り値の関数で返す。 */
  function holdFirst(...later: (() => Response | Promise<Response>)[]): (r: Response) => Promise<void> {
    let release: (r: Response) => void = () => undefined;
    const first = new Promise<Response>((resolve) => {
      release = resolve;
    });
    let calls = 0;
    ingest = () => {
      calls += 1;
      return calls === 1 ? first : later[Math.min(calls - 2, later.length - 1)]();
    };
    return async (r: Response) => {
      await act(async () => {
        release(r);
        await first;
      });
    };
  }
  const unreachable = (): Promise<Response> => Promise.reject(new TypeError("network"));

  // 先に押した 1 件が後から受理されても、後から押して届かなかった値の送り直しは消えない（review/code.md R10・R17）
  it("-2 の返事待ちに +1 を押し、+1 が届かず先に返って -2 が受理されて後に返ると、+1 の印と送り直しが残る", async () => {
    const answerFirst = holdFirst(unreachable);
    await openStay();
    press("stay-stay-1--2");
    press("stay-stay-1-1");
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("サーバに届きませんでした");
    await answerFirst(accepted());
    expect((await screen.findByTestId("feel-notice-ok")).textContent).toContain("保存しました");
    expect(pressed("stay-stay-1-1")).toBe("true");
    ingest = accepted;
    press("feel-resend");
    await sent(2);
    expect(scalesOf(1)).toEqual([{ id: "valence", version: 1, value: 1 }]);
  });

  // 断られた 1 件と同じ値を後から押し直していても、その印は消えない（印を外すのは、断られた 1 件が最後に押したものだけ。R18）
  it("+1 の返事待ちに +2（受理）→ +1（届かない）と押し、最初の +1 が断られて後に返ると、+1 の印と送り直しが残る", async () => {
    const answerFirst = holdFirst(accepted, unreachable);
    await openStay();
    press("stay-stay-1-1");
    press("stay-stay-1-2");
    await screen.findByTestId("feel-notice-ok");
    press("stay-stay-1-1");
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("サーバに届きませんでした");
    await answerFirst(rejected());
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("書けませんでした");
    await settle();
    expect(pressed("stay-stay-1-1")).toBe("true");
    expect(screen.getByTestId("feel-resend")).toBeTruthy();
  });

  // 滞在の行の失敗は、日の欄のひとことの控えを古い値に戻さない（戻すと、受理されたひとことを欄を離れたときにもう一度送る。R21）
  it("滞在の行の返事待ちに、ひとことを書いて日の欄の「快」を押して受理され、滞在の行が断られて後に返ると、欄を離れても日の記録を送り直さない", async () => {
    const answerFirst = holdFirst(accepted);
    await openStay();
    press("stay-stay-1-1");
    typeNote("b");
    press("valence-1");
    await screen.findByTestId("feel-notice-ok");
    await answerFirst(rejected());
    await screen.findByTestId("feel-notice-bad");
    await settle();
    fireEvent.blur(note());
    await settle();
    expect(posts).toHaveLength(2);
  });
});

describe("開いたまま日の終わりをまたいで押す", () => {
  /** JST 2026-10-07 23:58 に 2026-10-07 の画面を開く。 */
  async function openBeforeMidnight(): Promise<void> {
    vi.setSystemTime(new Date("2026-10-07T14:58:00Z"));
    window.location.hash = "#/feel/2026-10-07";
    await open("2026-10-07");
  }
  const at = (iso: string): void => {
    vi.setSystemTime(new Date(iso));
  };

  // Scenario: 開いたまま日の終わりをまたいで押した記録は後として格納される
  it("23:58 に開いて 00:01 に「不快」を押すと、送る原文の記入の日時は翌日の 00:01（サーバはこれで「後」に決める）", async () => {
    await openBeforeMidnight();
    at("2026-10-07T15:01:00Z");
    press("valence--1");
    await sent(1);
    expect(posts[0].body.target).toEqual({ kind: "day", date: "2026-10-07" });
    expect(posts[0].body.written_at).toBe("2026-10-08T00:01:00.000+09:00");
  });

  // Scenario: 開いたまま日の終わりをまたいで押すと振り返りとして残したと出る
  it("「前」に +1 がある日を 23:58 に開き 00:01 に「不快」を押すと、保存と「振り返りとして残しました」が出て、-1 が押され、「前に書いた」と +1 が見える", async () => {
    records = [rec(1, { phase: "before" })];
    await openBeforeMidnight();
    expect(screen.getByTestId("valence-1").getAttribute("aria-pressed")).toBe("true");
    at("2026-10-07T15:01:00Z");
    press("valence--1");
    await sent(1);
    const ok = await screen.findByTestId("feel-notice-ok");
    expect(ok.textContent).toContain("保存しました");
    expect(ok.textContent).toContain("振り返りとして残しました");
    await waitFor(() => {
      expect(screen.getByTestId("valence--1").getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByTestId("valence-1").getAttribute("aria-pressed")).toBe("false");
    });
    const other = screen.getByTestId("feel-other").textContent ?? "";
    expect(other).toContain("前に書いた");
    expect(other).toContain("+1");
  });

  // Scenario: 日の終わりをまたがずに押すと振り返りの知らせは出ない
  it("23:58 に開いて 23:59 に「快」を押すと、保存は出て「振り返りとして残しました」は出ない", async () => {
    await openBeforeMidnight();
    at("2026-10-07T14:59:00Z");
    press("valence-1");
    await sent(1);
    expect(posts[0].body.written_at).toBe("2026-10-07T23:59:00.000+09:00");
    const ok = await screen.findByTestId("feel-notice-ok");
    expect(ok.textContent).toContain("保存しました");
    expect(ok.textContent).not.toContain("振り返りとして残しました");
  });

  // Scenario: 日をまたいでから届いた送り直しは振り返りの知らせを出さない
  it("23:59 に届かない状態で「快」を押し、00:01 に届く状態に戻して送り直すと、原文は 23:59 のまま届き、振り返りの知らせは出ない", async () => {
    await openBeforeMidnight();
    at("2026-10-07T14:59:00Z");
    ingest = () => Promise.reject(new TypeError("network"));
    press("valence-1");
    expect((await screen.findByTestId("feel-notice-bad")).textContent).toContain("サーバに届きませんでした");
    at("2026-10-07T15:01:00Z");
    ingest = accepted;
    press("feel-resend");
    // 届かなかった 1 回目は posts に残らない（届いた分だけ数える）。届いたのは原文そのまま
    await sent(1);
    expect(posts[0].body.written_at).toBe("2026-10-07T23:59:00.000+09:00");
    const ok = await screen.findByTestId("feel-notice-ok");
    expect(ok.textContent).toContain("保存しました");
    expect(ok.textContent).not.toContain("振り返りとして残しました");
  });
});
