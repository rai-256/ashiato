// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 設定画面の「衛星と資格情報」の節（ST29 / tasks 9.1 / design D12）。
 * jsdom は実寸を測らない —— 見るのは**指定と勘定だけ**（拒む種別の全部に日本語の理由があること・値が保存領域に書かれないこと・
 * 操作の対象に大きさの指定が付いていること）。実寸・フォーカス・横の揺れは `web/e2e/settings-satellites.spec.ts`。
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";
import { msg } from "../messages";
import { REFUSAL_KINDS, type Satellite } from "../satellites";
import satelliteRs from "../../../crates/server/src/satellite.rs?raw";

const VALUE = "ashsat_dev.example.booklog_AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_ABCDE";

const sat = (identifier: string, name: string, builtin: boolean, labels: string[]): Satellite => ({
  id: `id-${identifier}`,
  identifier,
  name,
  builtin,
  registered_at: "2026-10-01T00:00:00Z",
  removed_at: null,
  credentials: labels.map((label, i) => ({
    id: `c-${identifier}-${i}`,
    label,
    issued_at: "2026-10-10T00:00:00Z",
    last_used_at: i === 0 ? null : "2026-10-21T07:40:00Z",
  })),
});

let satellites: Satellite[];
let posts: { path: string; body: unknown }[];
let refusal: { status: number; error: string } | null;

function res(code: number, body: unknown): Response {
  return { ok: code >= 200 && code < 300, status: code, json: () => Promise.resolve(body) } as Response;
}

beforeEach(() => {
  satellites = [
    sat("ashiato.ai-external", "外部 AI", true, ["Claude Desktop"]),
    sat("ashiato.ai-local", "ローカル AI", true, []),
    sat("dev.example.booklog", "読書ログ", false, ["携帯", "ノート PC"]),
  ];
  posts = [];
  refusal = null;
  window.location.hash = "#/settings";
  vi.stubGlobal("fetch", (path: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    if (path === "/api/satellites" && method === "GET") return Promise.resolve(res(200, satellites));
    if (method !== "GET") posts.push({ path, body: init?.body === undefined ? undefined : JSON.parse(String(init.body)) });
    if (refusal !== null) return Promise.resolve(res(refusal.status, { error: refusal.error }));
    if (method === "POST" && path.endsWith("/credentials")) {
      return Promise.resolve(res(201, { id: "new", label: "タブレット", issued_at: "2026-10-22T00:00:00Z", value: VALUE }));
    }
    if (method === "DELETE") return Promise.resolve({ ok: true, status: 204, json: () => Promise.reject(new Error("no body")) } as Response);
    if (path === "/api/satellites") return Promise.resolve(res(201, sat("dev.example.kakeibo", "家計簿", false, [])));
    return Promise.resolve(res(404, {}));
  });
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.location.hash = "";
  localStorage.clear();
  sessionStorage.clear();
});

async function open(): Promise<HTMLElement> {
  render(<Root />);
  await waitFor(() => expect(screen.getAllByTestId("satellite").length).toBe(3));
  return screen.getByTestId("satellites-section");
}

const card = (identifier: string): HTMLElement =>
  screen.getAllByTestId("satellite").find((e) => e.getAttribute("data-identifier") === identifier)!;

