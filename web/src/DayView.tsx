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
import { DESTRUCTIVE_TARGET_PX, MIN_TARGET_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** 読み出しの状態。**「読み込み中」「失敗」「滞在が無い」を分ける**（spec「読み出しの失敗と『滞在が無い』を区別する」）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

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

/** OS の明暗の設定。**取得できないときはダーク**（NFR-17）。 */
export function useScheme(): Scheme {
  const query = (): Scheme =>
    typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  const [scheme, setScheme] = useState<Scheme>(query);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const on = (): void => setScheme(query());
    mq.addEventListener?.("change", on);
    return () => mq.removeEventListener?.("change", on);
  }, []);
  return scheme;
}

/** 操作できるもの（日の移動・日付の指定・画面の行き来）に付ける印。フォーカスの輪郭はこの印に掛ける。 */
export const FOCUS_ATTR = "data-focus-ring";

/** フォーカスの輪郭（NFR-22）。**`:focus-visible` は style 属性に書けない**ので、印に掛ける規則を 1 つ置く。 */
export function focusRule(scheme: Scheme): string {
  return `[${FOCUS_ATTR}]:focus-visible { outline: 3px solid ${tone(SCHEMES[scheme].text)}; outline-offset: 2px; }`;
}

/**
 * 輪郭の規則を文書に当てる。**`<style>` 要素は使わない** —— 画面の CSP（`default-src 'self'`。ST28 / D10）が
 * 要素として埋め込んだ style を止める。CSSOM で足した規則（構成可能スタイルシート）は止められない。
 */
