// SPDX-License-Identifier: AGPL-3.0-only
// Scenario: 直近に置いた書庫の箱は Must の前にある
// Scenario: 直近に置いた書庫の結果が箱に出る
// Scenario: 書庫が 1 つも置かれていないことが出る
// Scenario: 読んでいる間は件数が箱に出る
// Scenario: 形の確認を待っている書庫が箱に出る
// Scenario: 読めなかった書庫は文字で出る
// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
// Scenario: 既に読んだ書庫を置き直すとそれが箱に出る
// Scenario: 置き場が読めないことが画面に出る
// Scenario: 取り込み器が止まっていることが画面に出る
// Scenario: 箱は 160 px を超えない
// Scenario: 箱が溢れても読めなかった書庫は省かれない
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BOX_MAX_ROWS, LatestArchive } from "../LatestArchive";
import type { ArchivesStatus, LatestArchiveStatus } from "../archives";
import { declaredHeight } from "./layout";

/**
 * **spec が書いている固定の予算**（`external-ingestion` の「箱の高さは 160 CSS px 以下」）。
 *
 * `ARCHIVE_BOX_MAX_PX` を import して突き合わせていたときは、**両側が一緒に動く**ので
 * 定数を 120 に下げても全部緑のままだった（design D27 / review R10）。
 */
const BOX_BUDGET_PX = 160;

const NOW = new Date("2026-09-15T03:00:00Z");

function latest(over: Partial<LatestArchiveStatus> = {}): LatestArchiveStatus {
  return {
    file_name: "takeout-20260913T041200Z-001.zip",
    first_seen_at: "2026-09-13T04:12:00Z",
    outcome: "read",
    unreadable_kind: null,
    inserted: 2,
    duplicate: 1,
    unreadable: 0,
    previously_read_at: null,
    ...over,
  };
}

function box(over: Partial<ArchivesStatus> = {}): string {
  render(<LatestArchive status={{ sources: [], ...over }} now={NOW} />);
  return screen.getByTestId("latest-archive").textContent ?? "";
}

describe("直近に置いた書庫の箱", () => {
  it("読めた書庫の名前・見つけた時刻・件数を文字で出す", () => {
    const text = box({ latest_archive: latest() });
    expect(text).toContain("takeout-20260913T041200Z-001.zip");
    expect(text).toContain("2026-09-13 13:12");
    expect(text).toContain("入った 2 · 既にあった 1 · 読めなかった 0");
  });

  it("1 つも置かれていないことを文字で出す", () => {
    expect(box()).toContain("まだ書庫が置かれていません");
  });

  // **読み出せていないことと、置かれていないことを混ぜない**（review R12）。
  it("読み込み中は「置かれていない」と断言しない", () => {
    render(<LatestArchive status={null} now={NOW} />);
    const text = screen.getByTestId("latest-archive").textContent ?? "";
    expect(text).toContain("読み込み中");
    expect(text).not.toContain("まだ書庫が置かれていません");
  });

  it("読み出しに失敗したら、そのことを出す", () => {
    render(<LatestArchive status={null} now={NOW} failed />);
    const text = screen.getByTestId("latest-archive").textContent ?? "";
    expect(text).toContain("読み出せませんでした");
    expect(text).not.toContain("まだ書庫が置かれていません");
  });

  it("読めなかった書庫を、理由の種別つきで文字で出す", () => {
    const text = box({
      latest_archive: latest({ outcome: "unreadable", unreadable_kind: "broken_zip" }),
    });
    expect(text).toContain("読めなかった書庫です");
    expect(text).toContain("書庫が壊れている");
  });

  it("格納に失敗していることを文字で出す", () => {
    const text = box({ latest_archive: latest({ outcome: "store_failed" }) });
    expect(text).toContain("格納に失敗しています");
  });

  it("既に読んだ書庫と、前に読んだ時刻を文字で出す", () => {
    const text = box({
      latest_archive: latest({
        outcome: "already_read",
        previously_read_at: "2026-09-12T01:00:00Z",
      }),
    });
    expect(text).toContain("既に読んだ書庫です");
    expect(text).toContain("2026-09-12 10:00");
  });

  it("読んでいる途中のファイルと件数と読み始めた時刻を出す", () => {
    const text = box({
      reading: {
        file_name: "takeout-20260913T041200Z-001.zip",
        inner_path: "Records.json",
        items_read: 410_000,
        started_at: "2026-09-15T02:30:00Z",
      },
    });
    expect(text).toContain("読んでいます");
    expect(text).toContain("Records.json");
    expect(text).toContain("410,000 件まで");
    expect(text).toContain("2026-09-15 11:30");
  });

  it("形の確認を待っている書庫の数を出す", () => {
    expect(box({ pending_shape: { archives: 2, files: 3 } })).toContain(
      "形の確認を待っている書庫が 2 冊あります",
    );
  });

  it("置き場が読めないことを、どの置き場かまで出す", () => {
    const text = box({
      inbox: {
        capturable: false,
        blockers: ["dedicated_inbox_unreadable"],
        emitted_at: "2026-09-15T01:00:00Z",
      },
    });
    expect(text).toContain("専用のフォルダ が読めません");
  });

  it("取り込み器の最後の確認が 3 日より前なら、何日前かを出す", () => {
    const text = box({
      inbox: { capturable: true, blockers: [], emitted_at: "2026-09-11T01:00:00Z" },
    });
    expect(text).toContain("取り込み器の最後の確認: 4 日前");
  });

  it("取り込み器が一度も動いていないことを出す", () => {
    expect(box({ latest_archive: latest() })).toContain("取り込み器はまだ一度も動いていません");
  });

  it("最後の確認が 3 日以内なら、止まっているとは出さない", () => {
    const text = box({
      inbox: { capturable: true, blockers: [], emitted_at: "2026-09-13T01:00:00Z" },
    });
    expect(text).not.toContain("取り込み器の最後の確認");
  });
});

