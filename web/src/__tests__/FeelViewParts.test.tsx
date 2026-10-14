// SPDX-License-Identifier: AGPL-3.0-only
/**
 * S-3 の滞在の行・暦・通知の時刻の設定（ST17 / tasks 9.1 / design D13・D14・D10）。
 *
 * `fetch` は差し替え、時刻は `Date` だけ固定する。画面の Scenario は `web/e2e/feel-parts.spec.ts`（本物のブラウザ）が担保する。
 * ここは文字と属性の組み立てを細かく撃つ追加。
 */
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import dayFeelingSource from "../DayFeeling.tsx?raw";
import feelPartsSource from "../FeelParts.tsx?raw";
import feelViewSource from "../FeelView.tsx?raw";
import { FeelView } from "../FeelView";
import { calendarDays, type DayStatus, type Feeling, type StayBucket } from "../feelings";
import { Root } from "../Root";
import type { DayEntry } from "../stays";

/** JST 2026-10-08 12:00 */
const NOON = "2026-10-08T03:00:00Z";
const TARGET = "2026-10-08";

const json = (status: number, body: unknown): Response => ({ ok: status < 400, status, json: () => Promise.resolve(body) }) as Response;

let seq = 0;
function rec(v: number, over: Partial<Feeling> = {}): Feeling {
  seq += 1;
  return {
    id: `f-${seq}`,
    scales: [{ id: "valence", version: 1, value: v }],
    note: null,
    written_at: `2026-10-07T${String(10 + seq).padStart(2, "0")}:00:00+09:00`,
    ingested_at: "2026-10-07T02:00:00Z",
    phase: "before",
    target_stay: null,
    ...over,
  };
}

const stayEntry = (id: string, start: string, end: string): DayEntry => ({ kind: "stay", id, start, end });
const jst = (date: string, hm: string): string => `${date}T${hm}:00+09:00`;

interface DaySpec {
  status?: DayStatus;
  records?: Feeling[];
  stays?: StayBucket[];
  hidden?: number;
}

/** 偽のサーバの状態。 */
let entries: DayEntry[];
let days: Record<string, DaySpec>;
let staysFail: boolean;
let ingestFails: boolean;
let feelingsFail: boolean;
let reminderAt: string;
let reminderPostFails: boolean;
let posts: { path: string; body: Record<string, unknown> }[];

const latestOf = (records: Feeling[], phase: Feeling["phase"]): Feeling | null => records.find((r) => r.phase === phase) ?? null;

