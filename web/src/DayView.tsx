// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useRef, useState } from "react";
import { todayInTz } from "./App";
import {
  clock,
  criteriaLabel,
  dateLabel,
  duration,
  isDayView,
  shiftDay,
  type CriteriaTag,
  type DayEntry,
  type DayView as DayData,
} from "./stays";
import {
  foldedText,
  foldRuns,
  gapText,
  isDayRecords,
  joinParts,
  placeGaps,
  reasonsLine,
  recordText,
  regionStamp,
  summaryLines,
  TIMELINE_PAIRS,
  type DayRecord,
  type FoldRun,
  type Gap,
  type Reason,
  type Summary,
  type TimelineRole,
} from "./timeline";
import { msg } from "./messages";
import { FOCUS_ATTR, useFocusRule, useScheme } from "./scheme";
import { controlStyle as control, destructiveStyle, inputStyle, linkStyle, markStyle, pageStyle, titleStyle } from "./surface";
import { font, MIN_TARGET_PX, RADIUS, SCHEMES, SPACE, TYPE, tone, type Scheme, type SchemeColors } from "./tokens";

// 明暗とフォーカスの輪郭は `scheme.ts` に移した（S-1 も使う）。ここから引いていた部品とテストのために同じ名前で出す
export { FOCUS_ATTR, focusRule, useFocusRule, useScheme } from "./scheme";

/** 読み出しの状態。**「読み込み中」「失敗」「滞在が無い」を分ける**（spec「読み出しの失敗と『滞在が無い』を区別する」）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

const m = msg("day");
const common = msg("common");

/** 要約・事情の行間（添える文字。本文の 1.70 より詰める） */
const SUMMARY_LINE = 1.45;

/** 文字と背景の組（`TIMELINE_PAIRS`。8.3 の検査と同じ表から引く）。地の上の組は背景を描かない。 */
function paint(c: SchemeColors, role: TimelineRole): React.CSSProperties {
  const pair = TIMELINE_PAIRS[role];
  return pair.bg === "ground" ? { color: tone(c[pair.fg]) } : { color: tone(c[pair.fg]), background: tone(c[pair.bg]) };
}

/** 要約（ソースごとに 1 行。種類の文字で始まる）。消した行・要約の無い行には出ない。 */
function SummaryView({ summary, role, scheme }: { summary: Summary | undefined; role: TimelineRole; scheme: Scheme }): React.ReactElement | null {
  if (summary === undefined) return null;
  const lines = summaryLines(summary);
  if (lines.length === 0) return null;
  return (
    // 要約と事情は添える文字の大きさ（閉じた行を 200 px 以下・開いてすぐ 6 行に収める。spec の 1 日の画面の下限）
    <div data-testid="summary" style={{ font: font(TYPE.caption, 400, SUMMARY_LINE), ...paint(SCHEMES[scheme], role) }}>
      {lines.map((l) => (
        <p key={l.key} data-testid={`summary-${l.key.split("-")[0]}`} style={{ margin: 0 }}>
          {l.text}
        </p>
      ))}
    </div>
  );
}

/** 記録なしの事情（無ければ何も出さない）。 */
function ReasonsView({ reasons, scheme }: { reasons: Reason[] | undefined; scheme: Scheme }): React.ReactElement | null {
  if (reasons === undefined || reasons.length === 0) return null;
  return (
    <p data-testid="reasons" style={{ margin: 0, font: font(TYPE.caption, 400, SUMMARY_LINE), ...paint(SCHEMES[scheme], "reason") }}>
      {reasonsLine(reasons)}
    </p>
  );
}

/** 取れていない時間の行。**種類は文字で言う**（色も枠も使わない）。 */
function GapRow({ gap, viewing, scheme }: { gap: Gap; viewing: string; scheme: Scheme }): React.ReactElement {
  const range = m.range(clock(gap.start, viewing), clock(gap.end, viewing));
  return (
    <li data-testid="row-gap" data-kind={`gap-${gap.kind}`} style={{ padding: `${SPACE.rowY}px ${SPACE.rowX}px`, margin: `0 0 ${SPACE.rowGap}px`, ...paint(SCHEMES[scheme], "gap") }}>
      {gapText(gap, range)}
    </li>
  );
}

/** 取れていない時間の行の鍵。 */
function gapKey(g: Gap): string {
  return `gap-${g.kind}-${g.start}`;
}

type StayDetailCount = { logical_source: string; display_name: string; count: number };
type StayDetail = { stay_id: string; start: string; end: string; counts: StayDetailCount[] };

