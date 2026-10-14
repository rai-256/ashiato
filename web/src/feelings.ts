// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 毎日の気分（`GET /feelings`。ST17）の型と、主観の原文の組み立て。
 *
 * **原文は画面が組む**（design D1 / D4）—— サーバは受け取ったまま保存するので、
 * 乱数はここで足す。前/後は原文に持たせず、記入の日時と対象の終わりから決める（FR-37）。
 */

import { msg } from "./messages";

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
