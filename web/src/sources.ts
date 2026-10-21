// SPDX-License-Identifier: AGPL-3.0-only
import { msg } from "./messages";

/** `GET /sources` の 1 行（ST13 / design D10）。 */
export type SourceRow = {
  logical_source: string;
  display_name: string;
  acquisition: string;
  fetch_trigger: string;
  fetch_interval_sec: number | null;
  retired_on: string | null;
  alert_after_days: number | null;
  last_archive_on: string | null;
  alert_days_left: number | null;
  alert_passed: boolean;
  no_archive: boolean;
};

export type SourcesResponse = {
  sources: SourceRow[];
  device_notice: { unused_app_restrictions: string | null } | null;
};

const m = msg("grid").sourceNote;

/**
 * 取得の間隔の書き方（design D11（仮））。120 未満か 60 で割り切れなければ秒、
 * 3600 で割り切れれば時間、それ以外は分。
 */
export function intervalLabel(sec: number): string {
  if (sec < 120 || sec % 60 !== 0) return m.every(String(sec), m.unitSec);
  if (sec % 3600 === 0) return m.every(String(sec / 3600), m.unitHour);
  return m.every(String(sec / 60), m.unitMin);
}

/** 書庫の通知までの文字。起点（最後の書庫の日）を必ず添える。負の残り日数は出さない。 */
export function archiveAlertLabel(row: SourceRow): string {
  if (row.no_archive || row.last_archive_on === null) return m.noArchive;
  if (row.alert_passed || row.alert_days_left === null) return m.alertPassed(row.last_archive_on);
  return m.alertLeft(String(row.alert_days_left), row.last_archive_on);
}

/**
 * 格子の見出しに添える取り方と間隔（design D10）。**添えないときは `undefined`**。
 *
 * - `/sources` を読めない・行の無いソース・退役したソースには添えない
 * - 書庫は「手作業・2 か月ごと」に通知までを続ける
 * - `c01-location` が休止の対象のままなら、間隔の代わりに短い知らせ
 */
export function sourceNote(
  logicalSource: string,
  sources: SourcesResponse | null,
): string | undefined {
  const row = sources?.sources.find((s) => s.logical_source === logicalSource);
  if (sources === null || row === undefined || row.retired_on !== null) return undefined;
  if (logicalSource === "c01-location" && sources.device_notice?.unused_app_restrictions === "not_exempt") {
    return ` — ${m.pausable}`;
  }
  if (row.acquisition === "archive") {
    return ` — ${m.trigger.manual} · ${archiveAlertLabel(row)}`;
  }
  if (row.fetch_trigger === "interval" && row.fetch_interval_sec !== null) {
    return ` — ${intervalLabel(row.fetch_interval_sec)}`;
  }
  const trigger = m.trigger[row.fetch_trigger];
  return trigger === undefined ? undefined : ` — ${trigger}`;
}
