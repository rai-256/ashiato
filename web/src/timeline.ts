// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の画面の「要約」「取れていない時間」「記録なしの事情」（ST25 / design D7 / D9 / D11）の型・形の検査・並べ方。
 *
 * **数と規則はサーバが持つ**（`GET /day`）。画面が持つのは文字の組み立てと、取れていない時間の行を置く位置だけ（D9）。
 */
import { msg } from "./messages";
import type { DayEntry } from "./stays";

const t = msg("timeline");
const day = msg("day");

/** 要約に出す上位の数（Q1 の本人の決定。サーバの `SUMMARY_TOP` と同じ値。jsdom の試験が固定する）。 */
export const SUMMARY_TOP = 3;
/** 行にする離席の長さの下限（分。Q1 の本人の決定。サーバの `AWAY_ROW_MIN_MINUTES` と同じ値）。 */
export const AWAY_ROW_MIN_MINUTES = 10;

export interface AppSeconds {
  app: string;
  seconds: number;
}
export interface AppSummary {
  seconds: number;
  top: AppSeconds[];
}
export interface DomainCount {
  domain: string;
  count: number;
}
export interface BrowserSummary {
  count: number;
  top: DomainCount[];
}
export interface ArchiveCount {
  display_name: string;
  count: number;
}
/** 記録の無いソースは鍵ごと無い。 */
export interface Summary {
  pc?: AppSummary;
  phone?: AppSummary;
  browser?: BrowserSummary;
  archive?: ArchiveCount[];
}

export type ReasonKind = "before_start" | "stopped" | "dropped" | "archive";
export interface Reason {
  kind: ReasonKind;
  count?: number;
}

