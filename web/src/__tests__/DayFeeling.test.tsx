// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の画面（S-2）の頭の気分の欄と、滞在の行の気分（ST17 / tasks 10.1 / design D14・D20）。
 *
 * **応答を固定して描画を見る。** サーバが何を返すか（前/後・吸収・消した滞在の数え方）は Rust 側の試験が見る。
 * 実寸（40 px 以下・横溢れ）は jsdom で測れないので `web/e2e/day-feeling.spec.ts` が持つ。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import type { Feeling, FeelingsDay, Phase } from "../feelings";
import type { DayEntry } from "../stays";

const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;

const feeling = (valence: number, phase: Phase = "after", n = 0): Feeling => ({
  id: `f-${valence}-${phase}-${n}`,
  scales: [{ id: "valence", version: 1, value: valence }],
  note: null,
  written_at: `2026-10-02T21:0${n}:00+09:00`,
  ingested_at: `2026-10-02T21:0${n}:00Z`,
  phase,
  target_stay: null,
});

const latestOf = (records: Feeling[]): { before: Feeling | null; after: Feeling | null } => ({
  before: records.find((r) => r.phase === "before") ?? null,
  after: records.find((r) => r.phase === "after") ?? null,
});

interface Spec {
  status?: FeelingsDay["status"];
  day?: Feeling[];
  /** 滞在の識別子 → その滞在の記録（新しい順） */
  stays?: Record<string, Feeling[]>;
  /** 消した滞在も数える `stays_with_records` を上書きする */
  withRecords?: number;
}

function feelingsDay(date: string, spec: Spec): FeelingsDay {
  const day = spec.day ?? [];
  const stays = Object.entries(spec.stays ?? {}).map(([stay, records]) => ({ stay, latest: latestOf(records), records }));
  return {
    date,
    status: spec.status ?? (day.length + stays.length > 0 ? "recorded" : "missing"),
    day: { latest: latestOf(day), records: day },
    stays,
    stays_with_records: spec.withRecords ?? stays.length,
    hidden_stay_records: 0,
  };
}

const at = (date: string, hm: string): string => new Date(`${date}T${hm}:00+09:00`).toISOString();
const stay = (date: string, id: string, s: string, e: string): DayEntry => ({ kind: "stay", start: at(date, s), end: at(date, e), id, criteria_id: 1 });

let feelingsByDate: Record<string, Spec> = {};
let entriesByDate: Record<string, DayEntry[]> = {};
let feelingsFail = false;