describe("箱の高さ", () => {
  /** 箱が溢れる状態（読んでいる途中・置き場が読めない・止まっている・確認待ち）。 */
  const crowded = (over: Partial<ArchivesStatus> = {}): ArchivesStatus => ({
    sources: [],
    reading: {
      file_name: "takeout-20260913T041200Z-001.zip",
      inner_path: "Records.json",
      items_read: 410_000,
      started_at: "2026-09-15T02:30:00Z",
    },
    inbox: {
      capturable: false,
      blockers: ["dedicated_inbox_unreadable", "downloads_unreadable"],
      emitted_at: "2026-09-11T01:00:00Z",
    },
    pending_shape: { archives: 2, files: 3 },
    latest_archive: latest(),
    ...over,
  });

  it("溢れる材料があっても宣言の高さが 160 px を超えない", () => {
    render(<LatestArchive status={crowded()} now={NOW} />);
    const el = screen.getByTestId("latest-archive");
    expect(el.style.maxHeight, "箱が宣言している上限が spec の 160 px と違う").toBe(
      `${BOX_BUDGET_PX}px`,
    );
    expect(declaredHeight(el)).toBeLessThanOrEqual(BOX_BUDGET_PX);
  });

  /**
   * **予算を使い切っていることも見る**（review R10）。上限しか見ていなかったときは、
   * `ARCHIVE_BOX_MAX_PX` を 120 に下げても「行が減るだけ」で全部緑だった ——
   * 本人から見れば出るはずの行が黙って消えているのに、検査が何も言わない。
   */
  it("溢れているとき、あと 1 行足すと 160 px を超えるところまで使っている", () => {
    render(<LatestArchive status={crowded()} now={NOW} />);
    const el = screen.getByTestId("latest-archive");
    // 行の高さは**描かれた行から測る**（定数を読み直さない）。
    const row = screen.getByTestId("archive-row-more");
    const line = declaredHeight(row);
    expect(line).toBeGreaterThan(0);
    const height = declaredHeight(el);
    expect(
      height + line,
      `箱が ${Math.round(height)} px しか使っておらず、あと 1 行（${line} px）入る余地がある`,
    ).toBeGreaterThan(BOX_BUDGET_PX);
  });

  it("出しきれない行があれば、省いたことを出す", () => {
    render(<LatestArchive status={crowded()} now={NOW} />);
    expect(screen.getByTestId("archive-row-more").textContent).toMatch(/ほか \d+ 件/);
  });

  it("箱が溢れても、読めなかった書庫の行は省かれない", () => {
    render(
      <LatestArchive
        status={crowded({
          latest_archive: latest({ outcome: "unreadable", unreadable_kind: "broken_zip" }),
        })}
        now={NOW}
      />,
    );
    const el = screen.getByTestId("latest-archive");
    expect(el.textContent).toContain("読めなかった書庫です");
    expect(declaredHeight(el)).toBeLessThanOrEqual(BOX_BUDGET_PX);
  });

  it("勘定が空振りしていない（行を 1 本増やせば入る行が減る）", () => {
    // BOX_MAX_ROWS の 1 つ手前までは「ほか N 件」が出ない。
    expect(BOX_MAX_ROWS).toBeGreaterThan(1);
    render(<LatestArchive status={{ sources: [], latest_archive: latest() }} now={NOW} />);
    expect(screen.queryByTestId("archive-row-more")).toBeNull();
  });
});
