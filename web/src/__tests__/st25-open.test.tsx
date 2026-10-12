// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 開いた行の中身と畳み（ST25 / tasks 9.1・9.2 / design D8）。固定の応答の文字と開閉を確かめる。
 */
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DayView } from "../DayView";

const DATE = "2026-09-07";
const at = (hm: string): string => `${DATE}T${hm}:00+09:00`;
const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;

afterEach(() => {
  vi.unstubAllGlobals();
});

const stay = { kind: "stay", start: at("09:00"), end: at("12:00"), id: "s1", criteria_id: 1 };
const move = { kind: "move", start: at("12:00"), end: at("12:42") };

let calls: string[] = [];
let recordsFor: (path: string) => unknown[] = () => [];
const counts: unknown[] = [{ logical_source: "c01-location", display_name: "場所", count: 120 }];

async function show(entries: unknown[]): Promise<HTMLElement> {
  calls = [];
  vi.stubGlobal("fetch", (path: string) => {
    calls.push(path);
    if (path.startsWith("/api/day/records")) return Promise.resolve(ok(recordsFor(path)));
    if (path.startsWith("/api/day?")) return Promise.resolve(ok({ date: DATE, criteria: [], entries }));
    if (path.startsWith("/api/stays/detail")) return Promise.resolve(ok({ stay_id: "s1", start: at("09:00"), end: at("12:00"), counts }));
    return new Promise<Response>(() => {});
  });
  render(<DayView date={DATE} />);
  return screen.findByRole("list", { name: "1 日の並び" });
}

const r = (id: string, source: string, hm: string, extra: Record<string, unknown> = {}): Record<string, unknown> => ({
  id,
  kind: source === "browser" ? "visit" : "foreground",
  source,
  start: at(hm),
  tz_id: "Asia/Tokyo",
  ...extra,
});
const toggle = (row: HTMLElement): HTMLElement => within(row).getAllByRole("button")[0];
const detailTexts = (): string[] => within(screen.getByTestId("records")).getAllByRole("listitem").map((li) => (li.textContent ?? "").trim());

