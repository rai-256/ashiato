// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 取れていない時間の行・記録なしの事情（ST25 / tasks 8.2 / design D9 / D11）。固定の `/api/day` の応答の文字と並びを確かめる。
 */
import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DayView } from "../DayView";
import type { DayEntry } from "../stays";
import { AWAY_ROW_MIN_MINUTES, placeGaps, type Gap } from "../timeline";

const DATE = "2026-09-07";
const at = (hm: string): string => `${DATE}T${hm}:00+09:00`;
const atS = (hms: string): string => `${DATE}T${hms}+09:00`;
const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;

afterEach(() => {
  vi.unstubAllGlobals();
});

async function show(entries: unknown[], gaps: unknown[] = [], extra: Record<string, unknown> = {}): Promise<HTMLElement> {
  vi.stubGlobal("fetch", (path: string) =>
    path.startsWith("/api/day") ? Promise.resolve(ok({ date: DATE, criteria: [], entries, gaps, ...extra })) : new Promise<Response>(() => {}),
  );
  render(<DayView date={DATE} />);
  return screen.findByRole("list", { name: "1 日の並び" });
}
const texts = (list: HTMLElement): string[] => within(list).getAllByRole("listitem").map((li) => (li.textContent ?? "").trim());

const stay = { kind: "stay", start: at("09:00"), end: at("12:00"), id: "s1", criteria_id: 1 };
const move = { kind: "move", start: at("12:00"), end: at("12:42") };

