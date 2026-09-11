// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 稼働状況の型と、状態の読み方。**形は `crates/server/src/coverage.rs` が正典**
 * （契約は `docs/openapi.json`）。
 */
import { BAND, INITIAL_WEEKS, YEAR_WEEKS } from "./tokens";

/** ソース × 日 の 7 状態（FR-54）。 */
export type DayState =
  | "recorded"
  | "alive_no_record"
  | "alive_not_capturable"
  | "stopped"
  | "dropped"
  | "outage"
  | "before_start";

/**
 * 7 状態の名前。**週を選んだときにこの文字が出る**（深掘り 第 5 回 Q20 / Q21）——
 * 格子は 3 段しか担わないので、**区別の担い手はここ**。
 */
export const STATE_NAME: Record<DayState, string> = {
  recorded: "記録あり",
  alive_no_record: "動いていた・記録なし",
  alive_not_capturable: "動いていたが取れない状態だった",
  stopped: "意図的な停止",
  dropped: "破棄された期間",
  outage: "途絶",
  before_start: "導入前",
};

/** 格子のセルが担う 3 段。 */
export type Band = keyof typeof BAND;

/**
 * 7 状態を 3 段へ畳む（design D10）。
 * **サーバ側の `DayState::band` と同じ畳み方**でなければならない。
 */
export function bandOf(state: DayState): Band {
  if (state === "recorded") return "recorded";
  if (state === "alive_no_record") return "alive_no_record";
  return "other";
}

export type DayCell = {
  day: string;
  state: DayState;
  event_count: number;
  attempts: number | null;
  successes: number | null;
};

export type SourceCoverage = {
  logical_source: string;
  display_name: string;
  expected_gap_sec: number;
  collection_started_on: string | null;
  days: DayCell[];
};

export type SourceAchievement = {
  logical_source: string;
  display_name: string;
  subject: "device" | "usage";
  collection_started_on: string | null;
  achieved_days: number;
  denominator: number;
  threshold: number;
  met: boolean;
  window_closed: boolean;
  window_closes_on: string | null;
};

export type Achievement = {
  sources: SourceAchievement[];
  verdict: boolean;
  failing: string[];
  confirmed: boolean;
  confirms_on: string | null;
  days_until_confirmed: number | null;
  not_started: string[];
};

/** 1 週ぶん。**日曜から土曜の 7 日**で、足りない側は `null`（格子の形を崩さない）。 */
export type Week = {
  /** その週の始まりの日（`YYYY-MM-DD`）。選んだ週を見分ける鍵にもなる */
  start: string;
  days: (DayCell | null)[];
};

/**
 * 日の並びを**週に畳む**（FR-54 / 深掘り Q6）。
 *
 * **新しい週が先頭に来る**（第 6 回 Q25）—— 開いた直後に出る直近 4〜5 週が
 * 新しい側になるようにするため。
 *
 * 週の始まりは**日曜**。`days` は日付の昇順で渡す。
 */
export function foldIntoWeeks(days: DayCell[]): Week[] {
  const byWeek = new Map<string, (DayCell | null)[]>();
  for (const cell of days) {
    const date = new Date(`${cell.day}T00:00:00Z`);
    const weekday = date.getUTCDay();
    const start = new Date(date);
    start.setUTCDate(start.getUTCDate() - weekday);
    const key = start.toISOString().slice(0, 10);
    const slot = byWeek.get(key) ?? new Array<DayCell | null>(7).fill(null);
    slot[weekday] = cell;
    byWeek.set(key, slot);
  }
  return [...byWeek.entries()]
    .map(([start, week]) => ({ start, days: week }))
    // **新しい週が上**（第 6 回 Q25）
    .sort((a, b) => (a.start < b.start ? 1 : -1));
}

/**
 * 開いた直後に見せる週数（第 7 回 Q28）。
 *
 * **1 年ぶんを最初から出さない** —— 縦長の格子は 1 ソースで 53 行 × 24 px ≒ 1,300 px あり、
 * 2 本目以降のソースの直近週がそれだけ下に行く。完了の判定
 * 「1 か月放置した後に開くと欠けた日が一目で分かる」が 1 本目にしか成立しなくなる。
 */
export function visibleWeeks(weeks: Week[], expanded: boolean): Week[] {
  return weeks.slice(0, expanded ? YEAR_WEEKS : INITIAL_WEEKS);
}
