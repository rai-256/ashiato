// Scenario: 書庫のソースの見出しに最終日と何日前が出る
// Scenario: 何日前は日本時間の今日から数える
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { archiveLastEventLabel } from "../archives";
import { CoverageGrid } from "../CoverageGrid";

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
