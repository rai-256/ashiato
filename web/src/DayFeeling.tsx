// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { calendarDays, isFeelingsView, scaleValue, signed, valenceWord, type Feeling, type FeelingsDay, type Latest } from "./feelings";
import { msg } from "./messages";
import { FOCUS_ATTR } from "./scheme";
import { linkStyle } from "./surface";
import { font, MIN_TARGET_PX, SCHEMES, SPACE, TYPE, tone, type Scheme } from "./tokens";

const m = msg("feel");

/** 欄の高さ。「書く」の操作対象の最小（NFR-19）に揃える。上限 40 px の内側 */
const HEAD_PX = MIN_TARGET_PX;

/**
 * 1 日の画面（S-2）が読む、その日の気分（`GET /feelings`。design D14 / D20）。
 * **並びの読み出しとは別に読む** —— 片方の失敗が他方を巻き込まない。`date` を持ち、日を移った直後に前の日のものを出さない。
 *
 * 頭の欄はその日の分（`day`）だけで決める。滞在の行は、主観入力の画面の `stayRows` と同じく
 * **その日で終わる `CALENDAR_DAYS` 日（`days`）から滞在の識別子で集める** —— 記録は滞在の始まりの日に数えるので、
 * 前の日から続く滞在の記録は前の日にある。
 */
export type DayFeelingLoad = { at: "loading" } | { at: "ok"; date: string; day: FeelingsDay; days: FeelingsDay[] } | { at: "failed" };

export function useDayFeelings(date: string): DayFeelingLoad {
  const [load, setLoad] = useState<DayFeelingLoad>({ at: "loading" });
  useEffect(() => {
    let live = true;
    setLoad({ at: "loading" });
    const from = calendarDays(date)[0] ?? date;
    fetch(`/api/feelings?from=${from}&to=${date}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isFeelingsView(body)) throw new Error("unexpected_shape");
        const day = body.days.find((d) => d.date === date);
        if (day === undefined) throw new Error("unexpected_shape");
        return { day, days: body.days };
      })
      .then(({ day, days }) => live && setLoad({ at: "ok", date, day, days }))
      .catch(() => live && setLoad({ at: "failed" }));
    return () => {
      live = false;
    };
  }, [date]);
  // 日を移った直後（読み出しが戻る前）は前の日のものを使わない
  return load.at === "ok" && load.date !== date ? { at: "loading" } : load;
}

/** 最新の記録の快–不快（後があれば後、無ければ前。design D13）を「+1 快」の形に。快–不快の無い記録・記録の無いときは `null`。 */
function latestValence(latest: Latest): string | null {
  const v = scaleValue(latest.after ?? latest.before, "valence");
  return v === null ? null : m.dayValue(signed(v), valenceWord(v));
}

/**
 * 滞在の行に添える「気分 -1 不快」。記録の無い滞在には `null`（「未記入」とは出さない）。
 * 記録は読んだ全部の日の滞在の分から識別子で集める（前の日から続く滞在の記録は前の日にある。`stayRows` と同じ）。
 */
export function stayFeelingText(load: DayFeelingLoad, stayId: string): string | null {
  if (load.at !== "ok") return null;
  const buckets = load.days.flatMap((d) => d.stays.filter((s) => s.stay === stayId));
  if (buckets.length === 0) return null;
  const newest = (p: "before" | "after"): Feeling | null =>
    buckets
      .map((b) => b.latest[p])
      .filter((f): f is Feeling => f !== null)
      .sort((a, b) => Date.parse(b.written_at) - Date.parse(a.written_at))[0] ?? null;
  const value = latestValence({ before: newest("before"), after: newest("after") });
  return value === null ? null : m.dayLabel(value);
}

/** 頭の欄の文字。 */
function headText(day: FeelingsDay): string {
  switch (day.status) {
    case "before_start":
      return m.dayLabel(m.dayBeforeStart);
    case "future":
      return m.dayLabel(m.dayFuture);
    case "missing":
      return m.dayLabel(m.dayMissing);
    case "recorded": {
      if (day.day.records.length === 0) return m.dayLabel(m.dayStays(day.stays_with_records));
      return m.dayLabel(latestValence(day.day.latest) ?? m.dayNoValence);
    }
  }
}

/**
 * 見出しの下の気分の欄（**1 行・高さ 40 px 以下**。spec / design D14）。
 * 「未記入」は破線の枠と文字（色だけで区別しない）。「使い始める前」「これから」は「未記入」と出さない。
 * 「書く」は `#/feel/<日>`。読み出しに失敗したら失敗を出し、「未記入」とは出さない。
 */
export function DayFeeling({ date, load, scheme }: { date: string; load: DayFeelingLoad; scheme: Scheme }): React.ReactElement {
  const c = SCHEMES[scheme];
  const status = load.at === "ok" ? load.day.status : load.at;
  const missing = status === "missing";
  return (
    <div
      data-testid="day-feeling"
      data-state={status}
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "nowrap",
        gap: SPACE.targetGap,
        boxSizing: "border-box",
        // 高さは「書く」の操作対象（24 px）だけ。枠は線で足さず（高さを増やさない）、内側に描く —— 開いてすぐ見える行を減らさない
        height: HEAD_PX,
        maxHeight: 40,
        overflow: "hidden",
        padding: "0 8px",
        font: font(TYPE.caption),
        color: tone(c.text),
        ...(missing
          ? { outline: `2px dashed ${tone(c.muted)}`, outlineOffset: -2 }
          : { boxShadow: `inset 0 0 0 1px ${tone(c.line)}` }),
        borderRadius: 8,
      }}
    >
      {load.at === "loading" && <span data-testid="day-feeling-text">{m.dayLabel(m.dayLoading)}</span>}
      {load.at === "failed" && (
        <span data-testid="day-feeling-text" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {m.dayFailed}
        </span>
      )}
      {load.at === "ok" && (
        <>
          <span data-testid="day-feeling-text" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {headText(load.day)}
          </span>
          {/* 「使い始める前」の日にも書ける（遡って書く口は S-3 が持つ）。書く操作を出さないのは読み出しに失敗したときだけ */}
          <a href={`#/feel/${date}`} {...{ [FOCUS_ATTR]: "" }} style={{ ...linkStyle(c), flex: "none" }}>
            {m.dayWrite}
          </a>
        </>
      )}
    </div>
  );
}
