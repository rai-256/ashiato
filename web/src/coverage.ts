// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 稼働状況の型と、状態の読み方。**形は `crates/server/src/coverage.rs` が正典**
 * （契約は `docs/openapi.json`）。
 */
import { BAND, INITIAL_WEEKS, TEXT, YEAR_WEEKS } from "./tokens";

/** ソース × 日 の 8 状態（FR-54）。⑧「退役」は ST03 の差し戻し（R55 / R56）。 */
export type DayState =
  | "recorded"
  | "alive_no_record"
  | "alive_not_capturable"
  | "stopped"
  | "dropped"
  | "outage"
  | "before_start"
  | "retired";

/**
 * 8 状態の名前。**週を選んだときにこの文字が出る**（深掘り 第 5 回 Q20 / Q21）——
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
  retired: "退役",
};

/** 格子のセルが担う 3 段。 */
export type Band = keyof typeof BAND;

/**
 * 8 状態を 3 段へ畳む（design D10）。
 * **サーバ側の `DayState::band` と同じ畳み方**でなければならない。
 */
export function bandOf(state: DayState): Band {
  if (state === "recorded") return "recorded";
  if (state === "alive_no_record") return "alive_no_record";
  return "other";
}

/** その日の中で切った破棄の区間（ST04 / design D9）。時刻は `Asia/Tokyo` の `HH:MM`、日の終わりは `24:00`。 */
export type DroppedRange = {
  from: string;
  to: string;
  count: number;
};

export type DayCell = {
  day: string;
  state: DayState;
  event_count: number;
  attempts: number | null;
  successes: number | null;
  /** その日に属する破棄の件数（ST04）。範囲を持たない破棄は入らない */
  dropped_count: number;
  /** その日に重なる破棄の区間（つないでからその日で切ったもの） */
  dropped_ranges: DroppedRange[];
};

/**
 * 格子に破棄の印を付ける日か（ST04 / 深掘り Q3 / design D10）。
 *
 * **丸ごと覆う破棄の日（破棄された期間）には付けない** —— 段がすでに「それ以外」で、状態の名前が破棄を言っている。
 * **欄を返さないサーバでも落ちない**（`isRetired` と同じ理由。`undefined > 0` は偽）。
 */
export function hasDropMark(cell: DayCell): boolean {
  return (cell.dropped_count ?? 0) > 0 && cell.state !== "dropped";
}

/**
 * 印を描く明るさ（design D10（仮））。**段ごとに変える** —— 1 色だと「それ以外」の段の上で見えない。
 *
 * 「記録あり」と「動いていた・記録なし」の上は最も暗い段、「それ以外」の上は控えめな文字の明るさ。
 * **新しい色は足さない**（`tokens.ts` の既存の値だけを使う）。比は `drop-mark.test.tsx` が値から数える。
 */
export function dropMarkLightness(band: Band): number {
  return band === "other" ? TEXT.muted : BAND.other;
}

/** 週の詳細に添える破棄の文字（design D10）。丸ごとの日は件数だけ、それ以外は区間ごとに「うち N 件を破棄（from〜to）」。 */
export function dropNotes(cell: DayCell): string[] {
  const count = cell.dropped_count ?? 0;
  if (cell.state === "dropped") {
    return count > 0 ? [`— ${count.toLocaleString("ja-JP")} 件`] : [];
  }
  // 件数を持たない区間（時間ごとの件数が無い範囲・前の区間に数えた時間）は件数を添えない（design D19）
  return (cell.dropped_ranges ?? []).map((r) =>
    r.count > 0
      ? `うち ${r.count.toLocaleString("ja-JP")} 件を破棄（${r.from}〜${r.to}）`
      : `破棄（${r.from}〜${r.to}）`,
  );
}

export type SourceCoverage = {
  /** **引き継ぎの鎖の先端**（第 8 回 Q31） */
  logical_source: string;
  /** 定数が名指ししている名前。乗り換えが起きたことが読めるように返る */
  named_source: string;
  display_name: string;
  expected_gap_sec: number;
  /** **引き継ぎの鎖の根の日**（第 8 回 Q31） */
  collection_started_on: string | null;
  /** 退役した日（FR-61）。**この日より後が⑧**。退役していなければ `null` */
  retired_on: string | null;
  days: DayCell[];
};

export type SourceAchievement = {
  /** **実際に数えた名前**（引き継ぎの鎖の先端。第 8 回 Q31） */
  logical_source: string;
  /** 定数が名指ししている名前。鎖をたどっていなければ同じ */
  named_source: string;
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

/**
 * 退役したソースを**後ろへ回す**（ST03 の R63 / 第 8 回 Q30）。
 *
 * ST03 の運用では退役は 1 本きりではなく増える。退役した格子が上に並ぶと
 * **Must の 5 本が 1 画面から押し出される** —— 完了の判定
 * 「開いた直後に 2〜3 ソース、ひとスクロールで 5 ソースすべて」が成り立たなくなる。
 *
 * **並びは安定**（同じ側どうしはサーバが返した順のまま）—— 定数の順が画面の順（design D19）。
 */
export function retiredLast(sources: SourceCoverage[]): SourceCoverage[] {
  return [...sources.filter((s) => !isRetired(s)), ...sources.filter((s) => isRetired(s))];
}

/**
 * 退役しているか。**`!== null` で書かない**（review/code-r2.md の M-1）——
 * 欄を返さないサーバ（古い版・巻き戻し）だと値は `undefined` になり、
 * `undefined !== null` は真なので**5 本すべてが退役と判定されて格子が全部消える**。
 * 画面は `at === "ok"` のままなので、エラーも空の知らせも出ない。
 */
export function isRetired(s: SourceCoverage): boolean {
  return (s.retired_on ?? null) !== null;
}
