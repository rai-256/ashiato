// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の並び（`GET /stays`。ST16 / design D8）の型と、表示の書き方。
 *
 * **並び（滞在・移動・記録なし）はサーバが組む** —— 「記録なし」を出すには位置の記録の間隔を見る必要があり、
 * 画面に位置を 1,440 件送らない。画面は受け取った順に描くだけ。
 */
import { msg } from "./messages";
import { DAY_TZ } from "./tokens";
import { isGaps, isReasons, isSummary, type Gap, type Reason, type Summary } from "./timeline";

export type EntryKind = "stay" | "move" | "no-record" | "erased" | "other";

/** 本文ごと消した区間にすっぽり入る消した滞在（CH08 / design D6）。別の区間にはならず親の区間の下に出す。 */
export interface NestedEntry {
  start: string;
  end: string;
  stay_ids: string[];
  /** 本文ごと消した滞在か（戻す口が無い） */
  purged: boolean;
}

export interface DayEntry {
  kind: EntryKind;
  start: string;
  end: string;
  id?: string;
  stay_ids?: string[];
  criteria_id?: number;
  /** 本文ごと消した区間（「消した」の行だけ。戻すための `stay_ids` は持たない。ST23 / D12） */
  purged?: boolean;
  /** 本文ごと消すときに指定した滞在（本文ごと消した区間だけ） */
  purge_stay_id?: string;
  /** 後から届いて本文を消した件数（本文ごと消した区間だけ） */
  late_purged?: number;
  /** 本文ごと消した区間にすっぽり入る消した滞在（本文ごと消した区間だけ。無ければ入れ子なし） */
  nested?: NestedEntry[];
  /** 消した行には無い（D7） */
  summary?: Summary;
  /** 記録なしの行にだけ。無ければ事情なし（D11） */
  reasons?: Reason[];
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
  /** 取れていない時間（D9）。置く位置は画面が決める */
  gaps?: Gap[];
  /** ブラウザ履歴が最後に届いた時刻。「まだ届いていない」を出すべきときだけある（D10） */
  browser_through?: string;
}

/** `#/day/YYYY-MM-DD` から日付を読む。日付を含まない `#/day` は `null`（今日）。一覧のアドレスでなければ `undefined`。 */
export function dayFromHash(hash: string): string | null | undefined {
  const m = /^#\/day(?:\/(\d{4}-\d{2}-\d{2})?)?\/?$/.exec(hash);
  if (m === null) return undefined;
  return m[1] ?? null;
}

/** 暦にある日付か（`2026-13-45` を弾く）。 */
export function isRealDate(date: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return false;
  const d = new Date(`${date}T12:00:00Z`);
  return !Number.isNaN(d.getTime()) && d.toISOString().slice(0, 10) === date;
}

/** 応答が 1 日の並びの形をしているか（R46）。**形が違えば失敗として出す** —— 型の宣言だけで通すと描画で落ちて画面が白くなる。 */
export function isDayView(v: unknown): v is DayView {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  const kinds = ["stay", "move", "no-record", "erased", "other"];
  return (
    typeof o.date === "string" &&
    Array.isArray(o.criteria) &&
    o.criteria.every((c) => typeof c === "object" && c !== null && typeof (c as CriteriaTag).criteria_id === "number") &&
    Array.isArray(o.entries) &&
    o.entries.every((e) => {
      const x = e as DayEntry;
      return (
        typeof e === "object" &&
        e !== null &&
        kinds.includes(x.kind) &&
        typeof x.start === "string" &&
        typeof x.end === "string" &&
        (x.purged === undefined || typeof x.purged === "boolean") &&
        (x.purge_stay_id === undefined || typeof x.purge_stay_id === "string") &&
        (x.late_purged === undefined || typeof x.late_purged === "number") &&
        (x.nested === undefined || isNested(x.nested)) &&
        (x.stay_ids === undefined || (Array.isArray(x.stay_ids) && x.stay_ids.every((id) => typeof id === "string"))) &&
        (x.summary === undefined || isSummary(x.summary)) &&
        (x.reasons === undefined || isReasons(x.reasons))
      );
    }) &&
    (o.gaps === undefined || isGaps(o.gaps)) &&
    (o.browser_through === undefined || typeof o.browser_through === "string")
  );
}

function isNested(v: unknown): v is NestedEntry[] {
  return (
    Array.isArray(v) &&
    v.every((n) => {
      const x = n as NestedEntry;
      return (
        typeof n === "object" &&
        n !== null &&
        typeof x.start === "string" &&
        typeof x.end === "string" &&
        typeof x.purged === "boolean" &&
        Array.isArray(x.stay_ids) &&
        x.stay_ids.every((id) => typeof id === "string")
      );
    })
  );
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
  return h === 0 ? msg("day").minutes(m) : msg("day").hoursMinutes(h, m);
}

/** 基準を `半径 100 m / 10 分` で書く。 */
export function criteriaLabel(c: CriteriaTag): string {
  return msg("day").criteriaLabel(c.radius_m, c.min_minutes);
}

/** `2026 年 9 月 12 日` */
export function dateLabel(date: string): string {
  const [y, mo, d] = date.split("-").map(Number);
  return msg("day").dateLabel(y, mo, d);
}

/** 本文ごと消すときに消える件数（`GET /stays/purge/preview` の `counts`。ST23 / D1）。 */
export interface PurgeCounts {
  stays: number;
  locations: number;
  archive_locations: number;
  archive_items: number;
  versions: number;
}

/** 本文ごと消す前の確認の口の応答。 */
export interface PurgePreview {
  counts: PurgeCounts;
  range: { start: string; end: string };
  /** 残している書庫の写しの数 */
  copies: number;
  /** 消えずに残るものの名前（文面は `messages` が引く） */
  remains: string[];
  purged: boolean;
}

const COUNT_KEYS = ["stays", "locations", "archive_locations", "archive_items", "versions"] as const;

export function isPurgeCounts(v: unknown): v is PurgeCounts {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return COUNT_KEYS.every((k) => typeof o[k] === "number");
}

/** 応答が確認の口の形をしているか。形が違えば失敗として出す（`isDayView` と同じ理由）。 */
export function isPurgePreview(v: unknown): v is PurgePreview {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  const range = o.range as Record<string, unknown> | null;
  return (
    isPurgeCounts(o.counts) &&
    typeof range === "object" &&
    range !== null &&
    typeof range.start === "string" &&
    typeof range.end === "string" &&
    typeof o.copies === "number" &&
    Array.isArray(o.remains) &&
    o.remains.every((r) => typeof r === "string") &&
    typeof o.purged === "boolean"
  );
}
