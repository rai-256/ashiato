// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 画面そのもの（review/code.md の R10 / R19 / C3 / H-2 / F11）。
 *
 * **`App` には検査が 1 本も無かった** —— 窓の決め方も、取得の失敗も、
 * サーバの並び順どおりに描くことも、丸ごと検査の外にあった。
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App, todayInTz, yearRange } from "../App";
import { achievement, fiveSources } from "./fixtures";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("窓の決め方", () => {
  it("日を Asia/Tokyo で切る（UTC ではない）", () => {
    // JST の 2026-03-02 01:00 = UTC の 2026-03-01 16:00
    const at = new Date("2026-03-01T16:00:00Z");
    expect(todayInTz(at)).toBe("2026-03-02");
    expect(yearRange(at).to).toBe("2026-03-02");
    // **UTC で切ると前日になる** —— 毎日 9 時間だけ今日が格子から消えていた
    expect(at.toISOString().slice(0, 10)).toBe("2026-03-01");
  });

  it("JST の日中は両者が一致する（検査が空振りしていないこと）", () => {
    const noon = new Date("2026-03-02T03:00:00Z");
    expect(todayInTz(noon)).toBe("2026-03-02");
    expect(noon.toISOString().slice(0, 10)).toBe("2026-03-02");
  });

  it("53 週ぶんを引く", () => {
    const { from, to } = yearRange(new Date("2026-03-02T03:00:00Z"));
    const span = (Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 86_400_000;
    expect(span).toBe(53 * 7 - 1);
  });
});

describe("取得の失敗とデータが無いことを混ぜない", () => {
  const ok = (body: unknown): Response =>
    ({ ok: true, json: () => Promise.resolve(body) }) as Response;

  it("読み込み中は「データが無い」と見えない", async () => {
    vi.stubGlobal("fetch", () => new Promise<Response>(() => {}));
    render(<App />);
    expect(screen.getByTestId("coverage-loading")).toBeTruthy();
    expect(screen.getByTestId("achievement-loading")).toBeTruthy();
  });

  it("達成の取得が落ちても、取れた格子は描かれる", async () => {
    // **`Promise.all` にしていたときは、片方の失敗で 5 本とも消えた**
    vi.stubGlobal("fetch", (path: string) =>
      path.includes("achievement")
        ? Promise.resolve({ ok: false, status: 500 } as Response)
        : Promise.resolve(ok(fiveSources("2026-01-04", 60))),
    );
    render(<App />);
    await waitFor(() => expect(screen.getByTestId("achievement-error")).toBeTruthy());
    expect(screen.getAllByTestId(/^grid-/)).toHaveLength(5);
    expect(screen.getByTestId("achievement-error").textContent).toContain("0 なのではありません");
  });

  it("稼働状況の取得が落ちたら、収集が止まったのではないと出る", async () => {
    vi.stubGlobal("fetch", (path: string) =>
      path.includes("achievement")
        ? Promise.resolve(ok(achievement()))
        : Promise.resolve({ ok: false, status: 503 } as Response),
    );
    render(<App />);
    await waitFor(() => expect(screen.getByTestId("coverage-error")).toBeTruthy());
    const text = screen.getByTestId("coverage-error").textContent ?? "";
    expect(text).toContain("status_503");
    expect(text).toContain("収集が止まったのではありません");
    // 達成のほうは描かれている
    expect(screen.getByTestId("achievement")).toBeTruthy();
  });

  it("ソースが 0 本なら、その理由を出す（無言の空白にしない）", async () => {
    vi.stubGlobal("fetch", (path: string) =>
      path.includes("achievement")
        ? Promise.resolve(ok(achievement()))
        : Promise.resolve(ok([])),
    );
    render(<App />);
    await waitFor(() => expect(screen.getByTestId("coverage-empty")).toBeTruthy());
  });

  it("サーバが返した順に格子を並べる", async () => {
    const sources = fiveSources("2026-01-04", 60);
    vi.stubGlobal("fetch", (path: string) =>
      path.includes("achievement")
        ? Promise.resolve(ok(achievement()))
        : Promise.resolve(ok(sources)),
    );
    render(<App />);
    await waitFor(() =>
      expect(screen.getAllByTestId(/^grid-/)).toHaveLength(5),
    );
    expect(
      screen.getAllByTestId(/^grid-/).map((g) => g.getAttribute("data-testid")),
    ).toEqual(sources.map((s) => `grid-${s.logical_source}`));
  });
});
