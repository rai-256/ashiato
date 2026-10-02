// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 1 日の画面の「要約」「取れていない時間」「記録なしの事情」（ST25 / design D7 / D9 / D11）の型・形の検査・並べ方。
 *
 * **数と規則はサーバが持つ**（`GET /day`）。画面が持つのは文字の組み立てと、取れていない時間の行を置く位置だけ（D9）。
 */
import type { DayEntry } from "./stays";

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
      (g.reason === undefined || typeof g.reason === "string"),
  );

/** 並びの 1 つ。滞在・移動・記録なし・消した・ほかの記録の行か、取れていない時間の行。 */
export type Placed = { at: "entry"; entry: DayEntry } | { at: "gap"; gap: Gap };

const minutesOf = (g: Gap): number => (Date.parse(g.end) - Date.parse(g.start)) / 60_000;

/**
 * 取れていない時間の行を並びに差し込む（D9）。始まりを含む行（`[start, end)`）の直後に、始まりの時刻順で。
 * どの行にも入らなければ、始まりより前で最も近い行の直後（前に行が無ければ並びの先頭）。
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
    let idx = entries.findIndex((e) => Date.parse(e.start) <= at && at < Date.parse(e.end));
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

/** 秒を `2 時間 45 分` / `30 分` で書く。 */
export function spent(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  const h = Math.floor(minutes / 60);
  return h === 0 ? `${minutes} 分` : `${h} 時間 ${minutes % 60} 分`;
}

/** 上位のアプリの 1 つの時間を `1:30` で書く。 */
function hm(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
}

/** 要約の各行の文字。**どの行も種類の文字で始める**（色を外しても読み分けられる）。記録の無いソースの行は無い。 */
export function summaryLines(s: Summary): { key: string; text: string }[] {
  const out: { key: string; text: string }[] = [];
  const apps = (label: string, a: AppSummary): string =>
    `${label} ${spent(a.seconds)}${a.top.length > 0 ? ` — ${a.top.slice(0, SUMMARY_TOP).map((t) => `${t.app} ${hm(t.seconds)}`).join(" · ")}` : ""}`;
  if (s.pc !== undefined) out.push({ key: "pc", text: apps("PC", s.pc) });
  if (s.phone !== undefined) out.push({ key: "phone", text: apps("スマホ", s.phone) });
  if (s.browser !== undefined) {
    const sites = s.browser.top.slice(0, SUMMARY_TOP).map((t) => `${t.domain} ${t.count}`).join(" · ");
    out.push({ key: "browser", text: `ブラウザ ${s.browser.count} 件${sites === "" ? "" : ` — ${sites}`}` });
  }
  for (const a of s.archive ?? []) out.push({ key: `archive-${a.display_name}`, text: `書庫: ${a.display_name} ${a.count} 件` });
  return out;
}

/** 記録なしの事情の文字（D11 の並びのまま）。 */
export function reasonText(r: Reason): string {
  switch (r.kind) {
    case "before_start":
      return "位置の収集を始める前";
    case "stopped":
      return "止めていた";
    case "dropped":
      return `端末が破棄 ${r.count ?? 0} 件`;
    case "archive":
      return `書庫に位置 ${r.count ?? 0} 件`;
  }
}

/** 取れていない時間の行の文字。`range` は「始まり – 終わり」。 */
export function gapText(g: Gap, range: string): string {
  switch (g.kind) {
    case "away":
      return `離席 ${range}`;
    case "stopped":
      return `PC 停止 ${range}`;
    case "excluded":
      return `除外 ${range}（${g.count ?? 0} 件。本文なし）`;
    case "unavailable":
      return g.reason === "clock_skew_abandoned" ? `スマホ 時計のずれで取れていない ${range}` : `スマホ 取得元に残っていない ${range}`;
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
  /** 開いた中身 */
  detail: { fg: "text", bg: "surface1" },
  /** 畳んだ行 */
  folded: { fg: "muted", bg: "ground" },
} as const;
export type TimelineRole = keyof typeof TIMELINE_PAIRS;
