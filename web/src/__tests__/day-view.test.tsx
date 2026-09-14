// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の滞在の一覧（ST16 / tasks 6.1 / design D8）。
 *
 * **応答を固定して描画を見る。** サーバが何件返すかは `stays_day_api_walked_day`（Rust）が見る（R21）。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import { clock, dayFromHash, duration, type DayView } from "../stays";

const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;

/** 自宅・職場・昼の店・職場・自宅にとどまった 1 日（サーバが返す形）。 */
function walkedDay(): DayView {
  const j = (hm: string, day = "2026-07-01"): string => new Date(`${day}T${hm}:00+09:00`).toISOString();
  const stay = (s: string, e: string, id: string, sd?: string, ed?: string) => ({
    kind: "stay" as const, start: j(s, sd), end: j(e, ed), id, criteria_id: 1,
  });
  const move = (s: string, e: string) => ({ kind: "move" as const, start: j(s), end: j(e) });
  return {
    date: "2026-07-01",
    criteria: [{ criteria_id: 1, radius_m: 100, min_minutes: 10 }],
    entries: [
      stay("23:30", "08:00", "a", "2026-06-30"),
      move("08:00", "08:30"),
      stay("08:30", "12:00", "b"),
      move("12:00", "12:10"),
      stay("12:10", "12:50", "c"),
      move("12:50", "13:00"),
      stay("13:00", "18:00", "d"),
      move("18:00", "18:30"),
      stay("18:30", "00:30", "e", undefined, "2026-07-02"),
    ],
  };
}

let calls: string[] = [];

function serve(byDate: Record<string, DayView | number>): void {
  calls = [];
  vi.stubGlobal("fetch", (path: string) => {
    calls.push(path);
    const date = /date=(\d{4}-\d{2}-\d{2})/.exec(path)?.[1] ?? "";
    const body = byDate[date];
    if (typeof body === "number") return Promise.resolve({ ok: false, status: body } as Response);
    if (body === undefined) return Promise.resolve(ok({ date, criteria: [], entries: [] }));
    return Promise.resolve(ok(body));
  });
}

beforeEach(() => {
  window.location.hash = "";
});
afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

describe("書き方", () => {
  it("時刻は Asia/Tokyo の H:MM。見ている日の外には日付を添え、翌日 0:00 は 24:00", () => {
    expect(clock("2026-07-01T00:30:00Z", "2026-07-01")).toBe("9:30");
    expect(clock("2026-06-30T14:30:00Z", "2026-07-01")).toBe("6/30 23:30");
    expect(clock("2026-07-01T15:00:00Z", "2026-07-01")).toBe("24:00");
    expect(clock("2026-07-01T15:30:00Z", "2026-07-01")).toBe("7/2 0:30");
  });
  it("長さは 3 時間 34 分 / 42 分", () => {
    expect(duration("2026-07-01T00:00:00Z", "2026-07-01T03:34:00Z")).toBe("3 時間 34 分");
    expect(duration("2026-07-01T00:00:00Z", "2026-07-01T00:42:00Z")).toBe("42 分");
  });
  it("アドレスから日付を読む", () => {
    expect(dayFromHash("#/day/2026-09-12")).toBe("2026-09-12");
    expect(dayFromHash("#/day/")).toBeNull();
    expect(dayFromHash("#/day")).toBeNull();
    expect(dayFromHash("")).toBeUndefined();
    expect(dayFromHash("#/day/2026-9-12")).toBeUndefined();
  });
});

