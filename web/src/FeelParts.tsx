// SPDX-License-Identifier: AGPL-3.0-only
/**
 * S-3 の部品（ST17 / design D10・D13・D14）—— 快–不快の 5 つのボタン・滞在の行・30 日の暦・通知の時刻。
 * 送る・読む処理は `FeelView.tsx` が持ち、ここは受け取った値を描き、押されたことを返す。
 */
import { useEffect, useState } from "react";
import {
  atLabel,
  isReminder,
  isReminderTime,
  scaleValue,
  signed,
  valenceWord,
  weekdayOf,
  dayMark,
  type Feeling,
  type FeelingsDay,
  type StayRow,
} from "./feelings";
import { msg } from "./messages";
import { FOCUS_ATTR, useScheme } from "./scheme";
import { controlStyle, inputStyle, linkStyle, primaryStyle, sectionStyle } from "./surface";
import { clock } from "./stays";
import { font, MIN_TARGET_PX, SCHEMES, TYPE, tone } from "./tokens";

const m = msg("feel");

/** 尺度の並び（-2〜+2）。 */
const LEVELS = [-2, -1, 0, 1, 2] as const;

/** 滞在の行のボタンの高さ（proto の値。design D13） */
const STAY_BUTTON_PX = 44;

/**
 * 押しても欄のフォーカスを奪わない。奪うと「欄を離れた」で先に 1 件送られ、その知らせが出て
 * 画面が下へずれ、押したはずのボタンに指が届かない（押し込みと離しが別の要素になって押しが消える）。
 * 押した値はその操作が、ひとことは同じ記録に一緒に載せて送る。
 */
export const keepFocus = (e: React.MouseEvent): void => {
  e.preventDefault();
};

export function ScaleRow({
  group,
  pressed,
  label,
  onPress,
  withWords,
  testid,
  height,
}: {
  group: string;
  pressed: number | null;
  label: (value: number) => string;
  onPress: (value: number) => void;
  withWords: boolean;
  testid: string;
  /** 既定は言葉つき 64（NFR-19 の下限 24 とは別に、親指で外さない高さ。深掘り Q1）/ 数値だけ `MIN_TARGET_PX + 8` */
  height?: number;
}): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  return (
    <div role="group" aria-label={group} style={{ display: "flex", gap: 4 }}>
      {LEVELS.map((value) => {
        const on = pressed === value;
        return (
          <button
            key={value}
            type="button"
            data-testid={`${testid}-${value}`}
            aria-pressed={on}
            aria-label={label(value)}
            {...{ [FOCUS_ATTR]: "" }}
            onMouseDown={keepFocus}
            onClick={() => onPress(value)}
            style={{
              ...(on ? primaryStyle(c) : controlStyle(c)),
              flex: 1,
              minWidth: 0,
              minHeight: height ?? (withWords ? 64 : MIN_TARGET_PX + 8),
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              fontWeight: on ? 600 : 400,
              border: on ? `2px solid ${tone(c.accent)}` : `1px solid ${tone(c.muted)}`,
              overflowWrap: "anywhere",
            }}
          >
            <b>{signed(value)}</b>
            {withWords && <span style={{ font: font(TYPE.caption * 0.9) }}>{valenceWord(value)}</span>}
          </button>
        );
      })}
    </div>
  );
}

/** 書き直した履歴の 1 行（値・ひとこと・記入の日時・前/後）。読むだけ。 */
export function historyLine(r: Feeling): string {
  const v = scaleValue(r, "valence");
  const e = scaleValue(r, "energy");
  const parts = [
    v === null ? "" : `${signed(v)} ${valenceWord(v)}`,
    e === null ? "" : m.historyRow.energy(signed(e)),
    r.note === null || r.note === "" ? "" : m.historyRow.note([...r.note].slice(0, 20).join(""), [...r.note].length > 20),
    m.historyRow.writtenAt(atLabel(new Date(r.written_at))),
    m.phaseWord[r.phase] ?? "",
  ];
  return parts.filter((p) => p !== "").join(m.historySeparator);
}

/** 滞在の読み出し。`null` = まだ。 */
export type StaysLoad = { at: "loading" } | { at: "ok"; rows: StayRow[]; hidden: number } | { at: "failed" };