function isStayDetail(v: unknown): v is StayDetail {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.stay_id === "string" &&
    typeof o.start === "string" &&
    typeof o.end === "string" &&
    Array.isArray(o.counts) &&
    o.counts.every((count) => {
      if (typeof count !== "object" || count === null) return false;
      const c = count as Record<string, unknown>;
      return typeof c.logical_source === "string" && typeof c.display_name === "string" && typeof c.count === "number";
    })
  );
}

/**
 * S-2 の最小形（ST16 / design D8）—— 1 日の滞在の一覧。
 *
 * 行の形は深掘り Q4 の proto の出力のまま: 見出し＝時刻の範囲 / 添える値＝長さ・始まり – 終わり /
 * 「移動 42 分」/「記録なし 8:20 – 16:40」（**文字で**区別し、左端の線は補助）/ 一覧の上に作った基準。
 */
export function DayView({ date }: { date: string }): React.ReactElement {
  const scheme = useScheme();
  useFocusRule(scheme);
  const c = SCHEMES[scheme];
  const [data, setData] = useState<Load<DayData>>({ at: "loading" });
  const [reload, setReload] = useState(0);

  useEffect(() => {
    let live = true;
    setData({ at: "loading" });
    fetch(`/api/day?date=${date}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isDayView(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => live && setData({ at: "ok", value }))
      .catch((e: unknown) => live && setData({ at: "failed", why: e instanceof Error ? e.message : "unknown" }));
    return () => {
      live = false;
    };
  }, [date, reload]);

  const go = (next: string): void => {
    window.location.hash = `#/day/${next}`;
  };

  return (
    <main
      data-testid="day-view"
      data-scheme={scheme}
      style={pageStyle(c)}
    >
      <nav style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: SPACE.targetGap, marginBottom: round1(SPACE.gap * 0.3) }}>
        <a href="#/coverage" {...{ [FOCUS_ATTR]: "" }} style={linkStyle(c)}>
          {m.toCoverage}
        </a>
        <a href="#/master" {...{ [FOCUS_ATTR]: "" }} style={linkStyle(c)}>
          {m.toMaster}
        </a>
      </nav>
      <h1 data-testid="day-title" style={titleStyle(c)}>
        {m.title(dateLabel(date))}
      </h1>
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.targetGap, marginBottom: SPACE.underTools }}>
        <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(c)} onClick={() => go(shiftDay(date, -1))}>
          {m.prev}
        </button>
        <input
          type="date"
          aria-label={m.pickDate}
          {...{ [FOCUS_ATTR]: "" }}
          value={date}
          style={{ ...inputStyle(c), font: font(TYPE.caption) }}
          onChange={(e) => {
            if (/^\d{4}-\d{2}-\d{2}$/.test(e.target.value)) go(e.target.value);
          }}
        />
        <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(c)} onClick={() => go(shiftDay(date, 1))}>
          {m.next}
        </button>
      </div>

      {data.at === "loading" && <p data-testid="day-loading">{common.loading}</p>}
      {data.at === "failed" && (
        <p role="alert" data-testid="day-error">
          {m.failed(data.why)}
        </p>
      )}
      {data.at === "ok" && <Entries view={data.value} scheme={scheme} future={date > todayInTz(new Date())} reload={() => setReload((n) => n + 1)} />}
    </main>
  );
}