describe("取れていない時間の行の文字", () => {
  it("離席の下限は 10 分", () => {
    expect(AWAY_ROW_MIN_MINUTES).toBe(10);
  });

  // Scenario: 10 分以上の離席は行になる
  it("ちょうど 10 分の離席は「離席 10:15 – 10:25」", async () => {
    const list = await show([stay], [{ kind: "away", start: at("10:15"), end: at("10:25") }]);
    expect(texts(list)).toContain("離席 10:15 – 10:25");
  });

  // Scenario: 10 分に満たない離席は行にならない
  it("9 分 59 秒の離席は行にならない", async () => {
    const list = await show([stay], [{ kind: "away", start: atS("11:00:00"), end: atS("11:09:59") }]);
    expect(texts(list).some((t) => t.startsWith("離席"))).toBe(false);
  });

  // Scenario: いま離席中の時間は今日の行になる
  it("いままでの離席は「離席 14:05 – 14:30」で、それより後の時刻の行は無い", async () => {
    const list = await show([{ kind: "stay", start: at("13:00"), end: at("14:30"), id: "s1" }], [{ kind: "away", start: at("14:05"), end: at("14:30") }]);
    const t = texts(list);
    expect(t).toContain("離席 14:05 – 14:30");
    expect(t.join("\n")).not.toMatch(/14:[3-5]\d –|1[5-9]:\d\d –/);
  });

  // Scenario: PC の停止は行になる
  it("日の端までの PC 停止は「PC 停止 23:40 – 24:00」", async () => {
    const list = await show([stay], [{ kind: "stopped", start: at("23:40"), end: `2026-09-08T00:00:00+09:00` }]);
    expect(texts(list)).toContain("PC 停止 23:40 – 24:00");
  });

  // Scenario: 除外は件数だけの行になる
  it("除外は「除外 16:05 – 16:25（3 件。本文なし）」", async () => {
    const list = await show([stay], [{ kind: "excluded", start: at("16:05"), end: at("16:25"), count: 3 }]);
    expect(texts(list)).toContain("除外 16:05 – 16:25（3 件。本文なし）");
  });

  // Scenario: 除外の行に本文は出ない
  it("除外の行は種類と時刻と件数だけを持つ", async () => {
    const list = await show([stay], [{ kind: "excluded", start: at("16:05"), end: at("16:25"), count: 3, app: "secret.exe", title: "秘密", url: "https://secret" }]);
    expect(list.textContent).not.toMatch(/secret|秘密/);
  });

  // Scenario: 送る契機で区切られた除外は 1 行にまとまる
  it("まとまった除外は 1 行で描かれる", async () => {
    const list = await show([stay], [{ kind: "excluded", start: at("16:05"), end: at("16:25"), count: 3 }]);
    expect(texts(list).filter((t) => t.startsWith("除外"))).toEqual(["除外 16:05 – 16:25（3 件。本文なし）"]);
  });

  // Scenario: スマホの取得元に残っていなかった期間は行になる
  it("retention は「スマホ 取得元に残っていない 6:00 – 9:00」", async () => {
    const list = await show([], [{ kind: "unavailable", start: at("06:00"), end: at("09:00"), reason: "retention" }]);
    expect(texts(list)).toContain("スマホ 取得元に残っていない 6:00 – 9:00");
  });

  // Scenario: 時計のずれで取り直さなかった期間は別の文字の行になる
  it("clock_skew_abandoned は別の文字で、取得元に残っていないとは書かない", async () => {
    const list = await show([], [{ kind: "unavailable", start: at("06:00"), end: at("09:00"), reason: "clock_skew_abandoned" }]);
    expect(texts(list)).toContain("スマホ 時計のずれで取れていない 6:00 – 9:00");
    expect(list.textContent).not.toContain("取得元に残っていない");
  });

  // Scenario: まだ届いていないブラウザ履歴は行になる
  it("並びの先頭にブラウザの行があり、最後に届いた時刻とまだ届いていないことを言う", async () => {
    const list = await show([stay], [], { browser_through: "2026-09-06T09:00:00+09:00" });
    const first = texts(list)[0];
    expect(first.startsWith("ブラウザ")).toBe(true);
    expect(first).toContain("9/6 9:00");
    expect(first).toContain("まだ届いていない");
  });

  // Scenario: 届いた日にはまだ届いていない行は出ない
  it("browser_through が無ければブラウザの行は無い（届いた日）", async () => {
    const list = await show([stay]);
    expect(texts(list).some((t) => t.startsWith("ブラウザ"))).toBe(false);
  });

  // Scenario: ブラウザ履歴を集めていない日にはまだ届いていない行は出ない
  it("browser_through が無ければブラウザの行は無い（集めていない日）", async () => {
    const list = await show([{ kind: "no-record", start: at("00:00"), end: "2026-09-08T00:00:00+09:00" }]);
    expect(list.textContent).not.toContain("まだ届いていない");
  });

  // Scenario: 取れていない時間の行は始まりの時刻の位置に並ぶ
  it("離席の行は滞在の直後・次の移動の前に並ぶ", async () => {
    const list = await show([stay, move], [{ kind: "away", start: at("10:15"), end: at("10:32") }]);
    const t = texts(list);
    expect(t[0]).toContain("9:00");
    expect(t[1]).toBe("離席 10:15 – 10:32");
    expect(t[2]).toContain("移動");
  });

  it("どの行にも入らないものは始まりより前で最も近い行の直後で、始まりの順", () => {
    const entries: DayEntry[] = [
      { kind: "stay", start: at("09:00"), end: at("10:00") },
      { kind: "erased", start: at("10:00"), end: at("11:00") },
      { kind: "stay", start: at("12:00"), end: at("13:00") },
    ];
    const gaps: Gap[] = [
      { kind: "stopped", start: at("11:30"), end: at("11:50") },
      { kind: "away", start: at("11:10"), end: at("11:25") },
      { kind: "away", start: at("08:00"), end: at("08:30") },
    ];
    const kinds = placeGaps(entries, gaps).map((p) => (p.at === "gap" ? `gap:${p.gap.start.slice(11, 16)}` : p.entry.kind));
    expect(kinds).toEqual(["gap:08:00", "stay", "erased", "gap:11:10", "gap:11:30", "stay"]);
  });

  // Scenario: 「消した」の行の直後に「ほかの記録」の行がある
  it("消した行の中に始まる離席は、同じ範囲のほかの記録の行の直後に並ぶ", () => {
    const entries: DayEntry[] = [
      { kind: "erased", start: at("10:00"), end: at("11:00") },
      { kind: "other", start: at("10:00"), end: at("11:00") },
    ];
    const gaps: Gap[] = [{ kind: "away", start: at("10:15"), end: at("10:40") }];
    const kinds = placeGaps(entries, gaps).map((p) => (p.at === "gap" ? "gap" : p.entry.kind));
    expect(kinds).toEqual(["erased", "other", "gap"]);
  });

  // Scenario: 取れていない時間の行は文字で区別される
  it("離席・PC 停止・除外・スマホ・移動・記録なしがそれぞれの文字を持つ", async () => {
    const list = await show(
      [move, { kind: "no-record", start: at("13:00"), end: at("14:00") }],
      [
        { kind: "away", start: at("12:10"), end: at("12:30") },
        { kind: "stopped", start: at("13:10"), end: at("13:20") },
        { kind: "excluded", start: at("13:20"), end: at("13:30"), count: 1 },
        { kind: "unavailable", start: at("13:40"), end: at("13:50"), reason: "retention" },
      ],
    );
    const t = texts(list);
    for (const w of ["離席", "PC 停止", "除外", "スマホ", "移動", "記録なし"]) expect(t.some((x) => x.startsWith(w)), w).toBe(true);
  });
});