/** 対象の日の滞在ごとの気分（design D13）。押した値は上に渡し、押された状態はそこから受け取る。 */
export function StayRows({
  target,
  load,
  picks,
  onPress,
}: {
  target: string;
  load: StaysLoad;
  /** この画面で押した値（読み出した最新より優先する） */
  picks: Record<string, number>;
  onPress: (row: StayRow, value: number) => void;
}): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [opened, setOpened] = useState<string | null>(null);
  return (
    <section data-testid="feel-stays" style={{ ...sectionStyle(c, c.surface1), marginTop: 12 }}>
      <h2 style={{ margin: "0 0 8px", font: font(TYPE.caption), color: tone(c.muted) }}>{m.staysHeading}</h2>
      {load.at === "loading" && <p data-testid="feel-stays-loading">{m.staysLoading}</p>}
      {load.at === "failed" && (
        <p role="status" data-testid="feel-stays-failed">
          {m.staysFailed}
        </p>
      )}
      {load.at === "ok" && load.rows.length === 0 && <p data-testid="feel-stays-empty">{m.staysNone}</p>}
      {load.at === "ok" && (
        <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
          {load.rows.map((row) => {
            const range = m.stayRange(clock(row.start, target), clock(row.end, target));
            const latest = scaleValue(row.latest, "valence");
            return (
              <li key={row.id} data-testid="feel-stay-row" data-stay={row.id} style={{ margin: "0 0 12px" }}>
                <p style={{ margin: "0 0 4px" }}>
                  <span data-testid="feel-stay-range">{range}</span>
                  {row.carried && (
                    <span data-testid="feel-stay-carried" style={{ marginLeft: 8, color: tone(c.muted) }}>
                      {m.carriedFrom}
                    </span>
                  )}
                </p>
                <ScaleRow
                  group={m.stayGroup(range)}
                  pressed={picks[row.id] ?? latest}
                  label={(value) => m.valenceButton(signed(value), valenceWord(value))}
                  onPress={(value) => onPress(row, value)}
                  withWords={false}
                  testid={`stay-${row.id}`}
                  height={STAY_BUTTON_PX}
                />
                {row.records.length >= 2 && (
                  <div>
                    <button
                      type="button"
                      data-testid="feel-stay-history-toggle"
                      aria-expanded={opened === row.id}
                      {...{ [FOCUS_ATTR]: "" }}
                      style={{ ...linkStyle(c), background: "transparent", border: "none" }}
                      onClick={() => setOpened(opened === row.id ? null : row.id)}
                    >
                      {m.history(row.records.length)}
                    </button>
                    {opened === row.id && (
                      <ul data-testid="feel-stay-history" style={{ margin: "4px 0 0", paddingLeft: 18 }}>
                        {row.records.map((r) => (
                          <li key={r.id} data-testid="feel-stay-history-row">
                            {historyLine(r)}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {load.at === "ok" && load.hidden > 0 && (
        <p data-testid="feel-stays-hidden" style={{ margin: 0, color: tone(c.muted) }}>
          {m.hiddenStays(load.hidden)}
        </p>
      )}
    </section>
  );
}

/** 暦の読み出し。 */
export type CalendarLoad = { at: "loading" } | { at: "ok"; days: FeelingsDay[] } | { at: "failed" };

/**
 * 対象の日で終わる 30 日の暦（日曜始まり）。**状態は読めたときだけ出す** —— 読み出しに失敗したら
 * 日付だけを出し、「未」は付けない（失敗を未記入に見せない）。
 */
export function Calendar({
  dates,
  target,
  load,
  onPick,
}: {
  dates: string[];
  target: string;
  load: CalendarLoad;
  onPick: (date: string) => void;
}): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const byDate = new Map(load.at === "ok" ? load.days.map((d) => [d.date, d]) : []);
  const lead = weekdayOf(dates[0] ?? target);
  return (
    <section data-testid="feel-calendar" style={{ ...sectionStyle(c, c.surface1), marginTop: 12 }}>
      <h2 style={{ margin: "0 0 8px", font: font(TYPE.caption), color: tone(c.muted) }}>{m.calendarHeading}</h2>
      {load.at === "failed" && (
        <p role="status" data-testid="feel-calendar-failed">
          {m.calendarFailed}
        </p>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(7, minmax(0, 1fr))", gap: 4 }}>
        {m.weekdays.map((w) => (
          <span key={w} aria-hidden="true" style={{ textAlign: "center", color: tone(c.muted), font: font(TYPE.caption) }}>
            {w}
          </span>
        ))}
        {Array.from({ length: lead }, (_, i) => (
          <span key={`lead-${i}`} aria-hidden="true" />
        ))}
        {dates.map((date) => {
          const d = byDate.get(date);
          const status = d?.status;
          const latest = d === undefined ? null : (d.day.latest.after ?? d.day.latest.before);
          const mark = status === undefined ? "" : dayMark(status, scaleValue(latest, "valence"));
          const missing = status === "missing";
          const here = date === target;
          return (
            <button
              key={date}
              type="button"
              data-testid={`cal-${date}`}
              data-status={status ?? "unknown"}
              aria-current={here ? "date" : undefined}
              aria-label={m.calendarDay(date, status === undefined ? "" : (m.calendarState[status] ?? ""))}
              {...{ [FOCUS_ATTR]: "" }}
              onClick={() => onPick(date)}
              style={{
                ...(here ? primaryStyle(c) : controlStyle(c)),
                minWidth: 0,
                minHeight: STAY_BUTTON_PX,
                padding: 2,
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                font: font(TYPE.caption),
                border: missing ? `2px dashed ${tone(c.muted)}` : `1px solid ${tone(c.muted)}`,
              }}
            >
              <span>{Number(date.slice(8))}</span>
              <b data-testid="cal-mark">{mark}</b>
            </button>
          );
        })}
      </div>
    </section>
  );
}

type Reminder = { at: "loading" } | { at: "ok"; value: string } | { at: "failed" };

/** 通知の時刻（design D10）。いまの時刻を読み、時:分を変えて保存する。保存できなくても入れた値は消さない。 */
export function ReminderSetting({ date }: { date: string }): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [current, setCurrent] = useState<Reminder>({ at: "loading" });
  const [draft, setDraft] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ bad: boolean; text: string } | null>(null);

  useEffect(() => {
    let live = true;
    fetch(`/api/feelings/reminder?date=${date}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isReminder(body)) throw new Error("unexpected_shape");
        return body.at;
      })
      .then((value) => live && setCurrent({ at: "ok", value }))
      .catch(() => live && setCurrent({ at: "failed" }));
    return () => {
      live = false;
    };
  }, [date]);

  const shown = draft ?? (current.at === "ok" ? current.value : "");

  const save = async (): Promise<void> => {
    setNotice(null);
    try {
      const res = await fetch("/api/feelings/reminder", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ at: shown }),
      });
      if (!res.ok) throw new Error(`status_${res.status}`);
      const body: unknown = await res.json();
      if (!isReminder(body)) throw new Error("unexpected_shape");
      setCurrent({ at: "ok", value: body.at });
      setDraft(null);
      setNotice({ bad: false, text: m.reminderSaved(body.at) });
    } catch {
      setNotice({ bad: true, text: m.reminderNotSaved });
    }
  };

  return (
    <section data-testid="feel-reminder" style={{ ...sectionStyle(c, c.surface1), marginTop: 12 }}>
      <h2 style={{ margin: "0 0 8px", font: font(TYPE.caption), color: tone(c.muted) }}>{m.reminderHeading}</h2>
      <p data-testid="feel-reminder-current" style={{ margin: "0 0 8px" }}>
        {current.at === "ok" ? m.reminderCurrent(current.value) : current.at === "failed" ? m.reminderUnknown : m.loading}
      </p>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <label htmlFor="feel-reminder-at" style={{ color: tone(c.muted), font: font(TYPE.caption) }}>
          {m.reminderLabel}
        </label>
        <input
          id="feel-reminder-at"
          data-testid="feel-reminder-at"
          type="time"
          value={shown}
          {...{ [FOCUS_ATTR]: "" }}
          style={{ ...inputStyle(c), minHeight: STAY_BUTTON_PX }}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button
          type="button"
          data-testid="feel-reminder-save"
          disabled={!isReminderTime(shown)}
          {...{ [FOCUS_ATTR]: "" }}
          style={{ ...controlStyle(c), minHeight: STAY_BUTTON_PX }}
          onClick={() => void save()}
        >
          {m.reminderSave}
        </button>
      </div>
      <div role="status" data-testid="feel-reminder-notice">
        {notice !== null && (
          <p data-testid={notice.bad ? "feel-reminder-bad" : "feel-reminder-ok"} style={{ margin: "8px 0 0", fontWeight: 600 }}>
            {notice.text}
          </p>
        )}
      </div>
    </section>
  );
}
