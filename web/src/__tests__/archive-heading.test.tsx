// Scenario: 書庫のソースの見出しに最終日と何日前が出る
// Scenario: 何日前は日本時間の今日から数える
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { archiveLastEventLabel } from "../archives";
import { archiveAnnotation } from "../App";
import { CoverageGrid } from "../CoverageGrid";
import { STATE_NAME } from "../coverage";
import { days } from "./fixtures";

describe("archiveLastEventLabel", () => {
  it("日本時間の今日から最終日までの日数を出す", () => {
    expect(archiveLastEventLabel("2026-09-12", new Date("2026-09-15T00:00:00Z"))).toBe("2026-09-12 まで（3 日前）");
  });

  it("UTCの日付ではなく日本時間の日付で数える", () => {
    expect(archiveLastEventLabel("2026-09-12", new Date("2026-09-14T16:00:00Z"))).toBe("2026-09-12 まで（3 日前）");
  });

  it("書庫ソースの見出しに最終日を添える", () => {
    render(<CoverageGrid annotation="2026-09-12 まで（3 日前）" source={{ logical_source: "c03-youtube-watch", named_source: "c03-youtube-watch", display_name: "YouTube の視聴履歴", expected_gap_sec: 1, collection_started_on: null, retired_on: null, days: [] }} />);
    expect(screen.getByTestId("archive-note-c03-youtube-watch").textContent).toContain("2026-09-12 まで（3 日前）");
  });
});

// Scenario: まだ無いソースはまだ無いと出る
// Scenario: 書庫のソースの格子は開いた直後から直近 4 週を出す
// Scenario: 記録の無い日も同じ判定で出る
describe("書庫のソースの見出しと格子", () => {
  /** 稼働状況の読み出しが済んだ状態を作る。 */
  const loaded = (
    sources: { logical_source: string; last_event_on: string | null }[],
  ): Parameters<typeof archiveAnnotation>[1] => ({
    at: "ok",
    value: {
      sources: sources.map((s) => ({ ...s, last_archive_created_at: null })),
    },
  });
  const NOW = new Date("2026-09-15T00:00:00Z");

  it("1 件も入っていない書庫のソースに「まだ無い」と出す", () => {
    expect(
      archiveAnnotation("c03-chrome-history", loaded([{ logical_source: "c03-chrome-history", last_event_on: null }]), NOW),
    ).toBe("まだ無い");
  });

  it("Must の 5 本には注記を渡さない", () => {
    expect(
      archiveAnnotation("c01-location", loaded([{ logical_source: "c01-location", last_event_on: null }]), NOW),
    ).toBeUndefined();
  });

  it("読み出しが済んでいないうちは注記を出さない（「まだ無い」と混ぜない）", () => {
    expect(archiveAnnotation("c03-chrome-history", { at: "loading" }, NOW)).toBeUndefined();
    expect(archiveAnnotation("c03-chrome-history", { at: "failed", why: "status_500" }, NOW)).toBeUndefined();
  });

  it("押さなくても直近 4 週の行が出ている", () => {
    render(
      <CoverageGrid
        annotation="2026-09-12 まで（3 日前）"
        source={{
          logical_source: "c03-youtube-watch",
          named_source: "c03-youtube-watch",
          display_name: "YouTube の視聴履歴",
          expected_gap_sec: 5_184_000,
          collection_started_on: "2026-01-04",
          retired_on: null,
          days: days("2026-08-02", 49, ["recorded", "alive_no_record"]),
        }}
      />,
    );
    const grid = screen.getByTestId("grid-c03-youtube-watch");
    expect(Number(grid.getAttribute("data-weeks"))).toBeGreaterThanOrEqual(4);
  });

  it("記録の無い日を、Must の 5 本と同じ「動いていた・記録なし」の文字で出す", () => {
    render(
      <CoverageGrid
        annotation="2026-09-12 まで（3 日前）"
        source={{
          logical_source: "c03-youtube-watch",
          named_source: "c03-youtube-watch",
          display_name: "YouTube の視聴履歴",
          expected_gap_sec: 5_184_000,
          collection_started_on: "2026-01-04",
          retired_on: null,
          days: days("2026-09-07", 7, ["alive_no_record"]),
        }}
      />,
    );
    fireEvent.click(screen.getAllByRole("button", { name: /の週$/ })[0]);
    expect(screen.getByTestId("grid-c03-youtube-watch").parentElement?.textContent).toContain(
      STATE_NAME.alive_no_record,
    );
  });
});