describe("記録なしの事情の文字", () => {
  const noRecord = (reasons?: unknown[]) => ({ kind: "no-record", start: at("08:20"), end: at("16:40"), ...(reasons ? { reasons } : {}) });

  // Scenario: 収集を始める前の日の記録なしにはそう添えられる
  it("収集を始める前", async () => {
    const list = await show([{ kind: "no-record", start: at("00:00"), end: "2026-09-08T00:00:00+09:00", reasons: [{ kind: "before_start" }] }]);
    const row = within(list).getByTestId("row-no-record");
    expect(row.textContent).toContain("記録なし 0:00 – 24:00");
    expect(within(row).getByTestId("reasons").textContent).toContain("位置の収集を始める前");
  });

  // Scenario: 端末が破棄した時間の記録なしには件数が添えられる
  it("端末が破棄と件数", async () => {
    const list = await show([noRecord([{ kind: "dropped", count: 60 }])]);
    expect(within(list).getByTestId("reasons").textContent).toContain("端末が破棄 60 件");
  });

  // Scenario: 書庫に位置がある時間の記録なしには件数が添えられる
  it("書庫に位置と件数", async () => {
    const list = await show([noRecord([{ kind: "archive", count: 14 }])]);
    expect(within(list).getByTestId("reasons").textContent).toContain("書庫に位置 14 件");
  });

  // Scenario: 本人が止めていた時間の記録なしにはそう添えられる
  it("止めていた", async () => {
    const list = await show([noRecord([{ kind: "stopped" }])]);
    expect(within(list).getByTestId("reasons").textContent).toContain("止めていた");
  });

  // Scenario: 事情が重なると並べて添えられる
  it("受け取った順（破棄、書庫の順）に並べる", async () => {
    const list = await show([noRecord([{ kind: "dropped", count: 60 }, { kind: "archive", count: 14 }])]);
    const t = within(list).getByTestId("reasons").textContent ?? "";
    expect(t.indexOf("端末が破棄")).toBeGreaterThanOrEqual(0);
    expect(t.indexOf("端末が破棄")).toBeLessThan(t.indexOf("書庫に位置"));
  });

  // Scenario: 事情の無い記録なしには何も添えない
  it("事情が無ければ記録なしと範囲だけ", async () => {
    const list = await show([noRecord()]);
    const row = within(list).getByTestId("row-no-record");
    expect(row.textContent).toBe("記録なし 8:20 – 16:40");
    expect(within(row).queryByTestId("reasons")).toBeNull();
  });

  // Scenario: 事情は記録なしの行の範囲を変えない
  it("事情があっても行は 1 行で 8:20 – 16:40 のまま", async () => {
    const list = await show([noRecord([{ kind: "dropped", count: 60 }])]);
    const rows = within(list).getAllByTestId("row-no-record");
    expect(rows).toHaveLength(1);
    expect(rows[0].textContent).toContain("記録なし 8:20 – 16:40");
  });
});
