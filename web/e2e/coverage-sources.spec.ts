// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";

/**
 * 稼働状況の「直近に置いた書庫」に、読み出せなかったファイルが形の問題と別の文字で出ること（ST13 / design D8）。
 *
 * 状態は本物のサーバの `/api/archives/status` を受けてから `latest_archive` だけ、seed が台帳に置く
 * `file_unreadable` の行（`tools/seed.d/202610211200_st13-sources.sh`）と同じ値に差し替える
 * —— 直近の書庫は finished_at の新しい順で、seed の読めた書庫（latest-archive.spec が見る）を奪えないため。
 */

// Scenario: 読み出せなかったファイルは稼働状況に形の問題と別の文字で出る
test("読み出せなかったファイルは「読み出せません」と出て、形の対応の文言は出ない", async ({ page }) => {
  await page.route("**/api/archives/status**", async (route) => {
    const res = await route.fetch();
    const base = (await res.json()) as Record<string, unknown>;
    await route.fulfill({
      response: res,
      json: {
        ...base,
        latest_archive: {
          file_name: "Timeline.json",
          first_seen_at: "2026-09-01T00:00:00Z",
          outcome: "unreadable",
          unreadable_kind: "file_unreadable",
          inserted: 0,
          duplicate: 0,
          unreadable: 0,
          previously_read_at: null,
        },
      },
    });
  });
  await page.goto("/#/coverage");
  const latest = page.getByTestId("latest-archive").getByTestId("archive-row-latest");
  await expect(latest).toContainText("Timeline.json", { timeout: 15_000 });
  await expect(latest).toContainText("読み出せません");
  await expect(latest).toContainText("権限を確かめてください");
  await expect(latest).not.toContainText("扱えない形式");
  await expect(latest).not.toContainText("形が");
});