export function useFocusRule(scheme: Scheme): void {
  useEffect(() => {
    // 構成可能スタイルシートを持たない環境（jsdom）では当てない。ブラウザは全部持つ（Chromium / Firefox 101+ / Safari 16.4+）
    if (!("adoptedStyleSheets" in document)) return;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(focusRule(scheme));
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((s) => s !== sheet);
    };
  }, [scheme]);
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
    fetch(`/api/stays?date=${date}`)
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

  const control: React.CSSProperties = {
    minHeight: MIN_TARGET_PX,
    minWidth: MIN_TARGET_PX,
    padding: "4px 10px",
    font: "400 15px/1.6 system-ui, sans-serif",
    color: tone(c.text),
    background: tone(c.surface2),
    border: `1px solid ${tone(c.muted)}`,
    borderRadius: 8,
    boxSizing: "border-box",
  };
  const go = (next: string): void => {
    window.location.hash = `#/day/${next}`;
  };

  return (
    <main
      data-testid="day-view"
      data-scheme={scheme}
      style={{
        background: tone(c.ground),
        color: tone(c.text),
        font: "400 16px/1.65 system-ui, sans-serif",
        minHeight: "100vh",
        maxWidth: "100%",
        overflowX: "hidden",
        padding: 12,
        boxSizing: "border-box",
      }}
    >
      <nav style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <a href="#/" {...{ [FOCUS_ATTR]: "" }} style={{ ...control, background: "transparent", border: "none", display: "inline-flex", alignItems: "center" }}>
          稼働状況へ
        </a>
      </nav>
      <h1 data-testid="day-title" style={{ font: "600 18px/1.3 system-ui, sans-serif", margin: "0 0 8px" }}>
        {dateLabel(date)}の滞在
      </h1>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginBottom: 12 }}>
        <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control} onClick={() => go(shiftDay(date, -1))}>
          前の日
        </button>
        <input
          type="date"
          aria-label="日付を指定"
          {...{ [FOCUS_ATTR]: "" }}
          value={date}
          style={control}
          onChange={(e) => {
            if (/^\d{4}-\d{2}-\d{2}$/.test(e.target.value)) go(e.target.value);
          }}
        />
        <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control} onClick={() => go(shiftDay(date, 1))}>
          次の日
        </button>
      </div>

      {data.at === "loading" && <p data-testid="day-loading">読み込み中…</p>}
      {data.at === "failed" && (
        <p role="alert" data-testid="day-error">
          1 日の並びの読み出しに失敗しました（{data.why}）。滞在や記録が無いのではありません。
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
  const detailGeneration = useRef(0);
  const [actionError, setActionError] = useState<{ entryKey: string; message: string } | null>(null);

  const open = (entry: DayEntry): void => {
    const id = entry.id;
    if (id === undefined) return;
    if (openId === id) {
      detailGeneration.current += 1;
      setOpenId(null);
      setDetail(null);
      return;
    }
    const generation = detailGeneration.current + 1;
    detailGeneration.current = generation;
    setOpenId(id);
    setDetail({ at: "loading" });
    fetch(`/api/stays/detail?stay_id=${encodeURIComponent(id)}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isStayDetail(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => generation === detailGeneration.current && setDetail({ at: "ok", value }))
      .catch((e: unknown) => generation === detailGeneration.current && setDetail({ at: "failed", why: e instanceof Error ? e.message : "unknown" }));
  };

  const action = (path: string, body: Record<string, unknown>, entryKey: string): void => {
    setActionError(null);
    fetch(path, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) })
      .then((res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        setOpenId(null);
        setDetail(null);
        reload();
      })
      .catch(() => setActionError({ entryKey, message: "操作に失敗しました。もう一度お試しください。" }));
  };
  return (
    <>
      {main !== undefined && (
        <p data-testid="criteria" style={{ color: tone(c.muted), margin: "0 0 8px" }}>
          この一覧は {criteriaLabel(main)} で作った
          {view.criteria.length > 1 && `（一部の滞在は ${view.criteria.slice(1).map(criteriaLabel).join("、")} で作った）`}
        </p>
      )}
      {/* **まだ来ていない日に「滞在はありません」と言わない**（R45）—— 事実として読まれる */}
      {stays === 0 && future && (
        <p data-testid="day-future" style={{ margin: "0 0 8px" }}>
          まだ来ていない日です。
        </p>
      )}
      {stays === 0 && !future && (
        <p data-testid="day-empty" style={{ margin: "0 0 8px" }}>
          この日の滞在はありません。
        </p>
      )}
      <ol aria-label="1 日の並び" style={{ listStyle: "none", margin: 0, padding: 0 }}>
        {view.entries.map((e) => (
          <Row
            key={`${e.kind}-${e.start}`}
            entry={e}
            viewing={view.date}
            scheme={scheme}
            criteria={e.criteria_id !== undefined && main !== undefined && e.criteria_id !== main.criteria_id ? byId.get(e.criteria_id) : undefined}
            open={openId === e.id}
            detail={openId === e.id ? detail : null}
            onOpen={() => open(e)}
            onAction={action}
            actionError={actionError}
          />
        ))}
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
  onOpen: () => void;
  onAction: (path: string, body: Record<string, unknown>, entryKey: string) => void;
  actionError: { entryKey: string; message: string } | null;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const range = `${clock(entry.start, viewing)} – ${clock(entry.end, viewing)}`;
  const long = duration(entry.start, entry.end);
  const base: React.CSSProperties = {
    borderLeft: `3px ${entry.kind === "no-record" ? "dashed" : "solid"} ${tone(c.muted)}`,
    padding: "6px 10px",
    margin: "0 0 6px",
  };
  const entryKey = entry.id ?? entry.stay_ids?.join(",") ?? "";
  if (entry.kind === "stay") {
    return (
      <li data-testid="row-stay" data-kind="stay" style={{ ...base, background: tone(c.surface1), borderRadius: 8 }}>
        <button
          type="button"
          aria-expanded={open}
          {...{ [FOCUS_ATTR]: "" }}
          onClick={onOpen}
          style={{ display: "block", width: "100%", minHeight: MIN_TARGET_PX, padding: 0, textAlign: "left", color: "inherit", background: "transparent", border: 0 }}
        >
          <h2 style={{ font: "600 17px/1.4 system-ui, sans-serif", margin: 0 }}>{range}</h2>
          <p style={{ margin: 0, color: tone(c.muted) }}>{long} ・ {range}</p>
          {criteria !== undefined && <p data-testid="row-criteria" style={{ margin: 0, color: tone(c.muted) }}>{criteriaLabel(criteria)} で作った</p>}
        </button>
        {open && <StayDetailView entry={entry} detail={detail} onAction={onAction} scheme={scheme} />}
        {actionError?.entryKey === entryKey && <p role="alert">{actionError.message}</p>}
      </li>
    );
  }
  if (entry.kind === "erased") {
    return (
      <li data-testid="row-erased" data-kind="erased" style={{ ...base, color: tone(c.muted) }}>
        消した {range}
        <button type="button" {...{ [FOCUS_ATTR]: "" }} onClick={() => onAction("/api/stays/restore", { stay_ids: entry.stay_ids ?? [] }, entryKey)} style={{ ...controlStyle(c), marginLeft: 8 }}>
          戻す
        </button>
        {actionError?.entryKey === entryKey && <p role="alert">{actionError.message}</p>}
      </li>
    );
  }
  const word = entry.kind === "move" ? "移動" : "記録なし";
  return (
    <li data-testid={`row-${entry.kind}`} data-kind={entry.kind} style={{ ...base, color: tone(c.muted) }}>
      {entry.kind === "move" ? `${word} ${long}` : `${word} ${range}`}
      {entry.kind === "move" && <span> ・ {range}</span>}
    </li>
  );
}

function controlStyle(c: (typeof SCHEMES)[Scheme]): React.CSSProperties {
  return { minHeight: MIN_TARGET_PX, minWidth: MIN_TARGET_PX, padding: "4px 10px", color: tone(c.text), background: tone(c.surface2), border: `1px solid ${tone(c.muted)}`, borderRadius: 8 };
}

function StayDetailView({ entry, detail, onAction, scheme }: { entry: DayEntry; detail: Load<StayDetail> | null; onAction: (path: string, body: Record<string, unknown>, entryKey: string) => void; scheme: Scheme }): React.ReactElement {
  const [confirming, setConfirming] = useState(false);
  const counts = detail?.at === "ok" ? detail.value.counts : [];
  return (
    <div data-testid="stay-detail" style={{ marginTop: 8 }}>
      {detail?.at === "loading" && <p>詳細を読み込み中…</p>}
      {detail?.at === "failed" && <p role="alert">詳細の読み出しに失敗しました（{detail.why}）。</p>}
      {detail?.at === "ok" && <div>{counts.map((count) => <p key={count.logical_source} style={{ margin: 0 }}>{count.display_name} {count.count} 件</p>)}</div>}
      {detail?.at === "ok" && !confirming ? (
        <button type="button" onClick={() => setConfirming(true)} style={{ ...controlStyle(SCHEMES[scheme]), minHeight: DESTRUCTIVE_TARGET_PX, minWidth: DESTRUCTIVE_TARGET_PX, marginTop: 8 }}>
          この滞在を消す
        </button>
      ) : detail?.at === "ok" && confirming ? (
        <div data-testid="erase-confirm" style={{ marginTop: 8 }}>
          <p>この滞在と一緒に消える位置の記録 {counts.find((count) => count.logical_source === "c01-location")?.count ?? 0} 件です。消しますか？</p>
          <button type="button" onClick={() => setConfirming(false)} style={controlStyle(SCHEMES[scheme])}>やめる</button>
          <button type="button" onClick={() => onAction("/api/stays/erase", { stay_id: entry.id }, entry.id ?? "")} style={{ ...controlStyle(SCHEMES[scheme]), minHeight: DESTRUCTIVE_TARGET_PX, minWidth: DESTRUCTIVE_TARGET_PX, marginLeft: 8 }}>消す</button>
        </div>
      ) : null}
    </div>
  );
}
