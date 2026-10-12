// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の画面の要約とほかの記録の行（ST25 / tasks 8.1 / design D7）。固定の `/api/day` の応答を描いて文字列を確かめる。
 */
import { render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DayView } from "../DayView";
import { SUMMARY_TOP, summaryLines } from "../timeline";

const DATE = "2026-09-07";
const at = (hm: string): string => `${DATE}T${hm}:00+09:00`;
const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;

function serve(entries: unknown[], extra: Record<string, unknown> = {}): void {
  vi.stubGlobal("fetch", (path: string) => {
    if (path.startsWith("/api/day")) return Promise.resolve(ok({ date: DATE, criteria: [], entries, gaps: [], ...extra }));
    return new Promise<Response>(() => {});
  });
}

const stay = (summary?: unknown) => ({ kind: "stay", start: at("09:00"), end: at("12:00"), id: "s1", criteria_id: 1, ...(summary ? { summary } : {}) });

beforeEach(() => {
  window.location.hash = "";
});
afterEach(() => {
  vi.unstubAllGlobals();
});

async function rows(): Promise<HTMLElement> {
  render(<DayView date={DATE} />);
  return screen.findByRole("list", { name: "1 日の並び" });
}

describe("要約の文字", () => {
  it("上位の数は 3", () => {
    expect(SUMMARY_TOP).toBe(3);
  });

  // Scenario: 滞在の行に PC のアプリの上位 3 つと時間が出る
  it("PC の合計と上位 3 つのアプリが時間つきでこの順に出る", async () => {
    serve([
      stay({
        pc: {
          seconds: 9900,
          top: [
            { app: "VS Code", seconds: 5400 },
            { app: "Chrome", seconds: 3000 },
            { app: "Slack", seconds: 1200 },
            { app: "Terminal", seconds: 300 },
          ],
        },
      }),
    ]);
    const list = await rows();
    const line = within(list).getByTestId("summary-pc").textContent ?? "";
    expect(line).toBe("PC 2 時間 45 分 — VS Code 1:30 · Chrome 0:50 · Slack 0:20");
    expect(line).not.toContain("Terminal");
  });

  // Scenario: ブラウザは見たサイトのドメインの上位 3 つと件数が出る
  it("ブラウザの件数と上位 3 つのドメインが出る", async () => {
    serve([
      stay({
        browser: {
          count: 32,
          top: [
            { domain: "github.com", count: 18 },
            { domain: "docs.rs", count: 7 },
            { domain: "example.com", count: 5 },
            { domain: "news.example.org", count: 2 },
          ],
        },
      }),
    ]);
    const list = await rows();
    const line = within(list).getByTestId("summary-browser").textContent ?? "";
    expect(line).toBe("ブラウザ 32 件 — github.com 18 · docs.rs 7 · example.com 5");
    expect(line).not.toContain("news.example.org");
  });

  // Scenario: スマホのアプリの上位 3 つと時間が出る
  it("スマホの使った時間と上位 3 つのアプリが出る", async () => {
    serve([
      { kind: "move", start: at("12:00"), end: at("12:42"), summary: { phone: { seconds: 2400, top: [{ app: "Maps", seconds: 1200 }, { app: "Mail", seconds: 600 }, { app: "News", seconds: 400 }, { app: "Game", seconds: 200 }] } } },
    ]);
    const list = await rows();
    expect(within(list).getByTestId("summary-phone").textContent).toBe("スマホ 40 分 — Maps 0:20 · Mail 0:10 · News 0:06");
  });

  // Scenario: 書庫の記録は表示名ごとの件数で出る
  it("書庫は「書庫: 表示名 件数」で出る", async () => {
    serve([stay({ archive: [{ display_name: "Chrome の履歴", count: 12 }] })]);
    const list = await rows();
    expect(within(list).getByTestId("summary-archive").textContent).toBe("書庫: Chrome の履歴 12 件");
  });

  // Scenario: 記録の無いソースの要約は出ない
  it("記録の無いソースの行は出ない", async () => {
    serve([stay({ pc: { seconds: 600, top: [{ app: "VS Code", seconds: 600 }] } })]);
    const list = await rows();
    expect(within(list).getByTestId("summary-pc")).toBeTruthy();
    for (const k of ["phone", "browser", "archive"]) expect(within(list).queryByTestId(`summary-${k}`)).toBeNull();
  });

  // Scenario: 記録なしの行にもその時間の要約が出る
  it("記録なしの行にも PC の要約が出る", async () => {
    serve([{ kind: "no-record", start: at("20:00"), end: at("23:00"), summary: { pc: { seconds: 3600, top: [{ app: "VS Code", seconds: 3600 }] } } }]);
    const list = await rows();
    const row = within(list).getByTestId("row-no-record");
    expect(row.textContent).toContain("記録なし 20:00 – 23:00");
    expect(within(row).getByTestId("summary-pc").textContent).toBe("PC 1 時間 0 分 — VS Code 1:00");
  });

  // Scenario: ソースの種類は文字で出る
  it("要約の各行が種類の文字で始まる", () => {
    const lines = summaryLines({
      pc: { seconds: 60, top: [] },
      phone: { seconds: 60, top: [] },
      browser: { count: 1, top: [] },
      archive: [{ display_name: "X", count: 1 }],
    });
    expect(lines.map((l) => l.text.split(" ")[0])).toEqual(["PC", "スマホ", "ブラウザ", "書庫:"]);
  });

  // Scenario: 消した行には要約が出ない
  it("消した行に要約は出ない", async () => {
    serve([{ kind: "erased", start: at("10:00"), end: at("11:00"), stay_ids: ["s1"] }]);
    const list = await rows();
    const row = within(list).getByTestId("row-erased");
    expect(row.textContent).toContain("消した 10:00 – 11:00");
    expect(within(row).queryByTestId("summary")).toBeNull();
  });

  // Scenario: どの行にも入らない時間の記録はほかの記録の行に出る
  it("消した行の直後にほかの記録の行が出て PC の要約を持つ", async () => {
    serve([
      { kind: "erased", start: at("10:00"), end: at("11:00"), stay_ids: ["s1"] },
      { kind: "other", start: at("10:00"), end: at("11:00"), summary: { pc: { seconds: 1800, top: [{ app: "VS Code", seconds: 1800 }] } } },
    ]);
    const list = await rows();
    const items = within(list).getAllByRole("listitem");
    expect(items[0].getAttribute("data-kind")).toBe("erased");
    expect(items[1].textContent).toContain("ほかの記録 10:00 – 11:00");
    expect(within(items[1]).getByTestId("summary-pc").textContent).toContain("PC 30 分");
  });

  // Scenario: 今日の最後の位置の後の記録は記録なしの行に出る
  it("並びの終わりの記録なしの行に PC の要約が出て、ほかの記録の行は無い", async () => {
    serve([
      stay(),
      { kind: "no-record", start: at("13:00"), end: at("14:30"), summary: { pc: { seconds: 600, top: [{ app: "VS Code", seconds: 600 }] } } },
    ]);
    const list = await rows();
    const items = within(list).getAllByRole("listitem");
    const last = items[items.length - 1];
    expect(last.getAttribute("data-kind")).toBe("no-record");
    expect(last.textContent).toContain("記録なし 13:00 – 14:30");
    expect(within(last).getByTestId("summary-pc").textContent).toContain("PC");
    expect(within(list).queryByTestId("row-other")).toBeNull();
  });

  // 合計と上位の各アプリは同じ丸め（切り捨て）。1 つのアプリだけの行で合計と内訳が食い違わない（R12(b)）
  it("165 分 30 秒は合計も内訳も 2 時間 45 分", () => {
    const [line] = summaryLines({ pc: { seconds: 9930, top: [{ app: "VS Code", seconds: 9930 }] } });
    expect(line.text).toBe("PC 2 時間 45 分 — VS Code 2:45");
    const [short] = summaryLines({ phone: { seconds: 59, top: [{ app: "LINE", seconds: 59 }] } });
    expect(short.text).toBe("スマホ 0 分 — LINE 0:00");
  });

  it("形が違う応答は失敗として出す", async () => {
    serve([stay({ pc: { seconds: "x", top: [] } })]);
    render(<DayView date={DATE} />);
    await waitFor(() => expect(screen.getByTestId("day-error").textContent).toContain("unexpected_shape"));
  });
});

