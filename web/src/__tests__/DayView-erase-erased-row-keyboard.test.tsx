import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import type { DayView } from "../stays";

const response = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;
const failedResponse = (status = 500): Response => ({ ok: false, status, json: () => Promise.resolve({}) }) as Response;
const stay = (id: string, start = "2026-09-29T00:00:00Z", end = "2026-09-29T01:00:00Z") => ({
  kind: "stay" as const,
  start,
  end,
  id,
});

let calls: string[];
let day: DayView;
let failActions = false;

beforeEach(() => {
  calls = [];
  failActions = false;
  day = { date: "2026-09-29", criteria: [], entries: [stay("stay-1"), stay("stay-2", "2026-09-29T02:00:00Z", "2026-09-29T03:00:00Z")] };
  vi.stubGlobal("fetch", (input: string, init?: RequestInit) => {
    calls.push(`${init?.method ?? "GET"} ${input}`);
    if (input.includes("/detail")) {
      return Promise.resolve(response({ stay_id: "stay-1", start: day.entries[0].start, end: day.entries[0].end, counts: [
        { logical_source: "c01-location", display_name: "位置", count: 12 },
        { logical_source: "c02-window", display_name: "PC のウィンドウ", count: 3 },
      ] }));
    }
    if (input.includes("/erase")) {
      if (failActions) return Promise.resolve(failedResponse());
      day = { ...day, entries: [{ kind: "erased", start: "2026-09-29T00:00:00Z", end: "2026-09-29T01:00:00Z", stay_ids: ["stay-1"] }] };
      return Promise.resolve(response({}));
    }
    if (input.includes("/restore")) return Promise.resolve(failActions ? failedResponse() : response({}));
    return Promise.resolve(response(day));
  });
  window.location.hash = "#/day/2026-09-29";
});

afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

describe("滞在の詳細", () => {
  // Scenario: 行を選ぶとその場で詳細が開く
  it("行を選ぶと同じ行の中に詳細が開き、アドレスは変わらない", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await waitFor(() => expect(within(row).getByTestId("stay-detail")).toBeTruthy());
    expect(window.location.hash).toBe("#/day/2026-09-29");
    expect(calls.filter((call) => call.includes("/detail"))).toHaveLength(1);
  });

  // Scenario: 別の行を開くと前の行は閉じる
  it("別の行を開くと前の行は閉じる", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const rows = screen.getAllByTestId("row-stay");
    fireEvent.click(within(rows[0]).getByRole("button"));
    fireEvent.click(within(rows[1]).getByRole("button"));
    await waitFor(() => expect(screen.getAllByTestId("stay-detail")).toHaveLength(1));
    expect(within(rows[0]).queryByTestId("stay-detail")).toBeNull();
    expect(within(rows[1]).getByTestId("stay-detail")).toBeTruthy();
  });

  it("先に開いた行の遅延した詳細を後から開いた行へ表示しない", async () => {
    let resolveFirst: (value: Response) => void = () => undefined;
    const firstDetail = new Promise<Response>((resolve) => {
      resolveFirst = resolve;
    });
    vi.stubGlobal("fetch", (input: string, init?: RequestInit) => {
      calls.push(`${init?.method ?? "GET"} ${input}`);
      if (input.includes("/detail?stay_id=stay-1")) return firstDetail;
      if (input.includes("/detail?stay_id=stay-2")) {
        return Promise.resolve(response({ stay_id: "stay-2", start: day.entries[1].start, end: day.entries[1].end, counts: [
          { logical_source: "c01-location", display_name: "位置", count: 2 },
        ] }));
      }
      return Promise.resolve(response(day));
    });
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const rows = screen.getAllByTestId("row-stay");
    fireEvent.click(within(rows[0]).getByRole("button"));
    fireEvent.click(within(rows[1]).getByRole("button"));
    await waitFor(() => expect(within(rows[1]).getByText("位置 2 件")).toBeTruthy());
    resolveFirst(response({ stay_id: "stay-1", start: day.entries[0].start, end: day.entries[0].end, counts: [
      { logical_source: "c01-location", display_name: "位置", count: 99 },
    ] }));
    await Promise.resolve();
    expect(within(rows[1]).getByText("位置 2 件")).toBeTruthy();
    expect(within(rows[1]).queryByText("位置 99 件")).toBeNull();
  });

  // Scenario: 詳細にその時間の記録の件数がソースごとに出る
  it("詳細にソース名と件数が出る", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    fireEvent.click(within(screen.getAllByTestId("row-stay")[0]).getByRole("button"));
    const detail = await screen.findByTestId("stay-detail");
    expect(detail.textContent).toContain("位置 12 件");
    expect(detail.textContent).toContain("PC のウィンドウ 3 件");
  });
});

