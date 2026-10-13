// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 設定画面の「全記録の書き出し」の節（ST33 / tasks 8.2 / design D13）。
 * 応答を固定して描画と操作を見る。何を返すかは Rust の結合テストが見る。
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import type { ExportStatus, Summary } from "../export";

const SUMMARY: Summary = {
  sources: [
    { logical_source: "c01-gps", display_name: "GPS", count: 1234, first: "2026-01-01T00:00:00Z", last: "2026-03-01T00:00:00Z" },
  ],
  estimate: { rows: 1234, bytes: 85_000_000, seconds: 7 },
};
const LAST = { finished_at: "2026-10-01T03:00:00Z", rows: 99, bytes: 5000, file: "/data/export/ashiato.zip", erased_after: 0 };
const PW = "correct-horse-1";

let calls: { path: string; init?: RequestInit }[] = [];
let status: ExportStatus;
let postResult: { status: number; body: unknown } | "reject";
let summaryStatus = 200;

function res(code: number, body: unknown): Response {
  return { ok: code >= 200 && code < 300, status: code, json: () => Promise.resolve(body) } as Response;
}

function serve(): void {
  calls = [];
  vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
    calls.push({ path, init });
    if (path === "/api/export/summary") return Promise.resolve(res(summaryStatus, SUMMARY));
    if (path === "/api/export" && init?.method === "POST") {
      if (postResult === "reject") return Promise.reject(new Error("down"));
      return Promise.resolve(res(postResult.status, postResult.body));
    }
    if (path === "/api/export") return Promise.resolve(res(200, status));
    return Promise.resolve(res(404, {}));
  });
}

const posts = (): typeof calls => calls.filter((c) => c.init?.method === "POST");
const gets = (p: string): typeof calls => calls.filter((c) => c.path === p && c.init?.method !== "POST");

beforeEach(() => {
  status = { leftover: [] };
  postResult = { status: 202, body: { export_id: "u" } };
  summaryStatus = 200;
  window.location.hash = "#/settings";
  serve();
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.location.hash = "";
  localStorage.clear();
  sessionStorage.clear();
});

async function open(): Promise<void> {
  render(<Root />);
  await waitFor(() => expect(screen.getByTestId("export-sources")).toBeTruthy());
  await waitFor(() => expect((screen.getByTestId("export-start") as HTMLButtonElement).disabled).toBe(false));
}

async function openConfirm(): Promise<void> {
  await open();
  fireEvent.click(screen.getByTestId("export-start"));
}

const pwInputs = (): HTMLInputElement[] =>
  Array.from(screen.getByTestId("export-confirm").querySelectorAll<HTMLInputElement>("input[type=password]"));
const goButton = (): HTMLButtonElement => screen.getByRole("button", { name: "書き出す" });

function type(a: string, b: string): void {
  const [i1, i2] = pwInputs();
  fireEvent.change(i1, { target: { value: a } });
  fireEvent.change(i2, { target: { value: b } });
}

