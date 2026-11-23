import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CollectorTokenChangePanel } from "../CollectorTokenChangePanel";
import { WebPasswordChangePanel } from "../WebPasswordChangePanel";
import { SCHEMES } from "../tokens";
import { parseSetupString } from "../collectorSetup";

function response(status: number, body: unknown = null): Response {
  return { ok: status >= 200 && status < 300, status, json: () => Promise.resolve(body) } as Response;
}

afterEach(() => vi.unstubAllGlobals());

describe("画面の合言葉を変える", () => {
  it("正しい現在の合言葉で送信すると成功文と入力消去を表示する", async () => {
    const fetcher = vi.fn(() => Promise.resolve(response(204)));
    vi.stubGlobal("fetch", fetcher);
    render(<WebPasswordChangePanel colors={SCHEMES.dark} />);

    fireEvent.change(screen.getByLabelText("今の合言葉"), { target: { value: "old-password-012345" } });
    fireEvent.change(screen.getByLabelText("新しい合言葉"), { target: { value: "new-password-012345" } });
    fireEvent.change(screen.getByLabelText("新しい合言葉（もう一度）"), { target: { value: "new-password-012345" } });
    fireEvent.click(screen.getByRole("button", { name: "変える" }));

    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("合言葉を変えました。ほかの端末のログインは切れました"));
    expect(fetcher).toHaveBeenCalledWith("/api/web-password", expect.objectContaining({ method: "POST" }));
    expect((screen.getByLabelText("今の合言葉") as HTMLInputElement).value).toBe("");
  });

  it("違う現在の合言葉では失敗文を表示する", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(response(403, { error: "wrong_password" }))));
    render(<WebPasswordChangePanel colors={SCHEMES.dark} />);
    fireEvent.change(screen.getByLabelText("今の合言葉"), { target: { value: "wrong-password-012345" } });
    fireEvent.change(screen.getByLabelText("新しい合言葉"), { target: { value: "new-password-012345" } });
    fireEvent.change(screen.getByLabelText("新しい合言葉（もう一度）"), { target: { value: "new-password-012345" } });
    fireEvent.click(screen.getByRole("button", { name: "変える" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("合言葉が違います"));
  });
});

describe("収集アプリの値を変える", () => {
  it("成功すると新しい設定の文字列とコピーのボタンを表示する", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(response(200, { token: "new-token-0123456789" }))));
    render(<CollectorTokenChangePanel colors={SCHEMES.dark} />);
    fireEvent.change(screen.getByLabelText("今の合言葉"), { target: { value: "old-password-012345" } });
    fireEvent.click(screen.getByRole("button", { name: "変える" }));
    const output = await screen.findByTestId("collector-token-change-value");
    expect(parseSetupString((output as HTMLInputElement).value)).toEqual({
      url: `${window.location.origin}/api`,
      token: "new-token-0123456789",
    });
    expect(screen.getByRole("button", { name: "値をコピー" })).toBeTruthy();
    expect(screen.getByText(/携帯に入れる文字列は/)).toBeTruthy();
  });
});
