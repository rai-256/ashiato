// 稼働状況の格子の見出しに添える取り方と間隔の文言（ST13 / design D10・D11）
import { describe, expect, it } from "vitest";
import { intervalLabel, sourceNote, type SourceRow, type SourcesResponse } from "../sources";

const row = (over: Partial<SourceRow>): SourceRow => ({
  logical_source: "c01-location",
  display_name: "携帯端末の位置",
  acquisition: "device",
  fetch_trigger: "interval",
  fetch_interval_sec: 60,
  retired_on: null,
  alert_after_days: null,
  last_archive_on: null,
  alert_days_left: null,
  alert_passed: false,
  no_archive: false,
  ...over,
});
const resp = (rows: SourceRow[], notice: string | null = null): SourcesResponse => ({
  sources: rows,
  device_notice: notice === null ? null : { unused_app_restrictions: notice },
});

describe("間隔の書き方（D11）", () => {
  it.each([
    [59, "59 秒ごと"],
    [60, "60 秒ごと"],
    [90, "90 秒ごと"],
    [120, "2 分ごと"],
    [1800, "30 分ごと"],
    [3600, "1 時間ごと"],
    [86400, "24 時間ごと"],
  ])("%i 秒は「%s」", (sec, text) => {
    expect(intervalLabel(sec)).toBe(text);
  });
});

describe("見出しに添える文字", () => {
  it("間隔ごとのソースは間隔", () => {
    expect(sourceNote("c01-location", resp([row({})]))).toBe(" — 60 秒ごと");
  });

  it("契機の文言", () => {
    const rows = [
      row({ logical_source: "a", fetch_trigger: "on_change", fetch_interval_sec: null }),
      row({ logical_source: "b", fetch_trigger: "once", fetch_interval_sec: null }),
      row({ logical_source: "c", fetch_trigger: "authored", fetch_interval_sec: null }),
      row({ logical_source: "d", fetch_trigger: "derived", fetch_interval_sec: null }),
    ];
    const r = resp(rows);
    expect(["a", "b", "c", "d"].map((n) => sourceNote(n, r))).toEqual([
      " — 変わったとき",
      " — 始めたときに 1 回",
      " — 本人が書いたとき",
      " — 位置から導く",
    ]);
  });

  const archive = (over: Partial<SourceRow>): SourceRow =>
    row({ logical_source: "c03-youtube-watch", acquisition: "archive", fetch_trigger: "manual", fetch_interval_sec: null, alert_after_days: 75, ...over });

  it("書庫は手作業と起点つきの残り日数", () => {
    const r = resp([archive({ last_archive_on: "2026-08-28", alert_days_left: 22 })]);
    expect(sourceNote("c03-youtube-watch", r)).toBe(" — 手作業・2 か月ごと · 通知まで あと 22 日（最後の書庫 2026-08-28 から）");
  });

  it("通知の日を過ぎたら過ぎたと出し、負の日数は出さない", () => {
    const r = resp([archive({ last_archive_on: "2026-08-28", alert_passed: true })]);
    const note = sourceNote("c03-youtube-watch", r)!;
    expect(note).toBe(" — 手作業・2 か月ごと · 通知の日を過ぎた（最後の書庫 2026-08-28 から）");
    expect(note).not.toMatch(/あと/);
  });

  it("書庫も記録も無ければまだ書庫が無い", () => {
    const r = resp([archive({ no_archive: true })]);
    expect(sourceNote("c03-youtube-watch", r)).toBe(" — 手作業・2 か月ごと · まだ書庫が無い");
  });

  it("退役した書庫のソースには何も添えない", () => {
    expect(sourceNote("c03-youtube-watch", resp([archive({ retired_on: "2026-01-01" })]))).toBeUndefined();
  });

  it("休止の対象から外れていなければ、間隔の代わりに短い知らせ", () => {
    expect(sourceNote("c01-location", resp([row({})], "not_exempt"))).toBe(" — 休止の対象（アプリで外す）");
    expect(sourceNote("c01-location", resp([row({})], "exempt"))).toBe(" — 60 秒ごと");
  });

  it("読めない・行が無いソースには添えない", () => {
    expect(sourceNote("c01-location", null)).toBeUndefined();
    expect(sourceNote("c01-location", resp([]))).toBeUndefined();
  });
});