function Entries({ view, scheme, future, reload }: { view: DayData; scheme: Scheme; future: boolean; reload: () => void }): React.ReactElement {
  const c = SCHEMES[scheme];
  const main = view.criteria[0];
  const byId = new Map<number, CriteriaTag>(view.criteria.map((t) => [t.criteria_id, t]));
  const stays = view.entries.filter((e) => e.kind === "stay").length;
  const [openId, setOpenId] = useState<string | null>(null);
  const [detail, setDetail] = useState<Load<StayDetail> | null>(null);
  const [records, setRecords] = useState<Load<DayRecord[]> | null>(null);
  const detailGeneration = useRef(0);
  const [actionError, setActionError] = useState<{ entryKey: string; message: string } | null>(null);

  const open = (entry: DayEntry): void => {
    if (entry.kind === "erased") return;
    const id = rowKey(entry);
    if (openId === id) {
      detailGeneration.current += 1;
      setOpenId(null);
      setDetail(null);
      setRecords(null);
      return;
    }
    const generation = detailGeneration.current + 1;
    detailGeneration.current = generation;
    const current = (): boolean => generation === detailGeneration.current;
    const why = (e: unknown): string => (e instanceof Error ? e.message : "unknown");
    const read = async (path: string): Promise<unknown> => {
      const res = await fetch(path);
      if (!res.ok) throw new Error(`status_${res.status}`);
      return res.json();
    };
    setOpenId(id);
    setRecords({ at: "loading" });
    // **行の実際の範囲で読む**（D7）。日をまたぐ行も前の日の側を含めて出す（`from` / `to` は日の端で切られない）
    const range = `date=${view.date}&from=${encodeURIComponent(entry.start)}&to=${encodeURIComponent(entry.end)}`;
    read(`/api/day/records?${range}`)
      .then((body) => {
        if (!isDayRecords(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => current() && setRecords({ at: "ok", value }))
      .catch((e: unknown) => current() && setRecords({ at: "failed", why: why(e) }));
    if (entry.kind !== "stay" || entry.id === undefined) {
      setDetail(null);
      return;
    }
    setDetail({ at: "loading" });
    read(`/api/stays/detail?stay_id=${encodeURIComponent(entry.id)}`)
      .then((body) => {
        if (!isStayDetail(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => current() && setDetail({ at: "ok", value }))
      .catch((e: unknown) => current() && setDetail({ at: "failed", why: why(e) }));
  };

  const action = (path: string, body: Record<string, unknown>, entryKey: string): void => {
    setActionError(null);
    fetch(path, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) })
      .then((res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        setOpenId(null);
        setDetail(null);
        setRecords(null);
        reload();
      })
      .catch(() => setActionError({ entryKey, message: m.actionFailed }));
  };
  return (
    <>
      {main !== undefined && (
        <p data-testid="criteria" style={{ color: tone(c.muted), margin: "0 0 8px" }}>
          {m.criteria(criteriaLabel(main))}
          {view.criteria.length > 1 && m.criteriaOthers(view.criteria.slice(1).map(criteriaLabel))}
        </p>
      )}
      {/* **まだ来ていない日に「滞在はありません」と言わない**（R45）—— 事実として読まれる */}
      {stays === 0 && future && (
        <p data-testid="day-future" style={{ margin: "0 0 8px" }}>
          {m.future}
        </p>
      )}
      {stays === 0 && !future && (
        <p data-testid="day-empty" style={{ margin: "0 0 8px" }}>
          {m.empty}
        </p>
      )}
      <ol aria-label={m.listLabel} style={{ listStyle: "none", margin: 0, padding: 0 }}>
        {view.browser_through !== undefined && (
          <li data-testid="row-browser-pending" data-kind="gap-browser" style={{ padding: `${SPACE.rowY}px ${SPACE.rowX}px`, margin: `0 0 ${SPACE.rowGap}px`, ...paint(c, "gap") }}>
            {m.browserPending(clock(view.browser_through, view.date))}
          </li>
        )}
        {placeGaps(view.entries, view.gaps ?? []).map((p) => {
          if (p.at === "gap") return <GapRow key={gapKey(p.gap)} gap={p.gap} viewing={view.date} scheme={scheme} />;
          const e = p.entry;
          return (
            <Row
              key={`${e.kind}-${e.start}`}
              entry={e}
              viewing={view.date}
              scheme={scheme}
              criteria={e.criteria_id !== undefined && main !== undefined && e.criteria_id !== main.criteria_id ? byId.get(e.criteria_id) : undefined}
              open={openId === rowKey(e)}
              detail={openId === rowKey(e) ? detail : null}
              records={openId === rowKey(e) ? records : null}
              onOpen={() => open(e)}
              onAction={action}
              actionError={actionError}
            />
          );
        })}
      </ol>
    </>
  );
}

function Row({
  entry,
  viewing,
  scheme,
  criteria,
  open,
  detail,
  records,
  onOpen,
  onAction,
  actionError,
}: {
  entry: DayEntry;
  viewing: string;
  scheme: Scheme;
  criteria: CriteriaTag | undefined;
  open: boolean;
  detail: Load<StayDetail> | null;
  records: Load<DayRecord[]> | null;
  onOpen: () => void;
  onAction: (path: string, body: Record<string, unknown>, entryKey: string) => void;
  actionError: { entryKey: string; message: string } | null;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const range = m.range(clock(entry.start, viewing), clock(entry.end, viewing));
  const long = duration(entry.start, entry.end);
  // **種類は文字で区別し、しるし（線の輪の中に記号）は添えるだけ**（design D8 / 本人の決定 2026-10-12）。
  // 移動のしるしだけ差し色（本人の決定）。どのしるしも面の 3 段すべてに 3:1 以上（NFR-23）
  const mark = <span data-mark={entry.kind} aria-hidden="true" style={markStyle(entry.kind === "move" ? c.spark : c.muted)}>{m.mark[entry.kind]}</span>;
  const base: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    gap: SPACE.targetGap,
    minHeight: MIN_TARGET_PX,
    padding: `${SPACE.rowY}px ${SPACE.rowX}px`,
    margin: `0 0 ${SPACE.rowGap}px`,
    borderRadius: RADIUS.row,
  };
  const entryKey = entry.id ?? entry.stay_ids?.join(",") ?? "";
  if (entry.kind === "stay") {
    return (
      <li data-testid="row-stay" data-kind="stay" style={{ ...base, alignItems: "flex-start", gap: SPACE.inRow, background: tone(c.surface1), border: `1px solid ${tone(c.line)}` }}>
        {mark}
        <div style={{ flex: 1, minWidth: 0 }}>
        <button
          type="button"
          aria-expanded={open}
          {...{ [FOCUS_ATTR]: "" }}
          onClick={onOpen}
          style={{ display: "block", width: "100%", minHeight: MIN_TARGET_PX, padding: 0, textAlign: "left", color: "inherit", font: "inherit", background: "transparent", border: 0 }}
        >
          <h2 style={{ font: font(TYPE.h2, 600, 1.35), margin: 0 }}>{range}</h2>
          <p style={{ margin: 0, color: tone(c.muted) }}>{m.lengthAndRange(long, range)}</p>
          {criteria !== undefined && <p data-testid="row-criteria" style={{ margin: 0, color: tone(c.muted) }}>{m.rowCriteria(criteriaLabel(criteria))}</p>}
          <SummaryView summary={entry.summary} role="summaryOnStay" scheme={scheme} />
        </button>
        {open && <StayDetailView entry={entry} viewing={viewing} detail={detail} records={records} onAction={onAction} scheme={scheme} />}
        {actionError?.entryKey === entryKey && <p role="alert">{actionError.message}</p>}
        </div>
      </li>
    );
  }
  if (entry.kind === "erased") {
    return (
      <li data-testid="row-erased" data-kind="erased" style={{ ...base, flexWrap: "wrap", color: tone(c.muted) }}>
        {mark}
        <span>{m.erased(range)}</span>
        <button type="button" {...{ [FOCUS_ATTR]: "" }} onClick={() => onAction("/api/stays/restore", { stay_ids: entry.stay_ids ?? [] }, entryKey)} style={control(c)}>
          {m.restore}
        </button>
        {actionError?.entryKey === entryKey && <p role="alert" style={{ width: "100%", margin: 0 }}>{actionError.message}</p>}
      </li>
    );
  }
  return (
    <li data-testid={`row-${entry.kind}`} data-kind={entry.kind} style={{ ...base, alignItems: "flex-start", color: tone(c.muted) }}>
      {mark}
      <div style={{ flex: 1, minWidth: 0 }}>
        <button
          type="button"
          aria-expanded={open}
          {...{ [FOCUS_ATTR]: "" }}
          onClick={onOpen}
          style={{ display: "block", width: "100%", minHeight: MIN_TARGET_PX, padding: 0, textAlign: "left", color: "inherit", font: "inherit", background: "transparent", border: 0 }}
        >
          {entry.kind === "move" ? m.move(long) : entry.kind === "other" ? m.other(range) : m.noRecord(range)}
          {entry.kind === "move" && <span>{m.alsoRange(range)}</span>}
        </button>
        <SummaryView summary={entry.summary} role="summaryOnGround" scheme={scheme} />
        <ReasonsView reasons={entry.reasons} scheme={scheme} />
        {open && <RecordsView load={records} viewing={viewing} scheme={scheme} />}
      </div>
    </li>
  );
}

/** 行の開閉の鍵。滞在は `id`、ほかの行は種類と始まり（ST22 の `openId` 1 つのまま）。 */
function rowKey(entry: DayEntry): string {
  return entry.id ?? `${entry.kind}-${entry.start}`;
}

/** 記録 1 件の行の文字。時刻は Asia/Tokyo。地域が違えば地域の時刻を添える。 */
function recordLine(r: DayRecord, viewing: string): string {
  return `${clock(r.start, viewing)}${regionStamp(r.tz_id, r.start)} ${recordText(r)}`;
}

/** 開いた中身（時刻順の記録）。同じアプリの連続は畳み、畳んだ行も一度に 1 つ開く（D8）。 */
function RecordsView({ load, viewing, scheme }: { load: Load<DayRecord[]> | null; viewing: string; scheme: Scheme }): React.ReactElement | null {
  const [foldOpen, setFoldOpen] = useState<string | null>(null);
  if (load === null) return null;
  if (load.at === "loading") return <p>{m.recordsLoading}</p>;
  if (load.at === "failed") return <p role="alert">{m.recordsFailed(load.why)}</p>;
  const c = SCHEMES[scheme];
  return (
    <ol
      data-testid="records"
      aria-label={m.recordsLabel}
      style={{ listStyle: "none", margin: "8px 0 0", padding: `${SPACE.rowY}px ${SPACE.rowX}px`, borderRadius: RADIUS.row, ...paint(c, "detail") }}
    >
      {foldRuns(load.value).map((f) =>
        f.at === "one" ? (
          <li key={f.record.id} style={{ margin: 0 }}>
            {recordLine(f.record, viewing)}
          </li>
        ) : (
          <FoldedRow key={f.key} run={f} viewing={viewing} open={foldOpen === f.key} onToggle={() => setFoldOpen(foldOpen === f.key ? null : f.key)} c={c} />
        ),
      )}
    </ol>
  );
}

function FoldedRow({ run, viewing, open, onToggle, c }: { run: FoldRun; viewing: string; open: boolean; onToggle: () => void; c: SchemeColors }): React.ReactElement {
  return (
    <li style={{ margin: 0 }}>
      <button
        type="button"
        aria-expanded={open}
        {...{ [FOCUS_ATTR]: "" }}
        onClick={onToggle}
        style={{ display: "block", width: "100%", minHeight: MIN_TARGET_PX, padding: 0, textAlign: "left", font: "inherit", border: 0, ...paint(c, "folded") }}
      >
        {foldedText(run, m.range(clock(run.start, viewing), clock(run.end, viewing)))}
      </button>
      {open && (
        <ol data-testid="fold-records" style={{ listStyle: "none", margin: 0, padding: "0 0 0 12px" }}>
          {run.records.map((r) => (
            <li key={r.id} style={{ margin: 0 }}>
              {`${clock(r.start, viewing)}${regionStamp(r.tz_id, r.start)} ${joinParts([r.title, r.domain])}`}
            </li>
          ))}
        </ol>
      )}
    </li>
  );
}

function round1(n: number): number {
  return Math.round(n * 10) / 10;
}

/** 件数の題。**位置の記録は「場所」**（D23（仮））—— ソースの表示名（`携帯端末の位置`）は出さない。 */
function countTitle(count: StayDetailCount): string {
  return count.logical_source === "c01-location" ? m.locationTitle : count.display_name;
}

function StayDetailView({ entry, viewing, detail, records, onAction, scheme }: { entry: DayEntry; viewing: string; detail: Load<StayDetail> | null; records: Load<DayRecord[]> | null; onAction: (path: string, body: Record<string, unknown>, entryKey: string) => void; scheme: Scheme }): React.ReactElement {
  const [confirming, setConfirming] = useState(false);
  const counts = detail?.at === "ok" ? detail.value.counts : [];
  const c: SchemeColors = SCHEMES[scheme];
  return (
    <div data-testid="stay-detail" style={{ marginTop: 8, padding: `${SPACE.rowY}px ${SPACE.rowX}px`, background: tone(c.surface2), borderRadius: RADIUS.row }}>
      <RecordsView load={records} viewing={viewing} scheme={scheme} />
      {detail?.at === "loading" && <p>{m.detailLoading}</p>}
      {detail?.at === "failed" && <p role="alert">{m.detailFailed(detail.why)}</p>}
      {detail?.at === "ok" && <div>{counts.map((count) => <p key={count.logical_source} style={{ margin: 0 }}>{m.detailCount(countTitle(count), count.count)}</p>)}</div>}
      {detail?.at === "ok" && !confirming ? (
        <button type="button" onClick={() => setConfirming(true)} style={{ ...destructiveStyle(c), marginTop: 8 }}>
          {m.erase}
        </button>
      ) : detail?.at === "ok" && confirming ? (
        <div data-testid="erase-confirm" style={{ marginTop: 8 }}>
          <p>{m.eraseConfirm(counts.find((count) => count.logical_source === "c01-location")?.count ?? 0)}</p>
          <button type="button" onClick={() => setConfirming(false)} style={control(c)}>{common.cancel}</button>
          <button type="button" onClick={() => onAction("/api/stays/erase", { stay_id: entry.id }, entry.id ?? "")} style={{ ...destructiveStyle(c), marginLeft: SPACE.targetGap }}>{m.eraseYes}</button>
        </div>
      ) : null}
    </div>
  );
}