beforeEach(() => {
  feelingsByDate = {};
  entriesByDate = {};
  feelingsFail = false;
  window.location.hash = "";
  vi.stubGlobal("fetch", (path: string) => {
    if (path.startsWith("/api/feelings")) {
      if (feelingsFail) return Promise.resolve({ ok: false, status: 500 } as Response);
      const date = /from=(\d{4}-\d{2}-\d{2})/.exec(path)?.[1] ?? "";
      return Promise.resolve(ok({ today: "2026-10-03", first_day: "2026-09-06", days: [feelingsDay(date, feelingsByDate[date] ?? {})] }));
    }
    const date = /date=(\d{4}-\d{2}-\d{2})/.exec(path)?.[1] ?? "";
    return Promise.resolve(ok({ date, criteria: [{ criteria_id: 1, radius_m: 100, min_minutes: 10 }], entries: entriesByDate[date] ?? [] }));
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

async function openDay(date: string): Promise<HTMLElement> {
  window.location.hash = `#/day/${date}`;
  render(<Root />);
  await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
  return screen.getByTestId("day-feeling");
}
const headText = async (text: string): Promise<void> => {
  await waitFor(() => expect(screen.getByTestId("day-feeling-text").textContent).toBe(text));
};

describe("頭の欄", () => {
  // Scenario: 書いた日の 1 日の画面の頭に気分が出る
  it("「日」の最新が快–不快 +1 の日は「気分 +1 快」が出る", async () => {
    feelingsByDate["2026-10-02"] = { day: [feeling(1)] };
    await openDay("2026-10-02");
    await headText("気分 +1 快");
    expect(screen.getByTestId("day-feeling").getAttribute("data-state")).toBe("recorded");
  });

  it("後の最新があれば前より先に出す", async () => {
    feelingsByDate["2026-10-02"] = { day: [feeling(2, "before"), feeling(-1, "after", 1)] };
    await openDay("2026-10-02");
    await headText("気分 -1 不快");
  });

  // Scenario: 書いていない日の 1 日の画面の頭に未記入が出る
  it("「未記入」の日は破線の枠と「気分 未記入」と「書く」が出る", async () => {
    await openDay("2026-10-01");
    await headText("気分 未記入");
    const box = screen.getByTestId("day-feeling");
    expect(box.style.outline).toContain("dashed");
    expect(within(box).getByRole("link", { name: "書く" })).toBeTruthy();
  });

  // Scenario: 1 日の画面の書くから主観入力の画面が開く
  it("「書く」はその日を対象にした主観入力の画面を開く", async () => {
    await openDay("2026-10-02");
    await headText("気分 未記入");
    const link = within(screen.getByTestId("day-feeling")).getByRole("link", { name: "書く" });
    expect(link.getAttribute("href")).toBe("#/feel/2026-10-02");
    fireEvent.click(link);
    await waitFor(() => expect(window.location.hash).toBe("#/feel/2026-10-02"));
  });

  // Scenario: 滞在にだけ書いた日は未記入と出ない
  it("滞在にだけ書いた日は「気分 滞在ごとに 2 件」が出て、「未記入」は出ない", async () => {
    feelingsByDate["2026-10-02"] = { stays: { a: [feeling(1)], b: [feeling(0)] } };
    await openDay("2026-10-02");
    await headText("気分 滞在ごとに 2 件");
    expect(screen.getByTestId("day-feeling").textContent).not.toContain("未記入");
  });

  it("本人が消した滞在も数える（サーバの数をそのまま出す）", async () => {
    feelingsByDate["2026-10-02"] = { stays: { a: [feeling(1)] }, withRecords: 2 };
    await openDay("2026-10-02");
    await headText("気分 滞在ごとに 2 件");
  });

  // Scenario: 同じ滞在の書き直しは 1 と数える
  it("1 つの滞在に 2 件の記録がある日は「気分 滞在ごとに 1 件」", async () => {
    feelingsByDate["2026-10-02"] = { stays: { a: [feeling(2, "after", 1), feeling(1, "after", 0)] } };
    await openDay("2026-10-02");
    await headText("気分 滞在ごとに 1 件");
  });

  // Scenario: 使い始める前の日の 1 日の画面に未記入と出ない
  it("最初の記録より前の日は「気分 使い始める前」が出て、「未記入」は出ない", async () => {
    feelingsByDate["2026-09-04"] = { status: "before_start" };
    await openDay("2026-09-04");
    await headText("気分 使い始める前");
    expect(screen.getByTestId("day-feeling").textContent).not.toContain("未記入");
    expect(screen.getByTestId("day-feeling").style.outline).not.toContain("dashed");
  });

  // Scenario: これからの日の 1 日の画面に未記入と出ない
  it("明日の日は「未記入」が出ず、「書く」が出る", async () => {
    feelingsByDate["2026-10-04"] = { status: "future" };
    await openDay("2026-10-04");
    await headText("気分 これから");
    const box = screen.getByTestId("day-feeling");
    expect(box.textContent).not.toContain("未記入");
    expect(within(box).getByRole("link", { name: "書く" }).getAttribute("href")).toBe("#/feel/2026-10-04");
  });

  // Scenario: 気分の読み出しの失敗は未記入と出ない
  it("主観の読み出しが失敗したら失敗が出て、「未記入」は出ず、その日の並びは出る", async () => {
    feelingsFail = true;
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "a", "09:00", "10:00")];
    await openDay("2026-10-02");
    await headText("気分を読み出せませんでした");
    expect(screen.getByTestId("day-feeling").textContent).not.toContain("未記入");
    await waitFor(() => expect(screen.getByTestId("row-stay")).toBeTruthy());
    expect(screen.queryByTestId("row-feeling")).toBeNull();
  });

  // Scenario: 日を移ると気分の欄もその日のものになる
  it("「記入あり」の日から前の日（未記入）へ移ると、欄が「気分 未記入」に変わる", async () => {
    feelingsByDate["2026-10-02"] = { day: [feeling(1)] };
    await openDay("2026-10-02");
    await headText("気分 +1 快");
    fireEvent.click(screen.getByRole("button", { name: "前の日" }));
    await waitFor(() => expect(screen.getByTestId("day-title").textContent).toContain("1 日"));
    await headText("気分 未記入");
  });

  // Scenario: 気分の欄は 1 行で 40 px 以下
  it("1 行（折り返さない）で高さの上限を 40 px にしてある（実寸は e2e が測る）", async () => {
    const box = await openDay("2026-10-02");
    await headText("気分 未記入");
    expect(box.style.maxHeight).toBe("40px");
    expect(box.style.flexWrap).toBe("nowrap");
    expect(screen.getByTestId("day-feeling-text").style.whiteSpace).toBe("nowrap");
  });

  it("欄があっても並びの行は変わらない", async () => {
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "a", "09:00", "10:00")];
    feelingsByDate["2026-10-02"] = { day: [feeling(1)] };
    await openDay("2026-10-02");
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(1));
  });
});