function feelingsBody(from: string, to: string): unknown {
  const out = [];
  for (let d = from; d <= to; d = new Date(Date.parse(`${d}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10)) {
    const spec = days[d] ?? {};
    const records = spec.records ?? [];
    out.push({
      date: d,
      status: spec.status ?? "missing",
      day: { latest: { before: latestOf(records, "before"), after: latestOf(records, "after") }, records },
      stays: spec.stays ?? [],
      stays_with_records: (spec.stays ?? []).length,
      hidden_stay_records: spec.hidden ?? 0,
    });
  }
  return { today: TARGET, first_day: null, days: out };
}

beforeEach(() => {
  seq = 0;
  entries = [];
  days = {};
  staysFail = false;
  ingestFails = false;
  feelingsFail = false;
  reminderAt = "21:00";
  reminderPostFails = false;
  posts = [];
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(NOON));
  window.location.hash = "";
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string, init?: RequestInit) => {
      if (init?.method === "POST") {
        if (url === "/api/feelings/reminder") {
          if (reminderPostFails) return Promise.reject(new TypeError("network"));
          const at = (JSON.parse(init.body as string) as { at: string }).at;
          posts.push({ path: url, body: { at } });
          reminderAt = at;
          return Promise.resolve(json(200, { at }));
        }
        const item = (JSON.parse(init.body as string) as Record<string, unknown>[])[0];
        if (ingestFails) return Promise.resolve(json(503, null));
        posts.push({ path: url, body: JSON.parse(item.raw as string) as Record<string, unknown> });
        return Promise.resolve(json(200, [{ id: "x", duplicate: false, accepted: true, error: null }]));
      }
      if (url.startsWith("/api/stays")) {
        return Promise.resolve(staysFail ? json(500, {}) : json(200, { date: TARGET, criteria: [], entries }));
      }
      if (url.startsWith("/api/feelings/reminder")) return Promise.resolve(json(200, { at: reminderAt, recorded: false }));
      if (feelingsFail) return Promise.resolve(json(500, {}));
      const from = /from=([\d-]+)/.exec(url)?.[1] ?? "";
      const to = /to=([\d-]+)/.exec(url)?.[1] ?? "";
      return Promise.resolve(json(200, feelingsBody(from, to)));
    }),
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function open(date: string | null = TARGET): Promise<void> {
  render(<FeelView date={date} />);
  await screen.findByTestId("valence-1");
  await waitFor(() => {
    expect(screen.queryByTestId("feel-stays-loading")).toBeNull();
  });
}

const rows = (): HTMLElement[] => screen.queryAllByTestId("feel-stay-row");
const calMark = (date: string): string => within(screen.getByTestId(`cal-${date}`)).getByTestId("cal-mark").textContent ?? "";
const isDashed = (date: string): boolean => screen.getByTestId(`cal-${date}`).style.border.includes("dashed");

describe("滞在の行", () => {
  const three = (): DayEntry[] => [
    stayEntry("s3", jst(TARGET, "15:00"), jst(TARGET, "16:00")),
    stayEntry("s1", jst(TARGET, "08:00"), jst(TARGET, "09:30")),
    stayEntry("s2", jst(TARGET, "11:00"), jst(TARGET, "12:00")),
  ];

  // Scenario: その日の滞在が始まりの順に並ぶ
  it("滞在 3 件が時刻の範囲とともに始まりの順に並ぶ", async () => {
    entries = three();
    await open();
    expect(rows().map((r) => r.dataset.stay)).toEqual(["s1", "s2", "s3"]);
    expect(screen.getAllByTestId("feel-stay-range").map((e) => e.textContent)).toEqual(["8:00 – 9:30", "11:00 – 12:00", "15:00 – 16:00"]);
  });

  // Scenario: 滞在の行を押すとその滞在に保存される
  it("2 件目の行の快を押すと、2 件目の識別子と始まり・終わりを持つ快–不快 +1 が 1 件送られる", async () => {
    entries = three();
    await open();
    fireEvent.click(screen.getByTestId("stay-s2-1"));
    await waitFor(() => {
      expect(posts).toHaveLength(1);
    });
    expect(posts[0].body.target).toEqual({ kind: "stay", stay: "s2", start: jst(TARGET, "11:00"), end: jst(TARGET, "12:00") });
    expect(posts[0].body.scales).toEqual([{ id: "valence", version: 1, value: 1 }]);
    expect(screen.getByTestId("stay-s2-1").getAttribute("aria-pressed")).toBe("true");
    expect((await screen.findByTestId("feel-notice-ok")).textContent).toContain("滞在 11:00 – 12:00");
  });

  // Scenario: 届かなかった記録は同じ原文で送り直す
  it("「日」が届かないまま滞在の行を押しても、送り直しで「日」の 1 件も滞在の 1 件も送られる", async () => {
    entries = three();
    ingestFails = true;
    await open();
    fireEvent.click(screen.getByTestId("valence-1"));
    await screen.findByTestId("feel-resend");
    fireEvent.click(screen.getByTestId("stay-s2-1"));
    await waitFor(() => {
      expect(screen.getByTestId("stay-s2-1").getAttribute("aria-pressed")).toBe("true");
    });
    ingestFails = false;
    fireEvent.click(await screen.findByTestId("feel-resend"));
    await waitFor(() => {
      expect(posts).toHaveLength(2);
    });
    expect(posts.map((p) => (p.body.target as { kind: string }).kind).sort()).toEqual(["day", "stay"]);
  });

  // Scenario: 滞在の行に最新の気分が出る
  it("快–不快 -2 の記録がある滞在は、-2 のボタンが押された状態で出る", async () => {
    entries = three();
    days[TARGET] = { stays: [{ stay: "s1", latest: { before: null, after: null }, records: [rec(-2)] }] };
    await open();
    const row = rows()[0];
    expect(within(row).getByTestId("stay-s1--2").getAttribute("aria-pressed")).toBe("true");
    expect(within(row).getByTestId("stay-s1-1").getAttribute("aria-pressed")).toBe("false");
  });

  // Scenario: 前の日から続く滞在の行は前の日からと出る
  it("2026-10-07 23:10 から 2026-10-08 07:30 までの滞在の行に「前の日から」が添えてある", async () => {
    entries = [stayEntry("sc", jst("2026-10-07", "23:10"), jst(TARGET, "07:30")), stayEntry("sn", jst(TARGET, "09:00"), jst(TARGET, "10:00"))];
    await open();
    expect(within(rows()[0]).getByTestId("feel-stay-carried").textContent).toBe("前の日から");
    expect(within(rows()[1]).queryByTestId("feel-stay-carried")).toBeNull();
  });

  // Scenario: 前の日から続く滞在に書いた気分はその行に出る
  it("前の日に数える記録は前の日の分から引かれ、その行の +1 が押された状態で出て、対象の日は記入ありにならない", async () => {
    entries = [stayEntry("sc", jst("2026-10-07", "23:10"), jst(TARGET, "07:30"))];
    days["2026-10-07"] = { status: "recorded", stays: [{ stay: "sc", latest: { before: null, after: null }, records: [rec(1)] }] };
    days[TARGET] = { status: "missing" };
    await open();
    expect(within(rows()[0]).getByTestId("stay-sc-1").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByTestId(`cal-${TARGET}`).dataset.status).toBe("missing");
  });

  // Scenario: 滞在の書き直した履歴を開ける
  it("記録 2 件の滞在は「書き直した履歴 2 件」を押すと、記入の日時の新しい順に 2 件が出る", async () => {
    entries = [stayEntry("s1", jst(TARGET, "08:00"), jst(TARGET, "09:00"))];
    const older = rec(-1, { written_at: jst("2026-10-08", "08:30") });
    const newer = rec(2, { written_at: jst("2026-10-08", "09:30") });
    days[TARGET] = { stays: [{ stay: "s1", latest: { before: newer, after: null }, records: [older, newer] }] };
    await open();
    expect(screen.queryByTestId("feel-stay-history")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "書き直した履歴 2 件" }));
    const lines = screen.getAllByTestId("feel-stay-history-row").map((r) => r.textContent ?? "");
    expect(lines).toHaveLength(2);
    expect(lines[0]).toContain("+2");
    expect(lines[0]).toContain("9:30");
    expect(lines[1]).toContain("-1");
    expect(lines[1]).toContain("8:30");
  });

  // Scenario: 消した滞在は気分の行に並ばない
  it("/stays に出ない滞在は、記録があっても行にならない", async () => {
    entries = [stayEntry("s1", jst(TARGET, "08:00"), jst(TARGET, "09:00"))];
    days[TARGET] = { stays: [{ stay: "gone", latest: { before: null, after: null }, records: [rec(2)] }] };
    await open();
    expect(rows().map((r) => r.dataset.stay)).toEqual(["s1"]);
  });

  // Scenario: 消した滞在の気分は数だけ出る
  it("消した滞在の記録が 1 件あると「消した滞在の気分 1 件」の行が出て、値は出ない", async () => {
    days[TARGET] = { hidden: 1 };
    await open();
    expect(screen.getByTestId("feel-stays-hidden").textContent).toBe("消した滞在の気分 1 件（ここには出しません）");
    expect(rows()).toHaveLength(0);
  });

  // Scenario: 吸収された滞在の気分は吸収先の行に出る
  it("吸収先の滞在 B の行に、吸収された A に書いた +2 が押された状態で出る", async () => {
    entries = [stayEntry("B", jst(TARGET, "10:00"), jst(TARGET, "11:40"))];
    days[TARGET] = { stays: [{ stay: "B", latest: { before: null, after: null }, records: [rec(2, { target_stay: "A" })] }] };
    await open();
    expect(within(rows()[0]).getByTestId("stay-B-2").getAttribute("aria-pressed")).toBe("true");
  });

  // Scenario: 滞在の無い日は滞在が無いと出る
  it("滞在が 1 件も無い日は、滞在が無いことを示す文が出る", async () => {
    await open();
    expect(screen.getByTestId("feel-stays-empty").textContent).toBe("この日の滞在はありません");
    expect(screen.queryByTestId("feel-stays-failed")).toBeNull();
  });

  // Scenario: 滞在の読み出しの失敗は滞在が無いと出ない
  it("滞在の読み出しが失敗したら、失敗が出て、滞在が無いことを示す文は出ない", async () => {
    staysFail = true;
    render(<FeelView date={TARGET} />);
    const failed = await screen.findByTestId("feel-stays-failed");
    expect(failed.textContent).toContain("滞在を読み出せませんでした");
    expect(screen.queryByTestId("feel-stays-empty")).toBeNull();
  });

  it("滞在の行のボタンは 44 px で、数値だけを見せて言葉は aria-label に持つ", async () => {
    entries = [stayEntry("s1", jst(TARGET, "08:00"), jst(TARGET, "09:00"))];
    await open();
    const button = screen.getByTestId("stay-s1-2");
    expect(button.style.minHeight).toBe("44px");
    expect(button.textContent).toBe("+2");
    expect(button.getAttribute("aria-label")).toBe("+2 とても快");
  });
});

describe("暦", () => {
  // Scenario: 暦に未記入の日が破線と未で出る
  it("30 日のうち未記入の 9 日だけに破線の枠と「未」が出る", async () => {
    const dates = calendarDays(TARGET);
    dates.forEach((d, i) => {
      days[d] = { status: i < 9 ? "missing" : "recorded", records: i < 9 ? [] : [rec(0, { id: `r${i}` })] };
    });
    await open();
    const missing = dates.filter((d) => calMark(d) === "未");
    expect(missing).toEqual(dates.slice(0, 9));
    for (const d of dates) expect(isDashed(d)).toBe(missing.includes(d));
  });

  // Scenario: 暦に記入ありの日の値が出る
  it("「日」の最新が +1 の日の欄に +1 が出る", async () => {
    days["2026-10-05"] = { status: "recorded", records: [rec(1)] };
    await open();
    expect(calMark("2026-10-05")).toBe("+1");
    expect(isDashed("2026-10-05")).toBe(false);
  });

  // Scenario: 前と後がある日の暦は後の値を出す
  it("前の記録（+2）と後の記録（-1）がある日の欄は -1", async () => {
    days["2026-10-04"] = { status: "recorded", records: [rec(-1, { phase: "after" }), rec(2, { phase: "before" })] };
    await open();
    expect(calMark("2026-10-04")).toBe("-1");
  });

  it("「日」の記録が無く滞在にだけある日は「滞」", async () => {
    days["2026-10-03"] = { status: "recorded", records: [] };
    await open();
    expect(calMark("2026-10-03")).toBe("滞");
  });

  // Scenario: 使い始める前の日は未記入と出ない
  it("使い始める前の日は破線も「未」も付かない", async () => {
    const dates = calendarDays(TARGET);
    dates.forEach((d, i) => {
      days[d] = { status: i < 19 ? "before_start" : "missing" };
    });
    await open();
    for (const d of dates.slice(0, 19)) {
      expect(calMark(d)).toBe("");
      expect(isDashed(d)).toBe(false);
    }
    expect(calMark(dates[19])).toBe("未");
  });

  // Scenario: これからの日は暦で未記入と出ない
  it("3 日後を対象にすると、今日より後の日に破線も「未」も付かない", async () => {
    const target = "2026-10-11";
    const dates = calendarDays(target);
    for (const d of dates) days[d] = { status: d > TARGET ? "future" : "missing" };
    await open(target);
    for (const d of dates.filter((x) => x > TARGET)) {
      expect(calMark(d)).toBe("");
      expect(isDashed(d)).toBe(false);
    }
    expect(calMark(TARGET)).toBe("未");
  });

  // Scenario: 暦は対象の日で終わる 30 日
  it("2026-10-07 を対象にすると 2026-09-08 から 2026-10-07 までの 30 日を、日曜始まりの列に出す", async () => {
    await open("2026-10-07");
    const cells = screen.getAllByTestId(/^cal-\d{4}-/).map((e) => e.dataset.testid);
    expect(cells).toHaveLength(30);
    expect(cells[0]).toBe("cal-2026-09-08");
    expect(cells[29]).toBe("cal-2026-10-07");
    expect(m_weekdays()).toEqual(["日", "月", "火", "水", "木", "金", "土"]);
  });

  // Scenario: 暦の日を押すとその日を書ける
  it("暦の 3 日前の日を押すと、見出しの日付が 3 日前になる", async () => {
    window.location.hash = `#/feel/${TARGET}`;
    render(<Root />);
    await screen.findByTestId("valence-1");
    fireEvent.click(await screen.findByTestId("cal-2026-10-05"));
    await waitFor(() => {
      expect(screen.getByTestId("feel-title").querySelector("time")?.getAttribute("datetime")).toBe("2026-10-05");
    });
  });
});

