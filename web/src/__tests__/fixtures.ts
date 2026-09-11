// SPDX-License-Identifier: AGPL-3.0-only
/** 検査が使う偽の稼働状況。**形は `crates/server/src/coverage.rs` が正典。** */
import type { Achievement, DayCell, DayState, SourceCoverage } from "../coverage";

/** `from` から `n` 日ぶん、状態を巡回させて作る。 */
export function days(from: string, n: number, states: DayState[]): DayCell[] {
  const out: DayCell[] = [];
  const d = new Date(`${from}T00:00:00Z`);
  for (let i = 0; i < n; i++) {
    out.push({
      day: d.toISOString().slice(0, 10),
      state: states[i % states.length],
      event_count: states[i % states.length] === "recorded" ? 3 : 0,
      attempts: null,
      successes: null,
    });
    d.setUTCDate(d.getUTCDate() + 1);
  }
  return out;
}

export function source(
  logical_source: string,
  display_name: string,
  cells: DayCell[],
  retired_on: string | null = null,
  named_source: string = logical_source,
): SourceCoverage {
  return {
    logical_source,
    named_source,
    display_name,
    expected_gap_sec: 21600,
    collection_started_on: cells[0]?.day ?? null,
    retired_on,
    days: cells,
  };
}

/** NFR-13 の 5 ソース。**この順が画面の縦の並び。** */
export const FIVE: [string, string][] = [
  ["c01-location", "携帯端末の位置"],
  ["c01-app-usage", "携帯端末のアプリ利用"],
  ["c01-photo", "端末に保存された写真"],
  ["c02-window", "PC のウィンドウ"],
  ["c02-browser-history", "PC のブラウザ履歴"],
];

export function fiveSources(from: string, n: number): SourceCoverage[] {
  return FIVE.map(([id, name]) =>
    source(id, name, days(from, n, ["recorded", "alive_no_record", "outage"])),
  );
}

export function achievement(over: Partial<Achievement> = {}): Achievement {
  return {
    sources: FIVE.map(([id, name], i) => ({
      logical_source: id,
      named_source: id,
      display_name: name,
      subject: i < 2 ? "device" : "usage",
      collection_started_on: "2026-01-01",
      achieved_days: [360, 355, 352, 351, 340][i],
      denominator: 365,
      threshold: 346.75,
      met: [360, 355, 352, 351, 340][i] >= 346.75,
      window_closed: true,
      window_closes_on: "2027-01-01",
    })),
    verdict: false,
    failing: ["c02-browser-history"],
    confirmed: true,
    confirms_on: "2027-01-01",
    days_until_confirmed: 0,
    not_started: [],
    ...over,
  };
}