describe("1 日の一覧", () => {
  // Scenario: 1 日歩き回った後、その日の滞在が一覧で出る
  it("滞在の行が 5 件、時刻の範囲を見出しにして長さと始まり – 終わりを添え、間に移動の行が 4 件", async () => {
    serve({ "2026-07-01": walkedDay() });
    window.location.hash = "#/day/2026-07-01";
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(5));
    const rows = screen.getAllByTestId("row-stay");
    const second = rows[1];
    expect(within(second).getByRole("heading").textContent).toBe("8:30 – 12:00");
    expect(second.textContent).toContain("3 時間 30 分 ・ 8:30 – 12:00");
    // 日をまたぐ滞在は実際の時刻で
    expect(within(rows[0]).getByRole("heading").textContent).toBe("6/30 23:30 – 8:00");
    expect(within(rows[4]).getByRole("heading").textContent).toBe("18:30 – 7/2 0:30");
    expect(screen.getAllByTestId("row-move")).toHaveLength(4);
    expect(screen.getAllByTestId("row-move")[0].textContent).toContain("移動 30 分");
    // 並びの順（滞在と移動が交互）
    const kinds = [...document.querySelectorAll("li[data-kind]")].map((li) => li.getAttribute("data-kind"));
    expect(kinds).toEqual(["stay", "move", "stay", "move", "stay", "move", "stay", "move", "stay"]);
    expect(screen.getByTestId("criteria").textContent).toContain("この一覧は 半径 100 m / 10 分 で作った");
    expect(screen.queryByTestId("row-criteria")).toBeNull();
  });

  it("記録なしは「移動」と文字で違う", async () => {
    const day: DayView = {
      date: "2026-07-06",
      criteria: [],
      entries: [{ kind: "no-record", start: "2026-07-05T23:20:00Z", end: "2026-07-06T07:40:00Z" }],
    };
    serve({ "2026-07-06": day });
    window.location.hash = "#/day/2026-07-06";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("row-no-record")).toBeTruthy());
    const row = screen.getByTestId("row-no-record");
    expect(row.textContent).toBe("記録なし 8:20 – 16:40");
    expect(row.textContent).not.toContain("移動");
    // 滞在が無いことは文字で出す（読み込み中や失敗と同じ顔にしない）
    expect(screen.getByTestId("day-empty")).toBeTruthy();
  });

  // Scenario: 基準の違う滞在が混ざると行に基準が添えられる
  it("一覧の上に 2 つの基準、最初の基準と違う滞在の行にその基準", async () => {
    const day: DayView = {
      date: "2026-07-11",
      criteria: [
        { criteria_id: 2, radius_m: 50, min_minutes: 10 },
        { criteria_id: 1, radius_m: 100, min_minutes: 10 },
      ],
      entries: [
        { kind: "stay", start: "2026-07-11T00:00:00Z", end: "2026-07-11T01:00:00Z", id: "new", criteria_id: 2 },
        { kind: "move", start: "2026-07-11T01:00:00Z", end: "2026-07-11T02:00:00Z" },
        { kind: "stay", start: "2026-07-11T02:00:00Z", end: "2026-07-11T03:00:00Z", id: "old", criteria_id: 1 },
      ],
    };
    serve({ "2026-07-11": day });
    window.location.hash = "#/day/2026-07-11";
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const top = screen.getByTestId("criteria").textContent ?? "";
    expect(top).toContain("半径 50 m / 10 分");
    expect(top).toContain("半径 100 m / 10 分");
    const [first, second] = screen.getAllByTestId("row-stay");
    expect(within(first).queryByTestId("row-criteria")).toBeNull();
    expect(within(second).getByTestId("row-criteria").textContent).toContain("半径 100 m / 10 分");
  });

  // Scenario: 読み出しに失敗すると失敗と出る
  it("失敗と出て、記録なしの行も空の一覧も出ない", async () => {
    serve({ "2026-07-12": 500 });
    window.location.hash = "#/day/2026-07-12";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-error")).toBeTruthy());
    expect(screen.getByRole("alert").textContent).toContain("読み出しに失敗しました");
    expect(screen.queryByTestId("row-no-record")).toBeNull();
    expect(screen.queryByTestId("day-empty")).toBeNull();
    expect(screen.queryByRole("list")).toBeNull();
  });

  it("読み込み中は滞在が無いと見えない", () => {
    vi.stubGlobal("fetch", () => new Promise<Response>(() => {}));
    window.location.hash = "#/day/2026-07-13";
    render(<Root />);
    expect(screen.getByTestId("day-loading")).toBeTruthy();
    expect(screen.queryByTestId("day-empty")).toBeNull();
  });
});

describe("日を移る", () => {
  // Scenario: 日付を含むアドレスでその日の一覧が開く
  it("#/day/2026-09-12 で 9 月 12 日の一覧", async () => {
    serve({});
    window.location.hash = "#/day/2026-09-12";
    render(<Root />);
    expect(screen.getByTestId("day-title").textContent).toContain("9 月 12 日");
    await waitFor(() => expect(calls).toContain("/api/stays?date=2026-09-12"));
  });

  // Scenario: 前の日へ移ると前の日の滞在が出る
  it("9 月 13 日から前の日へ移ると 9 月 12 日の一覧", async () => {
    const twelfth: DayView = {
      date: "2026-09-12",
      criteria: [{ criteria_id: 1, radius_m: 100, min_minutes: 10 }],
      entries: [{ kind: "stay", start: "2026-09-12T00:00:00Z", end: "2026-09-12T01:00:00Z", id: "x", criteria_id: 1 }],
    };
    serve({ "2026-09-12": twelfth });
    window.location.hash = "#/day/2026-09-13";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-empty")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "前の日" }));
    await waitFor(() => expect(screen.getByTestId("day-title").textContent).toContain("9 月 12 日"));
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(1));
    expect(calls.at(-1)).toBe("/api/stays?date=2026-09-12");
  });

  it("日付を省くと今日（Asia/Tokyo）", async () => {
    serve({});
    window.location.hash = "#/day/";
    render(<Root />);
    const today = new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Tokyo", year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date());
    await waitFor(() => expect(calls).toContain(`/api/stays?date=${today}`));
  });

  // Scenario: 稼働状況の画面の入口は変わらない
  it("ルートは稼働状況で、そこから 1 日の一覧へ移れる", async () => {
    vi.stubGlobal("fetch", () => new Promise<Response>(() => {}));
    render(<Root />);
    expect(screen.getByTestId("coverage-loading")).toBeTruthy();
    expect(screen.queryByTestId("day-view")).toBeNull();
    const link = screen.getByTestId("to-day") as HTMLAnchorElement;
    expect(link.getAttribute("href")).toBe("#/day/");
    window.location.hash = "#/day/";
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
    // 一覧から稼働状況へ戻る行き先もある
    expect(screen.getByRole("link", { name: "稼働状況へ" }).getAttribute("href")).toBe("#/");
  });
});
