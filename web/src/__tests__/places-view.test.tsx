// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所のカード（ST21 / tasks 8.3 / design D12）。応答を固定して描画の指定と勘定を見る。
 * 画面の Scenario の印は `web/e2e`（本物のブラウザ）が持つ。ここは補助。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlacesView } from "../PlacesView";
import type { Place, PlacesData } from "../places";

const ok = (body: unknown): Response => ({ ok: true, status: 200, json: () => Promise.resolve(body) }) as Response;
const HOURS = Array.from({ length: 24 }, () => 0);
const T = { written_at: "2026-09-01T10:00:00+09:00", ingested_at: "2026-09-01T01:00:00Z" };

function place(over: Partial<Place> & Pick<Place, "id" | "name">): Place {
  return {
    note: null,
    radius_m: 100,
    name_record: { record_id: "n", ...T },
    coord: { record_id: "c", lat: 35.681236, lon: 139.767125, change: "first", valid_from: null, supersedes: null, ...T },
    stays: { count: 2, minutes: 150, last_day: "2026-09-30", hours: [...HOURS.slice(0, 9), 120, 30, ...HOURS.slice(11)] },
    previous_names: [],
    previous_coords: [],
    ...over,
  };
}

function draw(places: Place[] | Response | Error): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(() => {
      if (places instanceof Error) return Promise.reject(places);
      if (places instanceof Response) return Promise.resolve(places);
      return Promise.resolve(ok({ today: "2026-10-01", places } satisfies PlacesData));
    }),
  );
  render(<PlacesView scheme="dark" />);
}

beforeEach(() => vi.unstubAllGlobals());
afterEach(() => vi.unstubAllGlobals());

describe("places-view", () => {
  it("カードは読み出した順に 1 枚ずつ、名前・合計・最後に居た日・広さ・座標が出る", async () => {
    draw([place({ id: "b", name: "職場" }), place({ id: "a", name: "自宅", radius_m: 200, stays: { count: 1, minutes: 45, last_day: "2026-09-28", hours: HOURS } })]);
    const cards = await screen.findAllByTestId("place-card");
    expect(cards.map((c) => within(c).getByTestId("place-name").textContent)).toEqual(["職場", "自宅"]);
    expect(within(cards[0]).getByTestId("place-total").textContent).toBe("2 時間");
    expect(within(cards[0]).getByTestId("place-meta").textContent).toBe("最後に居た日 2026-09-30 · 広さ 100 m");
    expect(within(cards[0]).getByTestId("place-coord").textContent).toBe("35.6812, 139.7671");
    expect(within(cards[1]).getByTestId("place-total").textContent).toBe("45 分");
    expect(within(cards[1]).getByTestId("place-meta").textContent).toContain("広さ 200 m");
  });

  it("滞在が無い場所は「まだ居たことが無い」", async () => {
    draw([place({ id: "a", name: "新居", stays: { count: 0, minutes: 0, last_day: null, hours: HOURS } })]);
    const meta = await screen.findByTestId("place-meta");
    expect(meta.textContent).toBe("まだ居たことが無い · 広さ 100 m");
  });

  it("帯は 24 区分で、0 の区分も枠を描き、濃さは最大の区分との割合", async () => {
    draw([place({ id: "a", name: "職場" })]);
    const band = await screen.findByTestId("place-band");
    const cells = [...band.querySelectorAll("[data-hour]")] as HTMLElement[];
    expect(cells).toHaveLength(24);
    for (const el of cells) expect(el.style.border, "枠が無い").not.toBe("");
    expect(cells.map((e) => Number(e.dataset.level))).toEqual([...HOURS.slice(0, 9), 1, 0.25, ...HOURS.slice(11)]);
  });

  it("前の名前・座標は押したときだけ出て、件数は名前と座標の和（予定も数える）", async () => {
    draw([
      place({
        id: "a",
        name: "本社",
        previous_names: [{ record_id: "p", name: "旧オフィス", ...T }],
        previous_coords: [
          { record_id: "k1", lat: 1, lon: 2, change: "first", state: "fixed", valid_from: null, supersedes: null, fixed_by: "f", ...T },
          { record_id: "k2", lat: 3, lon: 4, change: "move", state: "upcoming", valid_from: { precision: "month", date: "2027-01" }, supersedes: null, fixed_by: null, ...T },
        ],
      }),
    ]);
    const btn = await screen.findByRole("button", { name: /前の名前・座標 3/ });
    expect(btn.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText(/旧オフィス/)).toBeNull();
    fireEvent.click(btn);
    expect(btn.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText(/旧オフィス/)).toBeTruthy();
    const rows = screen.getAllByTestId("previous-coord").map((r) => r.textContent);
    expect(rows[0]).toContain("直した");
    expect(rows[1]).toContain("予定（2027-01 から）");
  });

  it("前の名前も座標も無ければ「前の名前・座標」を出さない", async () => {
    draw([place({ id: "a", name: "職場" })]);
    await screen.findByTestId("place-card");
    expect(screen.queryByRole("button", { name: /前の名前・座標/ })).toBeNull();
  });

  it("場所の識別子は画面に出ない", async () => {
    draw([place({ id: "11111111-aaaa-4bbb-8ccc-222222222222", name: "職場", previous_names: [{ record_id: "99999999-aaaa-4bbb-8ccc-222222222222", name: "旧", ...T }] })]);
    fireEvent.click(await screen.findByRole("button", { name: /前の名前・座標/ }));
    expect(document.body.textContent).not.toContain("11111111");
  });

  it("場所が無いと「場所がまだありません」と出る（失敗とは別）", async () => {
    draw([]);
    expect((await screen.findByTestId("places-empty")).textContent).toContain("場所がまだありません");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("読み出しの失敗は失敗として出し、「場所が無い」と読ませない", async () => {
    for (const r of [new Error("net"), { ok: false, status: 500, json: () => Promise.resolve({}) } as Response, ok({ today: "x", places: [{ id: 1 }] })]) {
      vi.unstubAllGlobals();
      const { unmount } = render(<div />);
      unmount();
      draw(r);
      await waitFor(() => expect(screen.getByTestId("places-failed")).toBeTruthy());
      expect(screen.queryByTestId("places-empty")).toBeNull();
      document.body.innerHTML = "";
    }
  });

  it("地図も位置情報も使わず、求めの宛先は /api/places だけ", async () => {
    draw([place({ id: "a", name: "職場" })]);
    await screen.findByTestId("place-card");
    const calls = (fetch as unknown as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0] as string);
    expect(calls).toEqual(["/api/places"]);
    expect(document.querySelectorAll("iframe, img, canvas, link[href]")).toHaveLength(0);
  });

  it("操作できるものは 24 px 以上を要求し、フォーカスの輪郭の印が付く", async () => {
    draw([place({ id: "a", name: "職場", previous_names: [{ record_id: "p", name: "旧", ...T }] })]);
    await screen.findByTestId("place-card");
    const targets = [...document.querySelectorAll("button")];
    expect(targets.length).toBeGreaterThan(0);
    for (const el of targets) {
      expect(parseFloat(el.style.minHeight)).toBeGreaterThanOrEqual(24);
      expect(parseFloat(el.style.minWidth)).toBeGreaterThanOrEqual(24);
      expect(el.hasAttribute("data-focus-ring")).toBe(true);
    }
  });
});
