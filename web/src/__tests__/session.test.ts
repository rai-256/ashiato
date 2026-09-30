// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 画面のログイン（ST28 / design D10）。
 *
 * 起動時に `GET /api/session`。401 なら合言葉の入力欄、通れば中身と「ログアウト」。
 * どの面の読み出しでも 401 が返れば入力欄へ戻す。**合言葉は画面のどこにも残さない。**
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { Gate } from "../session";

const inner = createElement("p", { "data-testid": "inner" }, "記録");
const mount = (): void => {
  render(createElement(Gate, null, inner));
};

type Reply = { status: number };
function stubFetch(handler: (url: string, init?: RequestInit) => Reply): ReturnType<typeof vi.fn> {
  const fn = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const r = handler(String(input), init);
    return new Response(r.status === 204 ? null : "{}", { status: r.status });
  });
  vi.stubGlobal("fetch", fn);
  return fn;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("画面のログイン", () => {
  it("起動時の GET /api/session が 401 なら合言葉の入力欄を出し、中身は描かない", async () => {
    stubFetch(() => ({ status: 401 }));
    mount();
    expect(await screen.findByLabelText("合言葉")).toBeTruthy();
    expect(screen.queryByTestId("inner")).toBeNull();
  });

  it("通っていれば中身と「ログアウト」を出す", async () => {
    stubFetch(() => ({ status: 200 }));
    mount();
    expect(await screen.findByTestId("inner")).toBeTruthy();
    expect(screen.getByRole("button", { name: "ログアウト" })).toBeTruthy();
  });

  it("合言葉を POST /api/session で送り、204 なら中身へ進む", async () => {
    const fn = stubFetch((_url, init) => {
      if (init?.method === "POST") return { status: 204 };
      return { status: fn.mock.calls.some((c) => (c[1] as RequestInit | undefined)?.method === "POST") ? 200 : 401 };
    });
    mount();
    fireEvent.change(await screen.findByLabelText("合言葉"), { target: { value: "x" } });
    fireEvent.click(screen.getByRole("button", { name: "ログイン" }));
    expect(await screen.findByTestId("inner")).toBeTruthy();
    const post = fn.mock.calls.find((c) => (c[1] as RequestInit | undefined)?.method === "POST");
    expect(String(post?.[0])).toBe("/api/session");
    expect(JSON.parse(String((post?.[1] as RequestInit).body))).toEqual({ password: "x" });
  });

  it("違う合言葉（401）では入力欄が残り、断られたと出る", async () => {
    stubFetch(() => ({ status: 401 }));
    mount();
    fireEvent.change(await screen.findByLabelText("合言葉"), { target: { value: "x" } });
    fireEvent.click(screen.getByRole("button", { name: "ログイン" }));
    await waitFor(() => expect(screen.getByRole("alert")).toBeTruthy());
    expect(screen.getByLabelText("合言葉")).toBeTruthy();
    expect(screen.queryByTestId("inner")).toBeNull();
  });

  it("中身の読み出しが 401 を返したら入力欄へ戻す", async () => {
    stubFetch((url) => ({ status: url === "/api/session" ? 200 : 401 }));
    mount();
    await screen.findByTestId("inner");
    await fetch("/api/coverage");
    expect(await screen.findByLabelText("合言葉")).toBeTruthy();
    expect(screen.queryByTestId("inner")).toBeNull();
  });

  it("ログアウトは DELETE /api/session を送って入力欄へ戻す", async () => {
    const fn = stubFetch(() => ({ status: 200 }));
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "ログアウト" }));
    expect(await screen.findByLabelText("合言葉")).toBeTruthy();
    const del = fn.mock.calls.find((c) => (c[1] as RequestInit | undefined)?.method === "DELETE");
    expect(String(del?.[0])).toBe("/api/session");
  });

  it("入力欄と「ログイン」は 44 px・文字は 24 px 以上（ui-direction の下限）", async () => {
    stubFetch(() => ({ status: 401 }));
    mount();
    const input = await screen.findByLabelText("合言葉");
    const button = screen.getByRole("button", { name: "ログイン" });
    for (const el of [input, button]) {
      expect(parseInt((el as HTMLElement).style.minHeight, 10)).toBeGreaterThanOrEqual(44);
      expect(parseInt((el as HTMLElement).style.fontSize, 10)).toBeGreaterThanOrEqual(24);
    }
  });
});
