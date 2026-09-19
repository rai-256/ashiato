// SPDX-License-Identifier: AGPL-3.0-only
import { DAY_TZ } from "./tokens";
import { isRetired, type SourceCoverage } from "./coverage";

export type ArchiveSourceStatus = {
  logical_source: string;
  last_event_on: string | null;
  last_archive_created_at: string | null;
};

/** 直近に置いた書庫 1 件（`GET /archives/status` の `latest_archive`）。 */
export type LatestArchiveStatus = {
  file_name: string | null;
  first_seen_at: string;
  outcome: "read" | "already_read" | "unreadable" | "pending_shape" | "store_failed";
  unreadable_kind: string | null;
  inserted: number;
  duplicate: number;
  unreadable: number;
  previously_read_at: string | null;
};

/** 読み手のメモリの状態。**台帳からは出ない**（台帳は読み終えてから書く）。 */
export type ReadingStatus = {
  file_name: string;
  inner_path: string;
  items_read: number;
  started_at: string;
};

/** 取り込み器そのものの直近の生存信号。 */
export type InboxStatus = { capturable: boolean; blockers: string[]; emitted_at: string };

export type ArchivesStatus = {
  sources: ArchiveSourceStatus[];
  latest_archive?: LatestArchiveStatus | null;
  reading?: ReadingStatus | null;
  pending_shape?: { archives: number; files: number } | null;
  inbox?: InboxStatus | null;
};

/**
 * 取り込み器が止まっていると見なすまでの日数（design D12 / C22。**仮**）。
 *
 * 想定間隔 1 日の 3 倍。反転条件: PC を数日切る運用で、本人が紛らわしいと感じたとき。
 */
export const INBOX_STALE_DAYS = 3;

/** 読めなかった理由の種別を、本人の言葉にする（D7 の 4 つ）。 */
export function unreadableKindLabel(kind: string | null): string {
  const names: Record<string, string> = {
    unsupported_format: "扱えない形式",
    broken_zip: "書庫が壊れている",
    html_only: "HTML しか入っていない",
    no_known_content: "読める中身が 1 つも無い",
  };
  return kind === null ? "理由は不明" : (names[kind] ?? kind);
}

/** `Asia/Tokyo` の「YYYY-MM-DD HH:MM」。時刻は画面でも日本時間で揃える。 */
export function jstMoment(at: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: DAY_TZ,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(new Date(at));
  const get = (type: string): string => parts.find((p) => p.type === type)?.value ?? "";
  return `${get("year")}-${get("month")}-${get("day")} ${get("hour")}:${get("minute")}`;
}

/** `Asia/Tokyo` の暦日で数えた「何日前」。見出しの「N 日前」と同じ数え方。 */
export function daysAgo(at: string, now: Date): number {
  const day = (d: Date): string =>
    new Intl.DateTimeFormat("en-CA", {
      timeZone: DAY_TZ,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).format(d);
  return Math.round(
    (Date.parse(`${day(now)}T00:00:00Z`) - Date.parse(`${day(new Date(at))}T00:00:00Z`)) / 86_400_000,
  );
}

/** 最終日は API と同じ Asia/Tokyo の暦日で数える。 */
export function archiveLastEventLabel(lastEventOn: string, now: Date): string {
  const today = new Intl.DateTimeFormat("en-CA", {
    timeZone: DAY_TZ,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
  const days = Math.round(
    (Date.parse(`${today}T00:00:00Z`) - Date.parse(`${lastEventOn}T00:00:00Z`)) / 86_400_000,
  );
  return `${lastEventOn} まで（${days} 日前）`;
}

/** Must の順序を崩さず、書庫のソースを退役済みの前に置く。 */
export function orderCoverageWithArchives(sources: SourceCoverage[]): SourceCoverage[] {
  const active = sources.filter((source) => !isRetired(source));
  return [
    ...active.filter((source) => !source.logical_source.startsWith("c03-")),
    ...active.filter((source) => source.logical_source.startsWith("c03-")),
    ...sources.filter(isRetired),
  ];
}
