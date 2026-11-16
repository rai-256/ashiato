import { msg } from "./messages";
import { DAY_TZ } from "./tokens";

export const UPDATE_STATES = ["off", "pending", "latest", "available", "stale", "available_stale", "unreadable", "not_running"] as const;
export type UpdateState = (typeof UPDATE_STATES)[number];
export type UpdateReason = { kind: string; failed_version: string | null; sqlstate: string | null };
export type LastUpdate = { at: string; result: "applied" | "rolled_back" | "not_applied"; to_version: string; from_version: string | null; reason: UpdateReason | null };
export type UpdateStatus = { state: UpdateState; installed: string; latest: string | null; found_at: string | null; checked_at: string | null; last_ok_at: string | null; reason: string | null; last_update?: LastUpdate | null };

const DATE = /^\d{4}-\d{2}-\d{2}T/;
export function isUpdateStatus(value: unknown): value is UpdateStatus {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  const date = (x: unknown): boolean => x === null || (typeof x === "string" && DATE.test(x));
  const last = v.last_update;
  const validLast = last === undefined || last === null || (typeof last === "object" && last !== null && (() => {
    const l = last as Record<string, unknown>;
    const reason = l.reason;
    const validReason = reason === null || (typeof reason === "object" && reason !== null && (() => {
      const r = reason as Record<string, unknown>;
      return typeof r.kind === "string" && (r.failed_version === null || typeof r.failed_version === "string") && (r.sqlstate === null || typeof r.sqlstate === "string");
    })());
    return typeof l.at === "string" && DATE.test(l.at) && ["applied", "rolled_back", "not_applied"].includes(l.result as string) &&
      typeof l.to_version === "string" && (l.from_version === null || typeof l.from_version === "string") && validReason;
  })());
  return typeof v.state === "string" && UPDATE_STATES.includes(v.state as UpdateState) && typeof v.installed === "string" &&
    (v.latest === null || typeof v.latest === "string") && date(v.found_at) && date(v.checked_at) && date(v.last_ok_at) &&
    (v.reason === null || typeof v.reason === "string") && validLast;
}

const m = msg("settings").update;
function day(value: string | null): string {
  if (value === null) return m.missingDate;
  const date = new Date(value);
  const parts = new Intl.DateTimeFormat("en-CA", { timeZone: DAY_TZ, month: "2-digit", day: "2-digit" }).formatToParts(date);
  const month = parts.find((part) => part.type === "month")?.value ?? "--";
  const day = parts.find((part) => part.type === "day")?.value ?? "--";
  return `${month}/${day}`;
}
export function formatDateTime(value: string | null): string {
  if (value === null) return m.missingDate;
  const date = new Date(value);
  const time = new Intl.DateTimeFormat("en-GB", { timeZone: DAY_TZ, hour: "2-digit", minute: "2-digit", hourCycle: "h23" }).format(date);
  return `${day(value)} ${time}`;
}
export function bannerLines(status: UpdateStatus): string[] {
  return bannerItems(status).map((item) => item.text);
}

export type BannerItem = { text: string; action: "procedure" | "none" };
export function bannerItems(status: UpdateStatus): BannerItem[] {
  const lines: BannerItem[] = [];
  if ((status.state === "available" || status.state === "available_stale") && status.latest !== null) lines.push({ text: m.available(status.latest, day(status.found_at)), action: "procedure" });
  if (status.state === "stale" || status.state === "available_stale") lines.push({ text: m.stale(day(status.last_ok_at), m.reasons[status.reason ?? ""] ?? m.unknownReason), action: "none" });
  const last = lastUpdateLine(status.last_update ?? null);
  if (last !== null) lines.push({ text: last, action: "none" });
  return lines;
}
export function updateReason(reason: string | null): string { return m.reasons[reason ?? ""] ?? m.missingDate; }

export function updateFailureReason(reason: UpdateReason | null): string {
  if (reason === null) return m.unknownReason;
  const kind = m.failureReasons[reason.kind] ?? m.dbError;
  if (reason.sqlstate === null) return kind;
  return m.sqlstate(reason.sqlstate, m.sqlstateClasses[reason.sqlstate.slice(0, 2)] ?? m.dbError);
}
export function lastUpdateLine(last: LastUpdate | null): string | null {
  if (last === null || last.result === "applied") return null;
  const from = last.from_version ?? m.previousVersion;
  if (last.reason === null) {
    if (last.result === "rolled_back") return m.rolledBack(last.to_version, from);
    return m.notApplied(last.to_version, from);
  }
  const reason = updateFailureReason(last.reason);
  if (last.result === "rolled_back") {
    if (last.reason.kind === "server_not_listening") return m.rolledBackNotListening(last.to_version, from, reason);
    return m.rolledBackMigration(last.to_version, from, last.reason.failed_version ?? m.unknownVersion, reason);
  }
  return m.notAppliedReason(last.to_version, from, reason);
}
