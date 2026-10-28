// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 毎日の気分（`GET /feelings`。ST17）の型と、主観の原文の組み立て。
 *
 * **原文は画面が組む**（design D1 / D4）—— サーバは受け取ったまま保存するので、
 * 乱数はここで足す。前/後は原文に持たせず、記入の日時と対象の終わりから決める（FR-37）。
 */

import { WEB_VERSION } from "./buildStamp";
import { msg } from "./messages";
import { isRealDate, type DayEntry } from "./stays";

const M = msg("feel");

/** ひとことの上限。NFC にしたコードポイントの数（design D5 / 深掘り C10）。サーバと同じ値。 */
export const NOTE_MAX_CHARS = 2000;
/** 暦の長さ（対象の日で終わる 30 日。design D14（仮）） */
export const CALENDAR_DAYS = 30;

const DAY_MS = 86_400_000;
/** 対象の日は `Asia/Tokyo` で区切る（design D1） */
const TARGET_TZ = "Asia/Tokyo";
const TARGET_OFFSET = "+09:00";

export type Phase = "before" | "after";
export type DayStatus = "recorded" | "missing" | "before_start" | "future";

export interface Scale {
  id: string;
  version: number;
  value: number;
}

export interface Feeling {
  id: string;
  scales: Scale[];
  note: string | null;
  written_at: string;
  ingested_at: string;
  phase: Phase;
  target_stay: string | null;
}

export interface Latest {
  before: Feeling | null;
  after: Feeling | null;
}

/** 紐づけ先。日は `Asia/Tokyo` の暦の日、滞在は `/stays` の区間の値。 */
export type FeelingTarget =
  | { kind: "day"; date: string }
  | { kind: "stay"; stay: string; start: string; end: string };

/** 送る 1 件（原文と、エンベロープの時刻・地域）。 */
export interface BuiltFeeling {
  id: string;
  raw: string;
  /** 出来事の時刻 = 対象の時刻（深掘り Q12） */
  eventTime: string;
  tzOffsetMin: number;
  tzId: string;
}

/** 128 bit を base64url で 22 文字。**識別子から導かない**（design D4）—— `id` は消去の後も残る。 */
function newNonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * ひとことにサーバが断る制御文字（改行とタブを除く Unicode の Cc）があるか。サーバの `char::is_control` と同じ範囲。
 * サーバはこれも長すぎるのと同じ `invalid_note` で断るので、画面が文を選び分けるのに使う（design D22（仮））。
 */
export function hasControlChar(note: string): boolean {
  return [...note].some((ch) => ch !== "\n" && ch !== "\t" && /\p{Cc}/u.test(ch));
}

/** ひとことの長さ。サーバと同じ数え方（NFC にしてからコードポイント。`maxLength` は UTF-16 なので使わない）。 */
export function countNoteChars(note: string): number {
  return [...note.normalize("NFC")].length;
}

const pad = (n: number): string => String(Math.trunc(Math.abs(n))).padStart(2, "0");

/** 地域のずれつきの RFC 3339（端末の地域）。 */
function localRfc3339(at: Date): string {
  const off = -at.getTimezoneOffset();
  const sign = off < 0 ? "-" : "+";
  const local = new Date(at.getTime() + off * 60_000);
  const ymd = `${local.getUTCFullYear()}-${pad(local.getUTCMonth() + 1)}-${pad(local.getUTCDate())}`;
  const hms = `${pad(local.getUTCHours())}:${pad(local.getUTCMinutes())}:${pad(local.getUTCSeconds())}`;
  const ms = String(local.getUTCMilliseconds()).padStart(3, "0");
  return `${ymd}T${hms}.${ms}${sign}${pad(off / 60)}:${pad(off % 60)}`;
}

function eventTimeOf(target: FeelingTarget): string {
  return target.kind === "day" ? `${target.date}T00:00:00${TARGET_OFFSET}` : target.start;
}

/**
 * 原文を組む（design D1 の形）。**書くたびに識別子・乱数・記入の日時が決まる**。
 * 送り直しは組み直さず同じ `BuiltFeeling` を送る（組み直すと乱数が変わって 2 件になる）。
 * `id` は試験が差し込めるよう引数にしてある。
 */
