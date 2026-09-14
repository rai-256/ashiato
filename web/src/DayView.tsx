// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import {
  clock,
  criteriaLabel,
  dateLabel,
  duration,
  shiftDay,
  type CriteriaTag,
  type DayEntry,
  type DayView as DayData,
} from "./stays";
import { MIN_TARGET_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** 読み出しの状態。**「読み込み中」「失敗」「滞在が無い」を分ける**（spec「読み出しの失敗と『滞在が無い』を区別する」）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

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
 * S-2 の最小形（ST16 / design D8）—— 1 日の滞在の一覧。
 *
 * 行の形は深掘り Q4 の proto の出力のまま: 見出し＝時刻の範囲 / 添える値＝長さ・始まり – 終わり /
 * 「移動 42 分」/「記録なし 8:20 – 16:40」（**文字で**区別し、左端の線は補助）/ 一覧の上に作った基準。
 */
export function DayView({ date }: { date: string }): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [data, setData] = useState<Load<DayData>>({ at: "loading" });

  useEffect(() => {
    let live = true;
    setData({ at: "loading" });
    fetch(`/api/stays?date=${date}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        return (await res.json()) as DayData;
      })
      .then((value) => live && setData({ at: "ok", value }))
      .catch((e: unknown) => live && setData({ at: "failed", why: e instanceof Error ? e.message : "unknown" }));
    return () => {
      live = false;
    };
  }, [date]);

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
      <style>{focusRule(scheme)}</style>
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
      {data.at === "ok" && <Entries view={data.value} scheme={scheme} />}
    </main>
  );
}

function Entries({ view, scheme }: { view: DayData; scheme: Scheme }): React.ReactElement {
  const c = SCHEMES[scheme];
  const main = view.criteria[0];
  const byId = new Map<number, CriteriaTag>(view.criteria.map((t) => [t.criteria_id, t]));
  const stays = view.entries.filter((e) => e.kind === "stay").length;
  return (
    <>
      {main !== undefined && (
        <p data-testid="criteria" style={{ color: tone(c.muted), margin: "0 0 8px" }}>
          この一覧は {criteriaLabel(main)} で作った
          {view.criteria.length > 1 && `（一部の滞在は ${view.criteria.slice(1).map(criteriaLabel).join("、")} で作った）`}
        </p>
      )}
      {stays === 0 && (
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
}: {
  entry: DayEntry;
  viewing: string;
  scheme: Scheme;
  criteria: CriteriaTag | undefined;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const range = `${clock(entry.start, viewing)} – ${clock(entry.end, viewing)}`;
  const long = duration(entry.start, entry.end);
  const base: React.CSSProperties = {
    borderLeft: `3px ${entry.kind === "no-record" ? "dashed" : "solid"} ${tone(c.muted)}`,
    padding: "6px 10px",
    margin: "0 0 6px",
  };
  if (entry.kind === "stay") {
    return (
      <li data-testid="row-stay" data-kind="stay" style={{ ...base, background: tone(c.surface1), borderRadius: 8 }}>
        <h2 style={{ font: "600 17px/1.4 system-ui, sans-serif", margin: 0 }}>{range}</h2>
        <p style={{ margin: 0, color: tone(c.muted) }}>
          {long} ・ {range}
        </p>
        {criteria !== undefined && (
          <p data-testid="row-criteria" style={{ margin: 0, color: tone(c.muted) }}>
            {criteriaLabel(criteria)} で作った
          </p>
        )}
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