describe("滞在の削除と復元", () => {
  // Scenario: 閉じている行に消す操作は出ない
  it("閉じている行には削除操作を出さない", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    expect(screen.queryByRole("button", { name: "この滞在を消す" })).toBeNull();
  });

  // Scenario: 詳細の末尾の消す操作は 44 px を下回らない
  it("削除操作は幅と高さが 44 px 以上", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await screen.findByTestId("stay-detail");
    const button = within(row).getByRole("button", { name: "この滞在を消す" });
    expect(button.getAttribute("style")).toContain("min-height: 44px");
    expect(button.getAttribute("style")).toContain("min-width: 44px");
  });

  // Scenario: 消す前に確認が出て、やめると消えない
  it("やめると削除せず一覧に残す", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await screen.findByTestId("stay-detail");
    fireEvent.click(within(row).getByRole("button", { name: "この滞在を消す" }));
    fireEvent.click(within(row).getByRole("button", { name: "やめる" }));
    expect(screen.getAllByTestId("row-stay")).toHaveLength(2);
    expect(calls.some((call) => call.includes("POST /api/stays/erase"))).toBe(false);
  });

  // Scenario: 確認の文面に一緒に消える位置の件数が出る
  it("確認の文面に位置の件数を出す", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await screen.findByTestId("stay-detail");
    fireEvent.click(within(row).getByRole("button", { name: "この滞在を消す" }));
    expect(within(row).getByTestId("erase-confirm").textContent).toContain("位置の記録 12 件");
  });

  it("詳細の読み込み中は削除操作を出さない", async () => {
    let resolveDetail: (value: Response) => void = () => undefined;
    vi.stubGlobal("fetch", (input: string, init?: RequestInit) => {
      calls.push(`${init?.method ?? "GET"} ${input}`);
      if (input.includes("/detail")) return new Promise<Response>((resolve) => {
        resolveDetail = resolve;
      });
      return Promise.resolve(response(day));
    });
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    expect(within(row).queryByRole("button", { name: "この滞在を消す" })).toBeNull();
    resolveDetail(response({ stay_id: "stay-1", start: day.entries[0].start, end: day.entries[0].end, counts: [] }));
    await waitFor(() => expect(within(row).getByRole("button", { name: "この滞在を消す" })).toBeTruthy());
  });

  it("削除の失敗を行内に表示し、再試行できる", async () => {
    failActions = true;
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await screen.findByTestId("stay-detail");
    fireEvent.click(within(row).getByRole("button", { name: "この滞在を消す" }));
    fireEvent.click(within(row).getByRole("button", { name: "消す" }));
    await waitFor(() => expect(within(row).getByRole("alert").textContent).toContain("操作に失敗しました"));
    expect(within(row).getByRole("button", { name: "消す" })).toBeTruthy();
  });

  // Scenario: 確認して消すとその滞在の行が一覧から消える
  it("消すで一覧を読み直し、消した行に置き換える", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const row = screen.getAllByTestId("row-stay")[0];
    fireEvent.click(within(row).getByRole("button"));
    await screen.findByTestId("stay-detail");
    fireEvent.click(within(row).getByRole("button", { name: "この滞在を消す" }));
    fireEvent.click(within(row).getByRole("button", { name: "消す" }));
    await waitFor(() => expect(screen.getByTestId("row-erased")).toBeTruthy());
    expect(screen.queryByTestId("row-stay")).toBeNull();
  });

  // Scenario: 消した行から戻すと滞在の行が戻る
  it("消した行の戻すで復元口を呼ぶ", async () => {
    day = { ...day, entries: [{ kind: "erased", start: "2026-09-29T00:00:00Z", end: "2026-09-29T01:00:00Z", stay_ids: ["stay-1"] }] };
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("row-erased")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "戻す" }));
    await waitFor(() => expect(calls.some((call) => call.includes("POST /api/stays/restore"))).toBe(true));
  });

  // Scenario: 消した行は文字で区別される
  it("消した行に消したという文字を出す", async () => {
    day = { ...day, entries: [{ kind: "erased", start: "2026-09-29T00:00:00Z", end: "2026-09-29T01:00:00Z", stay_ids: ["stay-1"] }] };
    render(<Root />);
    const row = await screen.findByTestId("row-erased");
    expect(row.textContent).toContain("消した");
    expect(within(row).queryByRole("button", { name: "戻す" })).toBeTruthy();
    expect(within(row).queryByTestId("stay-detail")).toBeNull();
  });
});

describe("キーボード操作", () => {
  // Scenario: キーボードで詳細を開ける
  it("キーボード決定操作で詳細を開閉できる", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getAllByTestId("row-stay")).toHaveLength(2));
    const button = within(screen.getAllByTestId("row-stay")[0]).getByRole("button");
    fireEvent.click(button);
    await screen.findByTestId("stay-detail");
    fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("false");
  });
});