export function buildFeeling(
  target: FeelingTarget,
  scales: Scale[],
  note: string | null,
  now: Date,
  id: string = crypto.randomUUID(),
): BuiltFeeling {
  const raw = JSON.stringify({
    feeling: id,
    nonce: newNonce(),
    target,
    written_at: localRfc3339(now),
    written_tz: Intl.DateTimeFormat().resolvedOptions().timeZone,
    scales,
    note: note === null || note.trim() === "" ? null : note,
  });
  return { id, raw, eventTime: eventTimeOf(target), tzOffsetMin: 540, tzId: TARGET_TZ };
}

/** 対象の終わり（UTC のミリ秒）。日は翌日の 0 時（`Asia/Tokyo`）、滞在は終わり。 */
function endOf(target: FeelingTarget): number {
  return target.kind === "day"
    ? Date.parse(`${target.date}T00:00:00${TARGET_OFFSET}`) + DAY_MS
    : Date.parse(target.end);
}

/** いま書けば付く前/後。`now < 対象の終わり` なら前。 */
export function phaseOf(target: FeelingTarget, now: Date): Phase {
  return now.getTime() < endOf(target) ? "before" : "after";
}

/** 対象の日で終わる `CALENDAR_DAYS` 日（古い順。`YYYY-MM-DD`）。 */
export function calendarDays(last: string): string[] {
  const end = Date.parse(`${last}T00:00:00Z`);
  return Array.from({ length: CALENDAR_DAYS }, (_, i) =>
    new Date(end - (CALENDAR_DAYS - 1 - i) * DAY_MS).toISOString().slice(0, 10),
  );
}