// サーバの試験（`crates/server/src/day_tests.rs` の同じ Scenario）が返す値をそのまま描き、画面の文字を確かめる（code-verify R25）
describe("サーバが数えた要約を画面の文字にする", () => {
  const pcLine = async (summary: unknown): Promise<string> => {
    serve([stay(summary)]);
    const list = await rows();
    return within(list).getByTestId("summary-pc").textContent ?? "";
  };

  // Scenario: 離席の時間はアプリを使った時間に数えない
  it("10:00 の VS Code が 10:15〜10:40 の離席を除いて 25 分", async () => {
    const line = await pcLine({ pc: { seconds: 1500 + 4200, top: [{ app: "Slack", seconds: 4200 }, { app: "VS Code", seconds: 1500 }] } });
    expect(line).toContain("VS Code 0:25");
  });

  // Scenario: 10 分に満たない離席も使った時間から差し引く
  it("10:00 の VS Code が 10:20〜10:26 の離席を除いて 24 分", async () => {
    const line = await pcLine({ pc: { seconds: 1440 + 5400, top: [{ app: "Slack", seconds: 5400 }, { app: "VS Code", seconds: 1440 }] } });
    expect(line).toContain("VS Code 0:24");
  });

  // Scenario: スマホの使った時間は 1 回 10 分で打ち切る
  it("22:00 の地図は 10 分で打ち切られて 0:10", async () => {
    serve([stay({ phone: { seconds: 1200, top: [{ app: "地図", seconds: 600 }, { app: "メール", seconds: 600 }] } })]);
    const list = await rows();
    const line = within(list).getByTestId("summary-phone").textContent ?? "";
    expect(line).toBe("スマホ 20 分 — 地図 0:10 · メール 0:10");
  });

  // Scenario: 同じ時間のアプリは名前の順に並ぶ
  it("Mail と Slack が同じ 20 分なら Mail · Slack の順", async () => {
    const line = await pcLine({ pc: { seconds: 2400, top: [{ app: "Mail", seconds: 1200 }, { app: "Slack", seconds: 1200 }] } });
    expect(line).toBe("PC 40 分 — Mail 0:20 · Slack 0:20");
  });

  // Scenario: ドメインは www を外して小文字で束ねる
  it("WWW.Example.com と example.com は example.com 2 の 1 つ", async () => {
    serve([stay({ browser: { count: 2, top: [{ domain: "example.com", count: 2 }] } })]);
    const list = await rows();
    expect(within(list).getByTestId("summary-browser").textContent).toBe("ブラウザ 2 件 — example.com 2");
  });

  // Scenario: 日付をまたぐ行の要約は両方の日で同じ
  it("23:00〜翌 2:00 の滞在は、前の日の画面でも次の日の画面でも同じ要約の文字", async () => {
    const summary = { pc: { seconds: 7200, top: [{ app: "Code", seconds: 7200 }] } };
    const across = { kind: "stay", start: at("23:00"), end: "2026-09-08T02:00:00+09:00", id: "s1", criteria_id: 1, summary };
    const texts: string[] = [];
    for (const date of [DATE, "2026-09-08"]) {
      vi.stubGlobal("fetch", (path: string) =>
        path.startsWith("/api/day")
          ? Promise.resolve(ok({ date, criteria: [], entries: [across], gaps: [] }))
          : new Promise<Response>(() => {}),
      );
      const { unmount } = render(<DayView date={date} />);
      const list = await screen.findByRole("list", { name: "1 日の並び" });
      texts.push(within(list).getByTestId("summary-pc").textContent ?? "");
      unmount();
    }
    expect(texts[0]).toBe("PC 2 時間 0 分 — Code 2:00");
    expect(texts[1]).toBe(texts[0]);
  });
});
