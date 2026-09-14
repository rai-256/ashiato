// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の並び（`GET /stays`。ST16 / design D8）の型と、表示の書き方。
 *
 * **並び（滞在・移動・記録なし）はサーバが組む** —— 「記録なし」を出すには位置の記録の間隔を見る必要があり、
 * 画面に位置を 1,440 件送らない。画面は受け取った順に描くだけ。
 */
import { DAY_TZ } from "./tokens";

export type EntryKind = "stay" | "move" | "no-record";

export interface DayEntry {
  kind: EntryKind;
  start: string;
  end: string;
  id?: string;
  criteria_id?: number;
}

export interface CriteriaTag {
  criteria_id: number;
  radius_m: number;
  min_minutes: number;
}

export interface DayView {
  date: string;
  /** 新しい版から順。先頭が一覧の上に出す基準 */
  criteria: CriteriaTag[];
  entries: DayEntry[];
}

/** `#/day/YYYY-MM-DD` から日付を読む。日付を含まない `#/day` は `null`（今日）。一覧のアドレスでなければ `undefined`。 */
export function dayFromHash(hash: string): string | null | undefined {
  const m = /^#\/day(?:\/(\d{4}-\d{2}-\d{2})?)?\/?$/.exec(hash);
  if (m === null) return undefined;
  return m[1] ?? null;
}

/** 日付を `days` 日ずらす（UTC 正午を足場にする。夏時間もうるう秒も跨がない）。 */
export function shiftDay(date: string, days: number): string {
  const d = new Date(`${date}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

/** その時刻の `Asia/Tokyo` の日付（`YYYY-MM-DD`）。 */
function dateInTz(at: Date): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: DAY_TZ, year: "numeric", month: "2-digit", day: "2-digit" }).format(at);
}

/**
 * 時刻を `H:MM` で書く。**見ている日の外の時刻には日付を添える**（日をまたぐ滞在を実際の時刻で出す）。
 * 見ている日の翌日 0:00 ちょうどは `24:00` と書く（「記録なし 0:00 – 24:00」）。
 */
export function clock(iso: string, viewing: string): string {
  const at = new Date(iso);
  const hm = new Intl.DateTimeFormat("en-GB", { timeZone: DAY_TZ, hour: "numeric", minute: "2-digit", hourCycle: "h23" })
    .format(at)
    .replace(/^0(\d):/, "$1:");
  const day = dateInTz(at);
  if (day === viewing) return hm;
  if (day === shiftDay(viewing, 1) && hm === "0:00") return "24:00";
  const [, mo, d] = day.split("-");
  return `${Number(mo)}/${Number(d)} ${hm}`;
}

/** 長さを `3 時間 34 分` / `42 分` で書く。 */
export function duration(startIso: string, endIso: string): string {
  const minutes = Math.round((Date.parse(endIso) - Date.parse(startIso)) / 60_000);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return h === 0 ? `${m} 分` : `${h} 時間 ${m} 分`;
}

/** 基準を `半径 100 m / 10 分` で書く。 */
export function criteriaLabel(c: CriteriaTag): string {
  return `半径 ${c.radius_m} m / ${c.min_minutes} 分`;
}

/** `2026 年 9 月 12 日` */
export function dateLabel(date: string): string {
  const [y, mo, d] = date.split("-").map(Number);
  return `${y} 年 ${mo} 月 ${d} 日`;
}