describe("開いた行の中身", () => {
  // Scenario: 行を開くとその時間の記録が時刻順に出る
  it("滞在を開くと /api/day/records を 1 回読み、PC とスマホの記録が時刻順に並ぶ", async () => {
    recordsFor = () => [
      r("a", "pc", "09:05", { app: "Code", title: "main.rs" }),
      r("b", "phone", "09:10", { app: "LINE", title: "トーク" }),
      r("c", "pc", "09:20", { app: "Chrome", title: "docs" }),
    ];
    const list = await show([stay, move]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    const t = detailTexts();
    expect(t[0]).toContain("PC");
    expect(t[1]).toContain("スマホ");
    expect(t[2]).toContain("Chrome");
    expect(t[0]).toMatch(/^9:05/);
    expect(calls.filter((c) => c.startsWith("/api/day/records"))).toEqual([
      `/api/day/records?date=${DATE}&from=${encodeURIComponent(at("09:00"))}&to=${encodeURIComponent(at("12:00"))}`,
    ]);
    expect(window.location.hash).not.toContain("records");
  });

  // 日をまたぐ行は、その日の端で切らずに行の実際の始まりと終わりで読む（design D7）
  it("前の日 23:00 から続く滞在を開くと、from は前の日の 23:00 のまま", async () => {
    recordsFor = () => [];
    const across = { kind: "stay", start: "2026-09-06T23:00:00+09:00", end: at("02:00"), id: "s1", criteria_id: 1 };
    const list = await show([across]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    expect(calls.filter((c) => c.startsWith("/api/day/records"))).toEqual([
      `/api/day/records?date=${DATE}&from=${encodeURIComponent("2026-09-06T23:00:00+09:00")}&to=${encodeURIComponent(at("02:00"))}`,
    ]);
  });

  // Scenario: 位置の点は件数で出る
  // Scenario: 滞在の詳細の消す操作は残る
  it("位置は 1 件ずつの行にならず「場所 120 件」。「この滞在を消す」は末尾に残る", async () => {
    recordsFor = () => [r("a", "pc", "09:05", { app: "Code" })];
    const list = await show([stay]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByText("場所 120 件");
    expect(detailTexts()).toHaveLength(1);
    const detail = screen.getByTestId("stay-detail");
    expect(within(detail).getByRole("button", { name: "この滞在を消す" })).toBeTruthy();
    const kids = Array.from(detail.children);
    expect(kids[kids.length - 1].textContent).toContain("この滞在を消す");
    expect(detail.textContent!.indexOf("Code")).toBeLessThan(detail.textContent!.indexOf("場所 120 件"));
  });

  // Scenario: 地域の違う記録にはその地域の時刻が添えられる
  it("Europe/London の記録は Tokyo の時刻に（London 8:02）が添う。Tokyo の記録には添わない", async () => {
    recordsFor = () => [r("a", "pc", "16:02", { app: "Code", tz_id: "Europe/London" }), r("b", "pc", "16:30", { app: "Chrome" })];
    const list = await show([{ ...stay, start: at("09:00"), end: at("18:00") }]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    const t = detailTexts();
    expect(t[0]).toMatch(/^16:02/);
    expect(t[0]).toContain("（London 8:02）");
    expect(t[1]).not.toContain("（");
  });

  // Scenario: 開ける行は一度に 1 つ
  it("滞在を開いた後に移動を開くと、移動の中身だけが開いている", async () => {
    recordsFor = (p) => (p.includes(encodeURIComponent(at("12:00"))) && p.includes("from=" + encodeURIComponent(at("12:00"))) ? [r("m", "pc", "12:10", { app: "Maps" })] : [r("a", "pc", "09:05", { app: "Code" })]);
    const list = await show([stay, move]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("stay-detail");
    fireEvent.click(toggle(within(list).getByTestId("row-move")));
    await screen.findByText(/Maps/);
    expect(screen.queryByTestId("stay-detail")).toBeNull();
    expect(screen.getAllByTestId("records")).toHaveLength(1);
    expect(within(list).getByTestId("row-move").querySelector("[aria-expanded=true]")).not.toBeNull();
  });
});

describe("畳んだ行", () => {
  const code = Array.from({ length: 23 }, (_, i) => r(`c${i}`, "pc", `09:${String(2 + Math.floor(i / 2)).padStart(2, "0")}`, { app: "VS Code", title: `file${i}` }));

  // Scenario: 同じアプリの連続は 1 行に畳まれる
  it("開いた中で VS Code の 23 件は「9:02 – 9:xx」「PC」「VS Code」「23 件」の 1 行、次が Chrome", async () => {
    recordsFor = () => [...code, r("x", "pc", "09:14", { app: "Chrome", title: "docs" })];
    const list = await show([stay]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    const t = detailTexts();
    expect(t).toHaveLength(2);
    expect(t[0]).toMatch(/^9:02 – 9:13/);
    expect(t[0]).toContain("PC");
    expect(t[0]).toContain("VS Code");
    expect(t[0]).toContain("23 件");
    expect(t[1]).toContain("Chrome");
  });

  // Scenario: 畳んだ行を開くと中の全件が時刻順に出る
  it("畳んだ行を開くと 23 件が時刻と題名つきで時刻順に出る", async () => {
    recordsFor = () => code;
    const list = await show([stay]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    fireEvent.click(screen.getByRole("button", { name: /23 件/ }));
    const inner = screen.getAllByTestId("fold-records");
    expect(inner).toHaveLength(1);
    const items = within(inner[0]).getAllByRole("listitem").map((li) => li.textContent ?? "");
    expect(items).toHaveLength(23);
    expect(items[0]).toMatch(/^9:02/);
    expect(items[0]).toContain("file0");
    expect(items[22]).toContain("file22");
  });

  // Scenario: 別のアプリが挟まると別の行になる
  // Scenario: 畳んだ行も一度に 1 つ
  it("VS Code・Chrome・VS Code は 3 行。畳んだ行を開いた後に別の畳んだ行を開くと後のだけが開く", async () => {
    recordsFor = () => [
      r("a", "pc", "09:00", { app: "VS Code", title: "a1" }),
      r("b", "pc", "09:01", { app: "VS Code", title: "a2" }),
      r("c", "pc", "09:02", { app: "Chrome", title: "c1" }),
      r("d", "pc", "09:03", { app: "VS Code", title: "d1" }),
      r("e", "pc", "09:04", { app: "VS Code", title: "d2" }),
    ];
    const list = await show([stay]);
    fireEvent.click(toggle(within(list).getByTestId("row-stay")));
    await screen.findByTestId("records");
    expect(detailTexts().map((t) => t.includes("VS Code"))).toEqual([true, false, true]);
    const folds = screen.getAllByRole("button", { name: /2 件/ });
    expect(folds).toHaveLength(2);
    fireEvent.click(folds[0]);
    expect(screen.getAllByTestId("fold-records")).toHaveLength(1);
    expect(screen.getByTestId("fold-records").textContent).toContain("a1");
    fireEvent.click(screen.getAllByRole("button", { name: /2 件/ })[1]);
    expect(screen.getAllByTestId("fold-records")).toHaveLength(1);
    expect(screen.getByTestId("fold-records").textContent).toContain("d1");
    expect(screen.getByTestId("fold-records").textContent).not.toContain("a1");
  });
});