const m_weekdays = (): string[] =>
  Array.from(screen.getByTestId("feel-calendar").querySelectorAll("span[aria-hidden='true']"))
    .map((e) => e.textContent ?? "")
    .filter((t) => t !== "");

describe("通知の時刻", () => {
  const input = (): HTMLInputElement => screen.getByTestId("feel-reminder-at") as HTMLInputElement;

  // Scenario: 画面で通知の時刻を変えられる
  it("22:30 にして保存すると、いまの通知の時刻が 22:30 になる", async () => {
    await open();
    await waitFor(() => {
      expect(screen.getByTestId("feel-reminder-current").textContent).toBe("いまの通知の時刻: 21:00");
    });
    fireEvent.change(input(), { target: { value: "22:30" } });
    await act(async () => {
      fireEvent.click(screen.getByTestId("feel-reminder-save"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("feel-reminder-current").textContent).toBe("いまの通知の時刻: 22:30");
    });
    expect(screen.getByTestId("feel-reminder-ok").textContent).toBe("通知の時刻を 22:30 にしました");
  });

  // Scenario: 画面で変えた通知の時刻が通知の判断に効く
  it("保存は POST /feelings/reminder に HH:MM を送り、次に通知の判断を読むと 22:30 が返る", async () => {
    await open();
    await waitFor(() => {
      expect(input().value).toBe("21:00");
    });
    fireEvent.change(input(), { target: { value: "22:30" } });
    await act(async () => {
      fireEvent.click(screen.getByTestId("feel-reminder-save"));
    });
    await waitFor(() => {
      expect(posts.filter((p) => p.path === "/api/feelings/reminder")).toEqual([{ path: "/api/feelings/reminder", body: { at: "22:30" } }]);
    });
    const decision = (await (await fetch(`/api/feelings/reminder?date=${TARGET}`)).json()) as { at: string };
    expect(decision.at).toBe("22:30");
  });

  // Scenario: 通知の時刻を保存できなかったとき入れた値が残る
  it("サーバに届かないとき、保存できなかったことが出て、欄は 22:30 のまま", async () => {
    reminderPostFails = true;
    await open();
    await waitFor(() => {
      expect(input().value).toBe("21:00");
    });
    fireEvent.change(input(), { target: { value: "22:30" } });
    await act(async () => {
      fireEvent.click(screen.getByTestId("feel-reminder-save"));
    });
    expect((await screen.findByTestId("feel-reminder-bad")).textContent).toContain("保存できませんでした");
    expect(input().value).toBe("22:30");
    expect(screen.getByTestId("feel-reminder-current").textContent).toBe("いまの通知の時刻: 21:00");
  });

  it("時刻が読めない欄は保存できない", async () => {
    await open();
    await waitFor(() => {
      expect(input().value).toBe("21:00");
    });
    fireEvent.change(input(), { target: { value: "" } });
    expect((screen.getByTestId("feel-reminder-save") as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("読み出しの失敗", () => {
  // Scenario: 読み出しの失敗と未記入を区別する
  it("主観の読み出しが失敗したら失敗が出て、暦に「未」は出ない", async () => {
    feelingsFail = true;
    render(<FeelView date={TARGET} />);
    expect((await screen.findByTestId("feel-calendar-failed")).textContent).toContain("暦を読み出せませんでした");
    expect(screen.getByTestId("feel-notice").textContent).toContain("気分を読み出せませんでした");
    for (const e of screen.getAllByTestId("cal-mark")) expect(e.textContent).toBe("");
    expect(screen.getAllByTestId(/^cal-\d{4}-/).every((e) => !e.style.border.includes("dashed"))).toBe(true);
  });
});

describe("色の出所と下限", () => {
  // Scenario: 主観入力の画面は確定した色だけを使う
  it("`FeelView.tsx` / `FeelParts.tsx` / `DayFeeling.tsx` に色の直書きが無く、色は `tone` と `SCHEMES` からだけ引く", () => {
    // 新しい色関数も網に入れる（`MasterView` の同じ検査と同じ式）。名前付き色は `transparent` だけ許す
    for (const [name, source] of [
      ["FeelView.tsx", feelViewSource],
      ["FeelParts.tsx", feelPartsSource],
      ["DayFeeling.tsx", dayFeelingSource],
    ] as const) {
      const literals = source.match(/#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/g) ?? [];
      expect(literals, `${name} に色が直書きされている: ${literals.join(" / ")}`).toHaveLength(0);
      const named = source.match(/\b(?:background|color|outline|fill|stroke)\s*:\s*"(?!transparent")[a-zA-Z]+"/g) ?? [];
      expect(named, `${name} に名前付き色が直書きされている: ${named.join(" / ")}`).toHaveLength(0);
    }
    expect(feelPartsSource).toContain('from "./tokens"');
    expect(feelPartsSource).toContain("tone(");
    expect(feelViewSource).toContain('from "./tokens"');
    expect(feelViewSource).toContain("tone(");
  });

  // Scenario: 快不快のボタンは高さ 64 px 以上
  it("快–不快の 5 つのボタンは高さ 64 px 以上を指定している（実寸は e2e が測る）", async () => {
    render(<FeelView date={TARGET} />);
    for (let v = -2; v <= 2; v++) {
      const b = await screen.findByTestId(`valence-${v}`);
      expect(Number.parseFloat(b.style.minHeight), `valence-${v}`).toBeGreaterThanOrEqual(64);
    }
  });
});
