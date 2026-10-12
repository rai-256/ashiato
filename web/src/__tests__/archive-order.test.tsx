// Scenario: 書庫のソースは Must の後ろで退役の前に並ぶ
import { describe, expect, it } from "vitest";
import { orderCoverageWithArchives } from "../archives";
import type { SourceCoverage } from "../coverage";

const source = (logical_source: string, retired_on: string | null = null): SourceCoverage => ({
  logical_source, named_source: logical_source, display_name: logical_source, expected_gap_sec: 1,
  collection_started_on: null, retired_on, days: [],
});

describe("orderCoverageWithArchives", () => {
  it("Must、書庫、退役の順にする", () => {
    expect(orderCoverageWithArchives([
      source("c03-youtube-watch"), source("c01-photo"), source("old", "2026-01-01"), source("c01-location"),
    ]).map((item) => item.logical_source)).toEqual(["c01-photo", "c01-location", "c03-youtube-watch", "old"]);
  });
});