describe("衛星と資格情報の節", () => {
  it("拒む種別の全部に、日本語の理由がある", () => {
    const reasons = msg("satellites").refusal;
    expect(Object.keys(reasons).sort()).toEqual([...REFUSAL_KINDS].sort());
    for (const kind of REFUSAL_KINDS) {
      expect(reasons[kind], kind).toMatch(/\P{ASCII}/u);
    }
  });

  it("口が返す種別ごとに、入力欄の下へ理由を出して衛星を増やさない", async () => {
    for (const kind of REFUSAL_KINDS) {
      cleanup();
      refusal = { status: kind === "name_taken" || kind === "identifier_taken" ? 409 : 400, error: kind };
      await open();
      fireEvent.click(screen.getByRole("button", { name: "衛星を登録" }));
      const form = screen.getByTestId("satellite-register-form");
      fireEvent.change(within(form).getAllByRole("textbox")[0]!, { target: { value: "家計簿" } });
      fireEvent.change(within(form).getAllByRole("textbox")[1]!, { target: { value: "kakeibo" } });
      fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
      await waitFor(() => expect(screen.getByTestId("satellite-reason").textContent, kind).toBe(msg("satellites").refusal[kind]));
      expect(screen.getAllByTestId("satellite").length).toBe(3);
    }
  }, 20_000);

  it("最初の 2 つには「最初から」があって「外す」が無い", async () => {
    await open();
    for (const id of ["ashiato.ai-external", "ashiato.ai-local"]) {
      expect(within(card(id)).queryByTestId("satellite-builtin")?.textContent).toBe("最初から");
      expect(within(card(id)).queryByRole("button", { name: "外す" })).toBeNull();
    }
    expect(within(card("dev.example.booklog")).getByRole("button", { name: "外す" })).toBeTruthy();
  });

  it("発行した値は画面の state にだけ置かれ、保存領域に書かれず、閉じると消える", async () => {
    await open();
    const setItem = vi.spyOn(Storage.prototype, "setItem");
    fireEvent.click(within(card("dev.example.booklog")).getByRole("button", { name: "資格情報を発行" }));
    const form = screen.getByTestId("satellite-issue-form");
    expect(within(form).getByRole("heading").textContent).toContain("読書ログ");
    fireEvent.change(within(form).getByRole("textbox"), { target: { value: "タブレット" } });
    fireEvent.click(within(form).getByRole("button", { name: "発行する" }));
    await waitFor(() => expect(screen.getByTestId("satellite-credential-value").textContent).toBe(VALUE));
    expect(posts[0]).toEqual({ path: "/api/satellites/id-dev.example.booklog/credentials", body: { label: "タブレット" } });
    expect(setItem).not.toHaveBeenCalled();
    expect(localStorage.length + sessionStorage.length).toBe(0);
    expect(window.location.href).not.toContain("ashsat_");
    fireEvent.click(screen.getByRole("button", { name: "設定したので閉じる" }));
    expect(screen.queryByTestId("satellite-credential-value")).toBeNull();
    expect(document.body.textContent).not.toContain("ashsat_");
  });

  it("値のコピーは clipboard に渡す", async () => {
    await open();
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    fireEvent.click(within(card("dev.example.booklog")).getByRole("button", { name: "資格情報を発行" }));
    fireEvent.click(within(screen.getByTestId("satellite-issue-form")).getByRole("button", { name: "発行する" }));
    await waitFor(() => screen.getByTestId("satellite-credential-value"));
    fireEvent.click(screen.getByRole("button", { name: "値をコピー" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(VALUE));
  });

  it("外すの確認は資格情報の本数を並べ、同時に開くのは 1 つ", async () => {
    await open();
    fireEvent.click(within(card("dev.example.booklog")).getByRole("button", { name: "外す" }));
    const confirm = screen.getByTestId("satellite-remove-confirm");
    expect(confirm.textContent).toContain("資格情報 2 本が今すぐ使えなくなります");
    expect(confirm.textContent).toContain("この衛星が書いた記録は消えずに残ります");
    expect(confirm.textContent).toContain("dev.example.booklog");
    fireEvent.click(within(card("dev.example.booklog")).getByRole("button", { name: "資格情報を発行" }));
    expect(screen.queryByTestId("satellite-remove-confirm")).toBeNull();
    expect(screen.getAllByTestId("satellite-issue-form").length).toBe(1);
    fireEvent.click(screen.getByRole("button", { name: "衛星を登録" }));
    expect(screen.queryByTestId("satellite-issue-form")).toBeNull();
    expect(screen.getAllByTestId("satellite-register-form").length).toBe(1);
  });

  it("操作の対象に大きさの指定が付いている（24 px、外すと確認は 44 px）", async () => {
    const section = await open();
    fireEvent.click(within(card("dev.example.booklog")).getByRole("button", { name: "外す" }));
    const px = (el: HTMLElement, key: "minHeight" | "minWidth"): number => parseFloat(el.style[key]);
    const targets = section.querySelectorAll<HTMLElement>("button, input");
    expect(targets.length).toBeGreaterThan(8);
    for (const t of targets) {
      expect(px(t, "minHeight"), `${t.tagName} ${t.textContent}`).toBeGreaterThanOrEqual(24);
      expect(px(t, "minWidth"), `${t.tagName} ${t.textContent}`).toBeGreaterThanOrEqual(24);
    }
    // 小節の「外す」・確認の「外す」・確認の「やめる」
    const big = within(card("dev.example.booklog")).getAllByRole("button", { name: /^(外す|やめる)$/ });
    expect(big.length).toBe(3);
    for (const b of big) {
      expect(px(b, "minHeight"), b.textContent ?? "").toBeGreaterThanOrEqual(44);
      expect(px(b, "minWidth"), b.textContent ?? "").toBeGreaterThanOrEqual(44);
    }
  });
});

describe("理由の文言の上限は satellite.rs の定数と同じ", () => {
  // Scenario: 画面の文言は satellite.rs の定数に合わせたことをテストで確かめる（D5）
  it("識別子 64・名前 40・資格情報の名前 40・2 つ以上の部分が、定数と文言で一致する", () => {
    const src = satelliteRs;
    const num = (name: string): number => {
      const m = new RegExp(`pub const ${name}: usize = (\\d+);`).exec(src);
      if (m === null) throw new Error(`${name} が satellite.rs に無い`);
      return Number(m[1]);
    };
    const m = msg("satellites");
    expect(m.refusal.malformed).toContain(`${num("IDENTIFIER_MAX_LEN")} 文字まで`);
    expect(m.refusal.malformed).toContain("2 つ以上");
    expect(m.refusal.name_invalid).toContain(`1〜${num("SATELLITE_NAME_MAX_LEN")} 文字`);
    expect(m.labelInvalid).toContain(`${num("CREDENTIAL_LABEL_MAX_LEN")} 文字まで`);
    // 形の正本（部分を 1 つ以上つなぐ）も文言の「2 つ以上」と同じ向き
    expect(src).toContain(String.raw`(\.[a-z0-9-]+)+$`);
  });
});
