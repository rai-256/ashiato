// SPDX-License-Identifier: AGPL-3.0-only
import { DAY_TZ } from "./tokens";
import { isRetired, type SourceCoverage } from "./coverage";

export type ArchiveSourceStatus = {
  logical_source: string;
  last_event_on: string | null;
  last_archive_created_at: string | null;
};

export type ArchivesStatus = { sources: ArchiveSourceStatus[]; latest_archive?: { file_name: string; inserted: number; duplicate: number; unreadable: number } | null; pending_shape?: { archives: number; files: number } | null };

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