/** 快–不快の値の表示（符号つき。0 は符号なし）。 */
export function signed(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

/** 快–不快の言葉。 */
export function valenceWord(value: number): string {
  return M.valenceWord[String(value)] ?? "";
}

/**
 * 暦の日の印。「記入あり」は快–不快の数値（「日」の記録が無ければ「滞」）、「未記入」は「未」。
 * 「使い始める前」「これから」は印を持たない（日付だけ）。
 */
export function dayMark(status: DayStatus, latestValence: number | null): string {
  if (status === "missing") return M.markMissing;
  if (status === "recorded") return latestValence === null ? M.markStayOnly : signed(latestValence);
  return "";
}

// ---- 読み出し（GET /feelings）と送る 1 件（S-3。design D12） ----

/** 1 日の分（S-3 が使う部分）。`day` は「日」を紐づけ先にする記録。 */
export interface DayBucket {
  latest: Latest;
  /** 記入の日時の新しい順 */
  records: Feeling[];
}

/** 1 つの滞在の分（サーバが吸収を解決済み）。 */
export interface StayBucket {
  stay: string;
  latest: Latest;
  records: Feeling[];
}

export interface FeelingsDay {
  date: string;
  status: DayStatus;
  day: DayBucket;
  stays: StayBucket[];
  /** 記録のある滞在の数（書き直しは 1。消した滞在も数える） */
  stays_with_records: number;
  /** 表示しない滞在（消した・たどれない）の記録の件数 */
  hidden_stay_records: number;
}

export interface FeelingsView {
  today: string;
  days: FeelingsDay[];
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null;

function isFeelingShape(v: unknown): v is Feeling {
  if (!isObj(v)) return false;
  return (
    typeof v.id === "string" &&
    Array.isArray(v.scales) &&
    v.scales.every((s) => isObj(s) && typeof s.id === "string" && typeof s.value === "number") &&
    (v.note === null || typeof v.note === "string") &&
    typeof v.written_at === "string" &&
    (v.phase === "before" || v.phase === "after")
  );
}

const isFeelingOrNull = (v: unknown): boolean => v === null || isFeelingShape(v);

const DAY_STATUSES: readonly string[] = ["recorded", "missing", "before_start", "future"];

const isLatest = (v: unknown): boolean => isObj(v) && isFeelingOrNull(v.before) && isFeelingOrNull(v.after);

function isStayBucket(v: unknown): v is StayBucket {
  return isObj(v) && typeof v.stay === "string" && isLatest(v.latest) && Array.isArray(v.records) && v.records.every(isFeelingShape);
}

/** 応答が読める形か。**形が違えば失敗として出す**（型の宣言だけで通すと描画で落ちる）。 */
export function isFeelingsView(v: unknown): v is FeelingsView {
  if (!isObj(v) || typeof v.today !== "string" || !Array.isArray(v.days)) return false;
  return v.days.every((d) => {
    if (!isObj(d) || typeof d.date !== "string" || !isObj(d.day)) return false;
    if (!DAY_STATUSES.includes(d.status as DayStatus)) return false;
    if (typeof d.hidden_stay_records !== "number" || typeof d.stays_with_records !== "number") return false;
    if (!Array.isArray(d.stays) || !d.stays.every(isStayBucket)) return false;
    const b = d.day;
    return (
      isObj(b.latest) &&
      isFeelingOrNull(b.latest.before) &&
      isFeelingOrNull(b.latest.after) &&
      Array.isArray(b.records) &&
      b.records.every(isFeelingShape)
    );
  });
}

/** 記録の中の尺度の値。無ければ `null`。 */
export function scaleValue(f: Feeling | null, id: string): number | null {
  return f?.scales.find((s) => s.id === id)?.value ?? null;
}

/** 取り込み口へ送る 1 件（`POST /api/ingest` の本文の要素）。 */
export function ingestFeelingItem(built: BuiltFeeling): unknown {
  return {
    id: built.id,
    user_id: "00000000-0000-0000-0000-000000000000",
    logical_source: "s01-feeling",
    external_id: null,
    device_id: null,
    origin: "authored",
    event_time: built.eventTime,
    tz_offset_min: built.tzOffsetMin,
    tz_id: built.tzId,
    schema_version: 1,
    client_version: WEB_VERSION,
    raw: built.raw,
    payload: {},
  };
}

/** 記入の日時の表示（`M 月 D 日 H:MM`。`Asia/Tokyo`）。 */
export function atLabel(at: Date): string {
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: TARGET_TZ,
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(at);
  const get = (t: string): string => parts.find((p) => p.type === t)?.value ?? "";
  return M.at(Number(get("month")), Number(get("day")), `${get("hour")}:${get("minute")}`);
}

/** `#/feel` の日付の部分。`null` = 日付なし（暦の今日）、`"invalid"` = 暦に無い。 */
export function feelDateFromHash(hash: string): { date: string | null } | { invalid: string } | undefined {
  const m = /^#\/feel(?:\/([^/]*))?\/?$/.exec(hash);
  if (m === null) return undefined;
  const raw = m[1] ?? "";
  if (raw === "") return { date: null };
  return isRealDate(raw) ? { date: raw } : { invalid: raw };
}

/** 滞在の行（S-3。design D13）。 */
export interface StayRow {
  id: string;
  /** `/stays` の区間の値（送る原文の `start` / `end`） */
  start: string;
  end: string;
  /** 始まりが対象の日より前（その行で書いた記録は前の日に数える） */
  carried: boolean;
  /** 記入の日時の新しい順 */
  records: Feeling[];
  /** 後の最新があれば後、無ければ前の最新 */
  latest: Feeling | null;
}

const writtenMs = (f: Feeling): number => Date.parse(f.written_at);

/**
 * 対象の日の滞在の行を作る。並びは `/stays` の `kind: "stay"`（消した・吸収された滞在は既に出ない）の始まりの順。
 * 各行の記録は、読んだ日の全部の滞在の分から識別子で集める（前の日から続く滞在の記録は前の日にある）。
 */
export function stayRows(entries: DayEntry[], days: FeelingsDay[], target: string): StayRow[] {
  const dayStart = Date.parse(`${target}T00:00:00${TARGET_OFFSET}`);
  return entries
    .filter((e): e is DayEntry & { id: string } => e.kind === "stay" && e.id !== undefined)
    .sort((a, b) => Date.parse(a.start) - Date.parse(b.start))
    .map((e) => {
      const records = days
        .flatMap((d) => d.stays.filter((s) => s.stay === e.id).flatMap((s) => s.records))
        .sort((a, b) => writtenMs(b) - writtenMs(a));
      const phased = (p: Phase): Feeling | null => records.find((r) => r.phase === p) ?? null;
      return {
        id: e.id,
        start: e.start,
        end: e.end,
        carried: Date.parse(e.start) < dayStart,
        records,
        latest: phased("after") ?? phased("before"),
      };
    });
}

const REMINDER_AT = /^([01]\d|2[0-3]):[0-5]\d$/;

/** 通知の時刻（`GET /feelings/reminder` の使う欄）。 */
export function isReminder(v: unknown): v is { at: string } {
  return isObj(v) && typeof v.at === "string" && REMINDER_AT.test(v.at);
}

/** 時刻の欄の値が `HH:MM` として読めるか（サーバと同じ形）。 */
export const isReminderTime = (s: string): boolean => REMINDER_AT.test(s);

/** 暦の先頭の日の曜日（0 = 日曜）。 */
export function weekdayOf(date: string): number {
  return new Date(`${date}T00:00:00Z`).getUTCDay();
}
