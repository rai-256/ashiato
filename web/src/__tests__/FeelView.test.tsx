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

beforeEach(() => {
  seq = 0;
  posts = [];
  gets = [];
  feelingsFail = false;
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
      if (url.startsWith("/api/stays")) return Promise.resolve(json(200, { date: "", criteria: [], entries: [] }));
      if (url.startsWith("/api/feelings/reminder")) return Promise.resolve(json(200, { at: "21:00", recorded: false }));
      gets.push(url);
      if (feelingsFail) return Promise.resolve(json(500, {}));
      const date = /to=([\d-]+)/.exec(url)?.[1] ?? "";
      return Promise.resolve(json(200, view(date, records)));
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
    for (const v of [-2, -1, 0, 1, 2]) {
      const b = screen.queryByTestId(`valence-${v}`);
      if (b !== null) expect(b.getAttribute("aria-pressed")).toBe("false");
    }
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
    expect(gets[1]).toBe(gets[0]);
  });
});