describe("滞在の行", () => {
  // Scenario: 気分を書いた滞在の行に気分が出る
  it("快–不快 -1 の記録を書いた滞在の行に「気分 -1 不快」が添えてある", async () => {
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "a", "09:00", "10:00"), stay("2026-10-02", "b", "11:00", "12:00")];
    feelingsByDate["2026-10-02"] = { stays: { a: [feeling(-1)] } };
    await openDay("2026-10-02");
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    await waitFor(() => expect(within(screen.getAllByTestId("row-stay")[0]!).getByTestId("row-feeling").textContent).toBe("気分 -1 不快"));
  });

  // Scenario: 滞在の行は後の気分を前より先に出す
  it("前（+2）と後（-1）がある滞在の行は「気分 -1 不快」", async () => {
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "a", "09:00", "10:00")];
    feelingsByDate["2026-10-02"] = { stays: { a: [feeling(-1, "after", 1), feeling(2, "before", 0)] } };
    await openDay("2026-10-02");
    await waitFor(() => expect(screen.getByTestId("row-feeling").textContent).toBe("気分 -1 不快"));
  });

  // Scenario: 1 日の画面で吸収された滞在の気分は吸収先の行に出る
  it("サーバが吸収先の滞在に寄せた記録は、吸収先の行に出る", async () => {
    // 滞在 A の記録は、吸収先 B の識別子の下に来る（サーバが解決済み）。並びに出るのは B だけ
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "B", "10:00", "11:40")];
    feelingsByDate["2026-10-02"] = { stays: { B: [feeling(0)] } };
    await openDay("2026-10-02");
    await waitFor(() => expect(within(screen.getByTestId("row-stay")).getByTestId("row-feeling").textContent).toBe("気分 0 どちらでもない"));
  });

  // Scenario: 気分の無い滞在の行には何も添えない
  it("記録の無い滞在の行には気分も「未記入」も添えない", async () => {
    entriesByDate["2026-10-02"] = [stay("2026-10-02", "a", "09:00", "10:00")];
    feelingsByDate["2026-10-02"] = { day: [feeling(1)] };
    await openDay("2026-10-02");
    await headText("気分 +1 快");
    const row = await screen.findByTestId("row-stay");
    expect(within(row).queryByTestId("row-feeling")).toBeNull();
    expect(row.textContent).not.toContain("気分");
    expect(row.textContent).not.toContain("未記入");
  });
});