export type GapKind = "away" | "stopped" | "excluded" | "unavailable";
export interface Gap {
  kind: GapKind;
  start: string;
  end: string;
  count?: number;
  reason?: string;
  /** どの端末の時間か（行の鍵。2 台の PC が同じ時刻に離席しても重ならない） */
  device_id?: string;
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null;
const isArr = <T>(v: unknown, ok: (x: unknown) => x is T): v is T[] => Array.isArray(v) && v.every(ok);
const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

const isAppSummary = (v: unknown): v is AppSummary =>
  isObj(v) && isNum(v.seconds) && isArr(v.top, (t): t is AppSeconds => isObj(t) && typeof t.app === "string" && isNum(t.seconds));
const isBrowserSummary = (v: unknown): v is BrowserSummary =>
  isObj(v) && isNum(v.count) && isArr(v.top, (t): t is DomainCount => isObj(t) && typeof t.domain === "string" && isNum(t.count));

/** 要約の形（無い鍵は許すが、あれば形が合っていなければならない）。 */
export function isSummary(v: unknown): v is Summary {
  return (
    isObj(v) &&
    (v.pc === undefined || isAppSummary(v.pc)) &&
    (v.phone === undefined || isAppSummary(v.phone)) &&
    (v.browser === undefined || isBrowserSummary(v.browser)) &&
    (v.archive === undefined || isArr(v.archive, (a): a is ArchiveCount => isObj(a) && typeof a.display_name === "string" && isNum(a.count)))
  );
}

const REASON_KINDS: readonly string[] = ["before_start", "stopped", "dropped", "archive"];
export const isReasons = (v: unknown): v is Reason[] =>
  isArr(v, (r): r is Reason => isObj(r) && typeof r.kind === "string" && REASON_KINDS.includes(r.kind) && (r.count === undefined || isNum(r.count)));

const GAP_KINDS: readonly string[] = ["away", "stopped", "excluded", "unavailable"];
export const isGaps = (v: unknown): v is Gap[] =>
  isArr(
    v,
    (g): g is Gap =>
      isObj(g) &&
      typeof g.kind === "string" &&
      GAP_KINDS.includes(g.kind) &&
      typeof g.start === "string" &&
      typeof g.end === "string" &&
      (g.count === undefined || isNum(g.count)) &&
      (g.reason === undefined || typeof g.reason === "string") &&
      (g.device_id === undefined || typeof g.device_id === "string"),
  );

/** 並びの 1 つ。滞在・移動・記録なし・消した・ほかの記録の行か、取れていない時間の行。 */
export type Placed = { at: "entry"; entry: DayEntry } | { at: "gap"; gap: Gap };

const minutesOf = (g: Gap): number => (Date.parse(g.end) - Date.parse(g.start)) / 60_000;

/**
 * 取れていない時間の行を並びに差し込む（D9）。始まりを含む行（`[start, end)`）の直後に、始まりの時刻順で。
 * 「消した」の行の中は、どの行にも入らない扱い。どの行にも入らなければ、始まりより前で最も近い行の直後（前に行が無ければ並びの先頭）。
 * 離席は `AWAY_ROW_MIN_MINUTES` 分に満たないものを行にしない。
 */
export function placeGaps(entries: DayEntry[], gaps: Gap[]): Placed[] {
  const rows = gaps
    .filter((g) => g.kind !== "away" || minutesOf(g) >= AWAY_ROW_MIN_MINUTES)
    .sort((a, b) => Date.parse(a.start) - Date.parse(b.start));
  const after: Gap[][] = entries.map(() => []);
  const head: Gap[] = [];
  for (const g of rows) {
    const at = Date.parse(g.start);
    let idx = entries.findIndex((e) => e.kind !== "erased" && Date.parse(e.start) <= at && at < Date.parse(e.end));
    if (idx < 0) {
      entries.forEach((e, i) => {
        if (Date.parse(e.start) <= at && (idx < 0 || Date.parse(e.start) >= Date.parse(entries[idx].start))) idx = i;
      });
    }
    (idx < 0 ? head : after[idx]).push(g);
  }
  const out: Placed[] = head.map((gap) => ({ at: "gap", gap }));
  entries.forEach((entry, i) => {
    out.push({ at: "entry", entry });
    for (const gap of after[i]) out.push({ at: "gap", gap });
  });
  return out;
}

/** 秒を `2 時間 45 分` / `30 分` で書く。**上位の各アプリ（`hm`）と同じく切り捨て**（合計と内訳の丸めを揃える）。 */
export function spent(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const h = Math.floor(minutes / 60);
  return h === 0 ? day.minutes(minutes) : day.hoursMinutes(h, minutes % 60);
}

/** 上位のアプリの 1 つの時間を `1:30` で書く。 */
function hm(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
}

/** 頭に続ける内訳（無ければ頭だけ）。 */
const withTop = (head: string, top: string[]): string => (top.length > 0 ? `${head}${t.lead}${top.join(t.topSep)}` : head);

/** 要約の各行の文字。**どの行も種類の文字で始める**（色を外しても読み分けられる）。記録の無いソースの行は無い。 */
export function summaryLines(s: Summary): { key: string; text: string }[] {
  const out: { key: string; text: string }[] = [];
  const apps = (label: string, a: AppSummary): string =>
    withTop(`${label} ${spent(a.seconds)}`, a.top.slice(0, SUMMARY_TOP).map((x) => `${x.app} ${hm(x.seconds)}`));
  if (s.pc !== undefined) out.push({ key: "pc", text: apps(t.pc, s.pc) });
  if (s.phone !== undefined) out.push({ key: "phone", text: apps(t.phone, s.phone) });
  if (s.browser !== undefined) {
    const sites = s.browser.top.slice(0, SUMMARY_TOP).map((x) => `${x.domain} ${x.count}`);
    out.push({ key: "browser", text: withTop(t.browserCount(s.browser.count), sites) });
  }
  for (const a of s.archive ?? []) out.push({ key: `archive-${a.display_name}`, text: t.archiveCount(a.display_name, a.count) });
  return out;
}

/** 記録なしの事情の文字（D11 の並びのまま）。 */
export function reasonText(r: Reason): string {
  switch (r.kind) {
    case "before_start":
      return t.reason.before_start;
    case "stopped":
      return t.reason.stopped;
    case "dropped":
      return t.reason.dropped(r.count ?? 0);
    case "archive":
      return t.reason.archive(r.count ?? 0);
  }
}

/** 記録なしの事情を 1 行に並べる。 */
export function reasonsLine(rs: Reason[]): string {
  return rs.map(reasonText).join(t.sep);
}

/** 取れていない時間の行の文字。`range` は「始まり – 終わり」。 */
export function gapText(g: Gap, range: string): string {
  switch (g.kind) {
    case "away":
      return t.gap.away(range);
    case "stopped":
      return t.gap.stopped(range);
    case "excluded":
      return t.gap.excluded(range, g.count ?? 0);
    case "unavailable":
      return g.reason === "clock_skew_abandoned" ? t.gap.clockSkew(range) : t.gap.retention(range);
  }
}

/**
 * 文字と背景の組（8.3）。**描くときも検査もこの表から引く** —— 検査が実際に描く色とずれない。
 * 値は `SCHEMES` の鍵（`text` / `muted` が文字、`ground` / `surface1` / `surface2` が背景）。
 */
export const TIMELINE_PAIRS = {
  /** 滞在の行（面 1）の要約 */
  summaryOnStay: { fg: "muted", bg: "surface1" },
  /** 滞在以外の行（地）の要約・ほかの記録 */
  summaryOnGround: { fg: "muted", bg: "ground" },
  /** 取れていない時間の行 */
  gap: { fg: "muted", bg: "ground" },
  /** 記録なしの事情 */
  reason: { fg: "muted", bg: "ground" },
  /** 開いた中身（中身の面を自分で塗る） */
  detail: { fg: "text", bg: "surface2" },
  /** 畳んだ行（開いた中身の面の上） */
  folded: { fg: "muted", bg: "surface2" },
} as const;
export type TimelineRole = keyof typeof TIMELINE_PAIRS;

/** `GET /day/records` の 1 件（D8）。`kind` は `foreground` / `away` / `stopped` / `excluded` / `visit` / `unavailable` / `archive`。 */
export interface DayRecord {
  id: string;
  kind: string;
  source: string;
  display_name?: string;
  start: string;
  end?: string;
  tz_id: string;
  app?: string;
  title?: string;
  domain?: string;
  count?: number;
}

const optStr = (v: unknown): boolean => v === undefined || typeof v === "string";

export const isDayRecords = (v: unknown): v is DayRecord[] =>
  isArr(
    v,
    (r): r is DayRecord =>
      isObj(r) &&
      typeof r.id === "string" &&
      typeof r.kind === "string" &&
      typeof r.source === "string" &&
      typeof r.start === "string" &&
      typeof r.tz_id === "string" &&
      optStr(r.display_name) &&
      optStr(r.end) &&
      optStr(r.app) &&
      optStr(r.title) &&
      optStr(r.domain) &&
      (r.count === undefined || isNum(r.count)),
  );

/** 同じアプリの連続を畳んだ 1 行（D8）。`end` は中の最後の記録の終わり（点なら出来事の時刻）。 */
export interface FoldRun {
  at: "run";
  /** 開閉の鍵（中の最初の記録の id） */
  key: string;
  source: string;
  app?: string;
  start: string;
  end: string;
  count: number;
  records: DayRecord[];
}
export type Folded = { at: "one"; record: DayRecord } | FoldRun;

/** 畳む記録の種類。取れていない時間（離席・停止・除外・取れていない）は畳まず 1 行ずつ出す（D8）。 */
const FOLDABLE_KINDS: readonly string[] = ["foreground", "visit", "archive"];

const runKey = (r: DayRecord): string => `${r.source}\u0000${r.display_name ?? ""}\u0000${r.app ?? ""}`;

/**
 * 同じ `source` の同じアプリ（ブラウザは同じ `browser` ＝ `app`）が続く間を 1 行にする（D8）。
 * 別のアプリが挟まれば別の行。1 件だけの連続は畳まない。入力は時刻順。
 */
export function foldRuns(records: DayRecord[]): Folded[] {
  const out: Folded[] = [];
  let run: DayRecord[] = [];
  const flush = (): void => {
    if (run.length === 1) out.push({ at: "one", record: run[0] });
    else if (run.length > 1) {
      const last = run[run.length - 1];
      out.push({ at: "run", key: run[0].id, source: run[0].source, app: run[0].app, start: run[0].start, end: last.end ?? last.start, count: run.length, records: run });
    }
    run = [];
  };
  for (const r of records) {
    if (!FOLDABLE_KINDS.includes(r.kind)) {
      flush();
      out.push({ at: "one", record: r });
      continue;
    }
    if (run.length > 0 && runKey(run[0]) !== runKey(r)) flush();
    run.push(r);
  }
  flush();
  return out;
}

/** 出どころの文字（種類は文字で言う。C1）。 */
export function sourceLabel(r: { source: string; display_name?: string }): string {
  switch (r.source) {
    case "pc":
      return t.pc;
    case "phone":
      return t.phone;
    case "browser":
      return t.browser;
    default:
      return t.archive(r.display_name ?? "");
  }
}

/** 地域の時刻（`8:02`）。知らない地域名なら `undefined`。 */
function clockIn(iso: string, tz: string): string | undefined {
  try {
    return new Intl.DateTimeFormat("en-GB", { timeZone: tz, hour: "numeric", minute: "2-digit", hourCycle: "h23" }).format(new Date(iso)).replace(/^0(\d):/, "$1:");
  } catch {
    return undefined;
  }
}

/** Asia/Tokyo でない記録に添える `（London 8:02）`（C11。地域の名前は `tz_id` の最後の区切りの後）。 */
export function regionStamp(tz: string, iso: string): string {
  if (tz === "Asia/Tokyo") return "";
  const time = clockIn(iso, tz);
  return time === undefined ? "" : t.region(tz.slice(tz.lastIndexOf("/") + 1), time);
}

/** 記録 1 件の文字（種類の文字・アプリ・題名・サイト）。時刻は含まない。 */
export function recordText(r: DayRecord): string {
  switch (r.kind) {
    case "away":
      return t.record.away;
    case "stopped":
      return t.record.stopped;
    case "excluded":
      return t.record.excluded(r.count ?? 0);
    case "unavailable":
      return t.record.unavailable(sourceLabel(r));
    default:
      return joinParts([sourceLabel(r), r.app, r.title, r.domain]);
  }
}

/** 空でないものを「・」で並べる。 */
export function joinParts(parts: (string | undefined)[]): string {
  return parts.filter((s): s is string => s !== undefined && s !== "").join(t.sep);
}

/** 畳んだ行の文字（`9:02 – 9:14 ・ PC ・ VS Code ・ 23 件`）。 */
export function foldedText(run: FoldRun, range: string): string {
  return joinParts([range, sourceLabel({ source: run.source, display_name: run.records[0].display_name }), run.app, t.count(run.count)]);
}
