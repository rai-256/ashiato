import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CollectorSetupPanel } from "../CollectorSetupPanel";
import { SCHEMES } from "../tokens";
import { parseSetupString } from "../collectorSetup";

const PASSWORD = "correct-horse-1";

function response(status: number, body: unknown): Response {
  return { ok: status >= 200 && status < 300, status, json: () => Promise.resolve(body) } as Response;
}

afterEach(() => vi.unstubAllGlobals());

describe("収集アプリの設定", () => {
  it("合言葉を入れ直すと設定の文字列を表示する", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(response(200, { token: "token-for-test-0123456789" }))));
    render(<CollectorSetupPanel colors={SCHEMES.dark} />);

    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: PASSWORD } });
    fireEvent.click(screen.getByRole("button", { name: "表示" }));

    const output = await screen.findByTestId("collector-setup-value");
    const parsed = parseSetupString((output as HTMLInputElement).value);
    expect(parsed).toEqual({ url: `${window.location.origin}/api`, token: "token-for-test-0123456789" });
    expect(screen.getByRole("button", { name: "値をコピー" })).toBeTruthy();
  });

  it("違う合言葉では設定の文字列を表示しない", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(response(403, { error: "wrong_password" }))));
    render(<CollectorSetupPanel colors={SCHEMES.dark} />);

    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "wrong-password-1" } });
    fireEvent.click(screen.getByRole("button", { name: "表示" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("合言葉が違います"));
    expect(screen.queryByTestId("collector-setup-value")).toBeNull();
  });
});