describe("設定の書き出しの節", () => {
  it("件数・期間と入らないもの（件数なし）を出し、開いたとき summary と状態を 1 回ずつ読む", async () => {
    await open();
    expect(screen.getByTestId("export-sources").textContent).toContain("GPS");
    expect(screen.getByTestId("export-sources").textContent).toContain("1,234");
    expect(screen.getByTestId("export-sources").textContent).toContain("2026/01/01");
    const ex = screen.getByTestId("export-excluded");
    expect(ex.querySelectorAll("li")).toHaveLength(3);
    expect(ex.textContent).not.toMatch(/\d+ ?件/);
    expect(gets("/api/export/summary")).toHaveLength(1);
    expect(gets("/api/export")).toHaveLength(1);
  });

  it("S-1 の見出しの行から「設定」へ行ける", async () => {
    window.location.hash = "";
    vi.stubGlobal("fetch", () => Promise.resolve(res(200, [])));
    render(<Root />);
    expect((screen.getByTestId("to-settings") as HTMLAnchorElement).getAttribute("href")).toBe("#/settings");
  });

  it("押しただけでは POST せず、確認が出て合言葉の入力にフォーカスが移る", async () => {
    await openConfirm();
    const dlg = screen.getByRole("dialog");
    expect(dlg.textContent).toContain("約 1,234 行");
    expect(dlg.textContent).toContain("合言葉は保存しません");
    expect(dlg.textContent).toContain("7-Zip");
    expect(dlg.textContent).not.toContain("前回の書き出しのファイルは消えます");
    expect(posts()).toHaveLength(0);
    expect(document.activeElement).toBe(pwInputs()[0]);
    for (const i of pwInputs()) expect(i.getAttribute("autocomplete")).toBe("off");
    expect(dlg.querySelector("form")?.getAttribute("action")).toBeNull();
    expect(document.querySelector("a[href^='file:'], a[download]")).toBeNull();
  });

  it("前回の書き出しがあるときだけ「前回のファイルは消えます」を出す", async () => {
    status = { leftover: [], last: LAST };
    await openConfirm();
    expect(screen.getByRole("dialog").textContent).toContain("書き終えると前回の書き出しのファイルは消えます");
  });

  it("11 文字と不一致では「書き出す」が disabled で理由が出る", async () => {
    await openConfirm();
    type("a".repeat(11), "a".repeat(11));
    expect(goButton().disabled).toBe(true);
    expect(screen.getByTestId("export-reason").textContent).toContain("12 文字以上");
    type("a".repeat(12), "a".repeat(11) + "b");
    expect(goButton().disabled).toBe(true);
    expect(screen.getByTestId("export-reason").textContent).toContain("一致していません");
    expect(screen.getByTestId("export-reason").getAttribute("aria-live")).toBe("polite");
  });

  it("文字数は符号位置: 絵文字 11 個は disabled、12 個は押せる", async () => {
    await openConfirm();
    type("😀".repeat(11), "😀".repeat(11));
    expect(goButton().disabled).toBe(true);
    type("😀".repeat(12), "😀".repeat(12));
    expect(goButton().disabled).toBe(false);
  });

  it("12 文字の一致で押すと本体に合言葉を持つ POST が 1 回だけ。入力も保存領域も URL も合言葉を持たない", async () => {
    await openConfirm();
    type(PW, PW);
    fireEvent.click(goButton());
    await waitFor(() => expect(screen.queryByTestId("export-confirm")).toBeNull());
    expect(posts()).toHaveLength(1);
    expect(posts()[0].path).toBe("/api/export");
    expect(JSON.parse(posts()[0].init?.body as string)).toEqual({ password: PW });
    expect(document.body.innerHTML).not.toContain(PW);
    expect(document.querySelectorAll("input[type=password]")).toHaveLength(0);
    expect(JSON.stringify({ ...localStorage })).not.toContain(PW);
    expect(JSON.stringify({ ...sessionStorage })).not.toContain(PW);
    expect(window.location.href).not.toContain(PW);
    // 開き直しても前の合言葉は残っていない
    fireEvent.click(screen.getByTestId("export-start"));
    expect(pwInputs().map((i) => i.value)).toEqual(["", ""]);
  });

  it("「やめる」では POST せず、入力を空にして押す前の形に戻る", async () => {
    await openConfirm();
    type(PW, PW);
    fireEvent.click(screen.getByRole("button", { name: "やめる" }));
    expect(screen.queryByTestId("export-confirm")).toBeNull();
    expect(posts()).toHaveLength(0);
    expect(screen.getByTestId("export-start")).toBeTruthy();
  });

  it("409 は節に 1 行出して入力が消え、状態を読み直す", async () => {
    postResult = { status: 409, body: { error: "export_running" } };
    await openConfirm();
    type(PW, PW);
    const before = gets("/api/export").length;
    fireEvent.click(goButton());
    await waitFor(() => expect(screen.getByTestId("export-refusal").textContent).toContain("すでに書き出し中"));
    expect(document.querySelectorAll("input[type=password]")).toHaveLength(0);
    expect(document.body.innerHTML).not.toContain(PW);
    expect(gets("/api/export").length).toBeGreaterThan(before);
    expect((screen.getByTestId("export-start") as HTMLButtonElement).disabled).toBe(false);
  });

  it("400 と通信の失敗も 1 行を出して押す前の形に戻る", async () => {
    postResult = { status: 400, body: { error: "password_too_short" } };
    await openConfirm();
    type(PW, PW);
    fireEvent.click(goButton());
    await waitFor(() => expect(screen.getByTestId("export-refusal").textContent).toContain("合言葉が短すぎます"));
    postResult = "reject";
    fireEvent.click(screen.getByTestId("export-start"));
    type(PW, PW);
    fireEvent.click(goButton());
    await waitFor(() => expect(screen.getByTestId("export-refusal").textContent).toContain("通信できませんでした"));
    expect(document.querySelectorAll("input[type=password]")).toHaveLength(0);
  });

  it("running の間は「全記録を書き出す」を押せず、1 秒ごとに読み直し、消えたら止まる", async () => {
    status = { leftover: [], running: { export_id: "u", started_at: "2026-10-13T00:00:00Z", written: 10, total: 100 } };
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(<Root />);
    await waitFor(() => expect(screen.getByTestId("export-progress")).toBeTruthy());
    expect((screen.getByTestId("export-start") as HTMLButtonElement).disabled).toBe(true);
    const n0 = gets("/api/export").length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    expect(gets("/api/export").length).toBeGreaterThanOrEqual(n0 + 3);
    status = { leftover: [], last: LAST };
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1100);
    });
    await waitFor(() => expect(screen.queryByTestId("export-progress")).toBeNull());
    const n1 = gets("/api/export").length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(gets("/api/export").length).toBe(n1);
    expect((screen.getByTestId("export-start") as HTMLButtonElement).disabled).toBe(false);
  });

  it("失敗・途中で止まった・消せなかったファイルは 1 行ずつ、押し直せる", async () => {
    status = {
      leftover: ["/data/export/old.zip"],
      last_failure: { at: "2026-10-01T00:00:00Z", kind: "io" },
      interrupted: { started_at: "2026-10-02T00:00:00Z", file: "/data/export/part.zip" },
    };
    await open();
    const lines = screen.getAllByTestId("export-line").map((l) => l.textContent);
    expect(lines).toHaveLength(3);
    expect(lines.join("\n")).toContain("書き出しに失敗しました（io）");
    expect(lines.join("\n")).toContain("/data/export/part.zip が残っています");
    expect(lines.join("\n")).toContain("/data/export/old.zip");
    expect((screen.getByTestId("export-start") as HTMLButtonElement).disabled).toBe(false);
  });

  it("erased_after が 0 なら注記を出さず、1 以上なら role=note で出す", async () => {
    status = { leftover: [], last: LAST };
    await open();
    expect(screen.queryByRole("note")).toBeNull();
    cleanup();
    status = { leftover: [], last: { ...LAST, erased_after: 3 } };
    serve();
    render(<Root />);
    await waitFor(() => expect(screen.getByRole("note").textContent).toContain("消した記録が 3 件"));
  });
});
