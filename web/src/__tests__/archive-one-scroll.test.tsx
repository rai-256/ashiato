// SPDX-License-Identifier: AGPL-3.0-only
/**
 * **書庫のソースを足してもひとスクロールの予算を超えない**（design D12 / tasks 10.4）。
 *
 * 予算の定数は `collection-coverage` のもの（`ONE_SCROLL_PX` / `VIEWPORT_H_PX`）を
 * そのまま使い、**箱の高さぶんだけ除く**（除ける量の上限は 160 px。第 2 回 Q9）。
 *
 * jsdom は実寸を測らないので、これは**指定の勘定**であって実寸ではない
 * （`layout.ts` の注記）。
 */
// Scenario: 書庫のソースを足しても Must の 5 本はひとスクロール以内
// Scenario: 書庫のソースの格子は 360 px に収まる
// Scenario: 書庫のソースの週の帯は 24 px 以上
// Scenario: 箱が上限の高さのとき 2 ソースが 800 px に収まる
// Scenario: 箱が上限の高さのとき 5 ソースが 1,440 px に収まる
// Scenario: 箱の高さを除く量は 160 px を超えない
import { render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { ARCHIVE_BOX_MAX_PX } from "../LatestArchive";
import { MIN_TARGET_PX, ONE_SCROLL_PX, VIEWPORT_H_PX } from "../tokens";
import type { ArchivesStatus } from "../archives";
import { achievement, days, fiveSources, source } from "./fixtures";
import { bottomOf, declaredHeight, px } from "./layout";

afterEach(() => {
  vi.unstubAllGlobals();
});

/** 登録簿にある書庫のソース 12 本（固定の 10 本 + マイアクティビティ 2 本）。 */
const ARCHIVE_SOURCES = [
  "c03-timeline-visit",
  "c03-timeline-move",
  "c03-timeline-route",
  "c03-timeline-signal",
  "c03-legacy-location",
  "c03-legacy-visit",
  "c03-legacy-activity",
  "c03-youtube-watch",
  "c03-youtube-search",
  "c03-chrome-history",
  "c03-myactivity-search",
  "c03-myactivity-maps",
];

/** 箱を上限の高さまで埋める材料（読んでいる途中・置き場・止まり・確認待ち）。 */
function crowdedStatus(): ArchivesStatus {
  return {
    sources: ARCHIVE_SOURCES.map((logical_source) => ({
      logical_source,
      last_event_on: "2026-09-12",
      last_archive_created_at: "2026-09-13T04:12:00Z",
    })),
    reading: {
      file_name: "takeout-20260913T041200Z-001.zip",
      inner_path: "Records.json",
      items_read: 410_000,
      started_at: "2026-09-15T02:30:00Z",
    },
    inbox: {
      capturable: false,
      blockers: ["dedicated_inbox_unreadable", "downloads_unreadable"],
      emitted_at: "2026-09-01T01:00:00Z",
    },
    pending_shape: { archives: 2, files: 3 },
    latest_archive: {
      file_name: "takeout-20260913T041200Z-001.zip",
      first_seen_at: "2026-09-13T04:12:00Z",
      outcome: "read",
      unreadable_kind: null,
      inserted: 2,
      duplicate: 1,
      unreadable: 0,
      previously_read_at: null,
    },
  };
}

async function renderWithArchives(): Promise<HTMLElement> {
  const archives = ARCHIVE_SOURCES.map((name) =>
    source(name, name, days("2026-01-04", 371, ["recorded", "alive_no_record"])),
  );
  const body: Record<string, unknown> = {
    coverage: [...fiveSources("2026-01-04", 371), ...archives],
    achievement: achievement({ confirmed: false, days_until_confirmed: 200 }),
    archives: crowdedStatus(),
  };
  vi.stubGlobal("fetch", (path: string) =>
    Promise.resolve({
      ok: true,
      json: () =>
        Promise.resolve(
          path.startsWith("/api/coverage?")
            ? body.coverage
            : path.startsWith("/api/archives")
              ? body.archives
              : body.achievement,
        ),
    } as Response),
  );
  render(<App />);
  await waitFor(() => {
    expect(document.querySelectorAll("section[data-source]")).toHaveLength(5 + archives.length);
  });
  return document.querySelector("main") as HTMLElement;
}

/** 箱の宣言の高さのうち、予算から除いてよい量。 */
function excludedBoxPx(): number {
  const box = document.querySelector('[data-testid="latest-archive"]');
  if (!(box instanceof HTMLElement)) return 0;
  return Math.min(declaredHeight(box), ARCHIVE_BOX_MAX_PX);
}

/** Must の 5 本（退役していないもののうち、`c03-` でない先頭 5 本）。 */
function mustSections(): HTMLElement[] {
  return [...document.querySelectorAll("section[data-source]")].filter(
    (s) => !(s.getAttribute("data-source") ?? "").startsWith("c03-"),
  ) as HTMLElement[];
}

describe("書庫のソースを足したときの予算", () => {
  it("Must の 5 本目の下端が 1,440 px 以内にある", async () => {
    const main = await renderWithArchives();
    expect(mustSections()).toHaveLength(5);
    const bottom = bottomOf(main, mustSections().at(-1) as HTMLElement);
    const budget = ONE_SCROLL_PX + excludedBoxPx();
    expect(
      bottom,
      `書庫のソースを 12 本足すと Must の 5 本目が ${Math.round(bottom)} px（予算 ${budget} px）`,
    ).toBeLessThanOrEqual(budget);
  });

  it("2 本目の直近 4 週の下端が 800 px 以内にある", async () => {
    const main = await renderWithArchives();
    const grids = [...document.querySelectorAll('[data-role="grid"]')] as HTMLElement[];
    const second = grids[1];
    const budget = VIEWPORT_H_PX + excludedBoxPx();
    const bottom = bottomOf(main, second);
    expect(
      bottom,
      `2 本目の直近 4 週の下端が ${Math.round(bottom)} px（予算 ${budget} px）`,
    ).toBeLessThanOrEqual(budget);
  });

  it("箱が上限より高くても、除ける量は 160 px を超えない", async () => {
    await renderWithArchives();
    const box = document.querySelector('[data-testid="latest-archive"]') as HTMLElement;
    // 何らかの誤りで箱が 200 px になっても、予算から除けるのは 160 px まで。
    box.style.paddingTop = `${px(box.style.paddingTop) + 200}px`;
    expect(declaredHeight(box)).toBeGreaterThan(ARCHIVE_BOX_MAX_PX);
    expect(excludedBoxPx()).toBe(ARCHIVE_BOX_MAX_PX);
  });

  it("書庫のソースの格子が 360 px の幅に収まり、週の帯が 24 px 以上ある", async () => {
    await renderWithArchives();
    const archiveGrids = [...document.querySelectorAll("section[data-source]")].filter((s) =>
      (s.getAttribute("data-source") ?? "").startsWith("c03-"),
    ) as HTMLElement[];
    expect(archiveGrids.length).toBeGreaterThan(0);
    for (const section of archiveGrids) {
      // 横スクロールは画面が `overflowX: hidden` で閉じている。格子の側は
      // 幅を固定しない（固定すると 360 px の外へ出る）。
      expect(section.style.width, `${section.getAttribute("data-source")} が幅を固定している`).toBe(
        "",
      );
      const rows = [...section.querySelectorAll("[data-week]")] as HTMLElement[];
      expect(rows.length, "週の帯が 1 本も無い").toBeGreaterThan(0);
      for (const row of rows) {
        expect(
          declaredHeight(row),
          `${section.getAttribute("data-source")} の週の帯が ${declaredHeight(row)} px`,
        ).toBeGreaterThanOrEqual(MIN_TARGET_PX);
      }
    }
  });
});
