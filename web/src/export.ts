// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 全記録の書き出し（`GET /export` `GET /export/summary` `POST /export`。ST33 / design D13）の型と読み手。
 *
 * **形が違えば失敗として出す** —— 型の宣言だけで通すと描画で落ちて画面が白くなる（`stays.ts` と同じ形）。
 * **合言葉は `POST` の本体にだけ載せる。** URL・保存領域には載せない。
 */

import { DAY_TZ } from "./tokens";

/** 合言葉の下限（符号位置。口の `chars().count()` と同じ。design D13 / C23）。 */
export const MIN_PASSWORD_CHARS = 12;

export interface SummarySource {
  logical_source: string;
  display_name: string;
  count: number;
  first: string;
  last: string;
}

export interface Estimate {
  rows: number;
  bytes: number;
  seconds: number;
}

export interface Summary {
  sources: SummarySource[];
  estimate: Estimate;
}

export interface RunningView {
  export_id: string;
  started_at: string;
  written: number;
  total: number;
}

export interface LastView {
  finished_at: string;
  rows: number;
  bytes: number;
  file: string;
  /** 後に消した記録の件数。`null` は数えられない（台帳の書いた記録が読めない形。deep Q12）。 */
  erased_after: number | null;
}

export interface FailureView {
  at: string;
  kind: string;
}

export interface InterruptedView {
  started_at: string;
  file?: string | null;
}

export interface ExportStatus {
  running?: RunningView | null;
  last?: LastView | null;
  last_failure?: FailureView | null;
  interrupted?: InterruptedView | null;
  leftover: string[];
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null;
const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isStr = (v: unknown): v is string => typeof v === "string";
/** 無い・`null` のどちらも「無い」。 */
const orNone = (v: unknown, ok: (x: unknown) => boolean): boolean => v === undefined || v === null || ok(v);

export function isSummary(v: unknown): v is Summary {
  if (!isObj(v) || !Array.isArray(v.sources) || !isObj(v.estimate)) return false;
  const e = v.estimate;
  return (
    isNum(e.rows) &&
    isNum(e.bytes) &&
    isNum(e.seconds) &&
    v.sources.every(
      (s) =>
        isObj(s) && isStr(s.logical_source) && isStr(s.display_name) && isNum(s.count) && isStr(s.first) && isStr(s.last),
    )
  );
}

export function isExportStatus(v: unknown): v is ExportStatus {
  if (!isObj(v) || !Array.isArray(v.leftover) || !v.leftover.every(isStr)) return false;
  return (
    orNone(v.running, (x) => isObj(x) && isStr(x.export_id) && isStr(x.started_at) && isNum(x.written) && isNum(x.total)) &&
    orNone(
      v.last,
      (x) => isObj(x) && isStr(x.finished_at) && isNum(x.rows) && isNum(x.bytes) && isStr(x.file) && (isNum(x.erased_after) || x.erased_after === null),
    ) &&
    orNone(v.last_failure, (x) => isObj(x) && isStr(x.at) && isStr(x.kind)) &&
    orNone(v.interrupted, (x) => isObj(x) && isStr(x.started_at) && orNone(x.file, isStr))
  );
}

/** 符号位置の数（UTF-16 の `length` を使わない。絵文字 1 つは 1 文字）。 */
export function passwordChars(value: string): number {
  return Array.from(value).length;
}

/** `POST /export` の結果。合言葉は戻り値にも載せない。 */
export type StartOutcome =
  | { at: "started" }
  | { at: "refused"; status: number; error: string }
  | { at: "unreachable" };

/** 合言葉を本体にだけ載せて書き出しを始める。 */
export async function startExport(password: string): Promise<StartOutcome> {
  try {
    const res = await fetch("/api/export", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ password }),
    });
    if (res.ok) return { at: "started" };
    let error = "";
    try {
      const body: unknown = await res.json();
      if (isObj(body) && isStr(body.error)) error = body.error;
    } catch {
      // 本体が読めなくても、状態の番号で種類は分かる
    }
    return { at: "refused", status: res.status, error };
  } catch {
    return { at: "unreachable" };
  }
}

/** `2026/10/13 21:05`（表示の時刻帯で書く）。 */
export function momentLabel(iso: string): string {
  const p = new Intl.DateTimeFormat("en-CA", {
    timeZone: DAY_TZ,
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(iso));
  const get = (t: string): string => p.find((x) => x.type === t)?.value ?? "";
  return `${get("year")}/${get("month")}/${get("day")} ${get("hour")}:${get("minute")}`;
}

/** `2026/10/13`（期間の端。日付だけ）。 */
export function dayLabel(iso: string): string {
  return momentLabel(iso).slice(0, momentLabel(iso).indexOf(" "));
}

/** 大きさを MB / GB / KB で書く（1000 刻み）。 */
export function sizeLabel(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(bytes >= 1e7 ? 0 : 1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}
