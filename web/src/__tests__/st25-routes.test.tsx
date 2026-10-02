// SPDX-License-Identifier: AGPL-3.0-only
/**
 * ST25 の行き先（design D12 / tasks 7.1）。ルート・`#/coverage`・解釈できないアドレス・見出し。
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";

const ok = (body: unknown): Response => ({ ok: true, json: () => Promise.resolve(body) }) as Response;
let calls: string[] = [];

const today = (): string =>
  new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Tokyo", year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date());

beforeEach(() => {
  calls = [];
  window.location.hash = "";
  vi.stubGlobal("fetch", (path: string) => {
    calls.push(path);
    if (path.startsWith("/api/day")) {
      const date = /date=(\d{4}-\d{2}-\d{2})/.exec(path)?.[1] ?? "";
      return Promise.resolve(ok({ date, criteria: [], entries: [] }));
    }
    return new Promise<Response>(() => {});
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

describe("ルートと行き先", () => {
  // Scenario: ルートを開くと今日の 1 日の画面が出る
  it("アドレスに何も付けないと今日の 1 日の画面で、稼働状況は出ない", async () => {
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
    expect(screen.queryByTestId("coverage-loading")).toBeNull();
    expect(screen.getByTestId("day-title").textContent).toContain("日");
    await waitFor(() => expect(calls).toContain(`/api/day?date=${today()}`));
  });

  it("#/ でも今日の 1 日の画面", async () => {
    window.location.hash = "#/";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
  });

  // Scenario: 稼働状況は専用のアドレスで開く
  it("#/coverage で稼働状況の画面が出る", () => {
    window.location.hash = "#/coverage";
    render(<Root />);
    expect(screen.getByTestId("coverage-loading")).toBeTruthy();
    expect(screen.queryByTestId("day-view")).toBeNull();
    expect(screen.getByTestId("to-master").getAttribute("href")).toBe("#/master");
  });

  // Scenario: 稼働状況から 1 日の画面へ移れる
  it("稼働状況の「1 日の画面へ」を選ぶと今日の 1 日の画面が出る", async () => {
    window.location.hash = "#/coverage";
    render(<Root />);
    const link = screen.getByTestId("to-day") as HTMLAnchorElement;
    expect(link.textContent).toBe("1 日の画面へ");
    expect(link.getAttribute("href")).toBe("#/day/");
    window.location.hash = link.getAttribute("href") ?? "";
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
    await waitFor(() => expect(calls).toContain(`/api/day?date=${today()}`));
  });

  // Scenario: 解釈できないアドレスでは今日の 1 日の画面が出る
  it("#/nowhere では今日の 1 日の画面が出る", async () => {
    window.location.hash = "#/nowhere";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
    await waitFor(() => expect(calls).toContain(`/api/day?date=${today()}`));
  });

  it("1 日の画面の上に「稼働状況」「マスタ管理」の行き先がある", async () => {
    window.location.hash = "#/day/2026-09-07";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-view")).toBeTruthy());
    expect(screen.getByRole("link", { name: "稼働状況" }).getAttribute("href")).toBe("#/coverage");
    expect(screen.getByRole("link", { name: "マスタ管理" }).getAttribute("href")).toBe("#/master");
  });
});

describe("見出し", () => {
  // Scenario: 1 日の画面の見出しは日付
  it("#/day/2026-09-07 の見出しは「2026 年 9 月 7 日」で「の滞在」が付かない", async () => {
    window.location.hash = "#/day/2026-09-07";
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("day-title").textContent).toBe("2026 年 9 月 7 日"));
    expect(screen.getByTestId("day-title").textContent).not.toContain("の滞在");
  });
});

describe("暦に無い日付", () => {
  it("暦に無い日付の画面は OS の明暗に追従する（C10）", () => {
    vi.stubGlobal("matchMedia", (q: string) => ({
      matches: q.includes("light"),
      addEventListener: () => {},
      removeEventListener: () => {},
    }));
    window.location.hash = "#/day/2026-13-45";
    render(<Root />);
    const el = screen.getByTestId("day-invalid");
    expect(el.getAttribute("data-scheme")).toBe("light");
  });
});
