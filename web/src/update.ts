import { msg } from "./messages";
import { DAY_TZ } from "./tokens";

export const UPDATE_STATES = ["off", "pending", "latest", "available", "stale", "available_stale", "unreadable", "not_running"] as const;
export type UpdateState = (typeof UPDATE_STATES)[number];
export type UpdateStatus = { state: UpdateState; installed: string; latest: string | null; found_at: string | null; checked_at: string | null; last_ok_at: string | null; reason: string | null };

const DATE = /^\d{4}-\d{2}-\d{2}T/;
export function isUpdateStatus(value: unknown): value is UpdateStatus {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  const date = (x: unknown): boolean => x === null || (typeof x === "string" && DATE.test(x));
  return typeof v.state === "string" && UPDATE_STATES.includes(v.state as UpdateState) && typeof v.installed === "string" &&
    (v.latest === null || typeof v.latest === "string") && date(v.found_at) && date(v.checked_at) && date(v.last_ok_at) &&
    (v.reason === null || typeof v.reason === "string");
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
  const lines: string[] = [];
  if ((status.state === "available" || status.state === "available_stale") && status.latest !== null) lines.push(m.available(status.latest, day(status.found_at)));
  if (status.state === "stale" || status.state === "available_stale") lines.push(m.stale(day(status.last_ok_at), m.reasons[status.reason ?? ""] ?? m.unknownReason));
  return lines;
}
export function updateReason(reason: string | null): string { return m.reasons[reason ?? ""] ?? m.missingDate; }
