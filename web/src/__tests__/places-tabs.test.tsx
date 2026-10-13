// SPDX-License-Identifier: AGPL-3.0-only
/**
 * マスタ管理のタブ（ST21 / tasks 8.2 / design D12）。個人属性（`#/master`）と場所（`#/master/places`）の 2 つで、
 * 押すと hash が変わり、`#/master` は個人属性を開く。
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Root } from "../Root";

const ok = (body: unknown): Response => ({ ok: true, status: 200, json: () => Promise.resolve(body) }) as Response;

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) =>
      Promise.resolve(url.includes("/places") ? ok({ today: "2026-10-01", places: [] }) : ok({ today: "2026-10-01", kinds: [] })),
    ),
  );
});
afterEach(() => {
  vi.unstubAllGlobals();
  window.location.hash = "";
});

describe("places-tabs", () => {
  it("#/master は個人属性を開き、タブは個人属性と場所の 2 つで人物は無い", async () => {
    window.location.hash = "#/master";
    render(<Root />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["個人属性", "場所"]);
    expect(screen.getByRole("tab", { name: "個人属性" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tab", { name: "場所" }).getAttribute("aria-selected")).toBe("false");
    expect(screen.queryByRole("tab", { name: "人物" })).toBeNull();
    expect(screen.queryByTestId("places-view")).toBeNull();
  });

  it("#/master/places は場所を開き、個人属性を読みに行かない", async () => {
    window.location.hash = "#/master/places";
    render(<Root />);
    await screen.findByTestId("places-empty");
    expect(screen.getByRole("tab", { name: "場所" }).getAttribute("aria-selected")).toBe("true");
    const urls = (fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0] as string);
    expect(urls.some((u) => u.includes("/attributes"))).toBe(false);
  });

  it("タブを押すと hash が変わり、画面が切り替わる", async () => {
    window.location.hash = "#/master";
    render(<Root />);
    await waitFor(() => expect(screen.queryByTestId("master-loading")).toBeNull());
    fireEvent.click(screen.getByRole("tab", { name: "場所" }));
    expect(window.location.hash).toBe("#/master/places");
    await screen.findByTestId("places-view");
    fireEvent.click(screen.getByRole("tab", { name: "個人属性" }));
    expect(window.location.hash).toBe("#/master");
    await waitFor(() => expect(screen.queryByTestId("places-view")).toBeNull());
  });
});
