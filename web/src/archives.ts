// SPDX-License-Identifier: AGPL-3.0-only
import { DAY_TZ } from "./tokens";

export type ArchiveSourceStatus = {
  logical_source: string;
  last_event_on: string | null;
  last_archive_created_at: string | null;
};

export type ArchivesStatus = { sources: ArchiveSourceStatus[] };

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
