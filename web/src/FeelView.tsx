// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useMemo, useRef, useState } from "react";
import { todayInTz } from "./App";
import { readIngestResponse } from "./attributes";
import {
  atLabel,
  buildFeeling,
  calendarDays,
  countNoteChars,
  ingestFeelingItem,
  isFeelingsView,
  NOTE_MAX_CHARS,
  phaseOf,
  scaleValue,
  signed,
  stayRows,
  valenceWord,
  type BuiltFeeling,
  type DayBucket,
  type Feeling,
  type FeelingsView,
  type Scale,
  type StayRow,
} from "./feelings";
import { Calendar, historyLine, keepFocus, ReminderSetting, ScaleRow, StayRows, type CalendarLoad, type StaysLoad } from "./FeelParts";
import { msg } from "./messages";
import { FOCUS_ATTR, useFocusRule, useScheme } from "./scheme";
import { controlStyle, inputStyle, linkStyle, pageStyle, sectionStyle, titleStyle } from "./surface";
import { clock, dateLabel, isDayView, shiftDay, type DayEntry } from "./stays";
import { font, SCHEMES, SPACE, TYPE, tone } from "./tokens";

const m = msg("feel");

type Load = { at: "loading" } | { at: "ok"; value: FeelingsView } | { at: "failed" };

/** いま選ばれている値（押した値が、読み出した最新より優先する）。 */
interface Pick {
  v: number | null;
  e: number | null;
}

/** 届かなかった 1 件。**送り直しは組み直さずこの原文を送る**（乱数が変わると 2 件になる）。 */
interface Pending {
  /** 対象の鍵。日の記録は "day"、滞在の行は滞在の id。対象ごとに 1 件だけ持つ */
  key: string;
  built: BuiltFeeling;
  v: number;
  note: string;
  at: Date;
  /** 滞在の行で押した 1 件の、時刻の範囲（保存の知らせに出す）。日の記録には無い */
  stayRange?: string;
}

interface Notice {
  bad: boolean;
  text: string;
}

const EMPTY_DAY: DayBucket = { latest: { before: null, after: null }, records: [] };

const without = (all: Record<string, Pending>, key: string): Record<string, Pending> => {
  return Object.fromEntries(Object.entries(all).filter(([k]) => k !== key));
};

/** 空白だけのひとことは「無い」と同じ（原文の組み立てと同じ規則）。 */
const norm = (note: string): string => (note.trim() === "" ? "" : note);

const numberText = (n: number): string => n.toLocaleString("en-US");

/** 暦に無い日付のアドレス。**書く操作は出さない**。 */
export function FeelInvalid({ raw }: { raw: string }): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  return (
    <main data-testid="feel-invalid" data-scheme={scheme} style={pageStyle(c)}>
      <p role="alert">{m.invalidDate(raw)}</p>
      <a href="#/feel" style={linkStyle(c)}>
        {m.toToday}
      </a>
    </main>
  );
}

/**
 * S-3 主観入力の画面（ST17 / design D12）—— **押した瞬間に保存する**。
 *
 * 快–不快を押す・元気さを押す・ひとことを変えて欄を離れる（か「ひとことを残す」）たびに、
 * その時点の値で記録を 1 件積む。ひとことが上限を超えている間は、どの操作でも送らない。
 * `date` が `null` のときは暦の今日（`Asia/Tokyo`。夜中も暦の今日）。
 */
export function FeelView({ date }: { date: string | null }): React.ReactElement {
  const scheme = useScheme();
  useFocusRule(scheme);
  const c = SCHEMES[scheme];
  const [target] = useState(() => date ?? todayInTz(new Date()));
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const [now, setNow] = useState(() => new Date());
  const [tick, setTick] = useState(0);
  const [pick, setPick] = useState<Pick | null>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [pending, setPending] = useState<Record<string, Pending>>({});
  const [historyOpen, setHistoryOpen] = useState(false);
  const [calendar, setCalendar] = useState<CalendarLoad>({ at: "loading" });
  /** 滞在の行で押した値（読み出した最新より優先する） */
  const [stayPicks, setStayPicks] = useState<Record<string, number>>({});
  const days = calendarDays(target);
  /** 最後に送った（送ろうとした）ひとこと。欄を離れたとき、これと違えば送る。`null` = まだ送っていない */
  const sentNote = useRef<string | null>(null);

  useEffect(() => {
    let live = true;
    fetch(`/api/feelings?from=${target}&to=${target}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isFeelingsView(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => {
        if (!live) return;
        setNow(new Date());
        setLoad({ at: "ok", value });
      })
      .catch(() => {
        if (live) setLoad({ at: "failed" });
      });
    return () => {
      live = false;
    };
  }, [target, tick]);

  /** 暦と滞在の行の元（対象の日で終わる 30 日）。日の読み出しとは別に読み、片方の失敗が他方を巻き込まない。 */
  const first = days[0] ?? target;
  useEffect(() => {
    let live = true;
    fetch(`/api/feelings?from=${first}&to=${target}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isFeelingsView(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => live && setCalendar({ at: "ok", days: value.days }))
      .catch(() => live && setCalendar({ at: "failed" }));
    return () => {
      live = false;
    };
  }, [first, target, tick]);

  /** 対象の日の滞在（`kind: "stay"`）。消した・吸収された滞在は `/stays` に出ない。 */
  const [staysRaw, setStaysRaw] = useState<{ at: "loading" } | { at: "ok"; entries: DayEntry[] } | { at: "failed" }>({ at: "loading" });
  useEffect(() => {
    let live = true;
    fetch(`/api/stays?date=${target}`)
      .then(async (res) => {
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isDayView(body)) throw new Error("unexpected_shape");
        return body;
      })
      .then((value) => live && setStaysRaw({ at: "ok", entries: value.entries }))
      .catch(() => live && setStaysRaw({ at: "failed" }));
    return () => {
      live = false;
    };
  }, [target]);

  const stays = useMemo<StaysLoad>(() => {
    if (staysRaw.at !== "ok") return staysRaw;
    if (calendar.at === "loading") return { at: "loading" };
    // 記録の読み出しに失敗しても、滞在の行そのものは出す（押された状態が無いだけ）
    const feelingDays = calendar.at === "ok" ? calendar.days : [];
    const hidden = feelingDays.find((d) => d.date === target)?.hidden_stay_records ?? 0;
    return { at: "ok", rows: stayRows(staysRaw.entries, feelingDays, target), hidden };
  }, [staysRaw, calendar, target]);

  const goto = (to: string): void => {
    window.location.hash = `#/feel/${to}`;
  };

  const title = (
    <div style={{ display: "flex", alignItems: "center", gap: SPACE.rowX, marginBottom: SPACE.underTitle }}>
      <button
        type="button"
        aria-label={m.prevDay}
        data-testid="feel-prev"
        {...{ [FOCUS_ATTR]: "" }}
        style={controlStyle(c)}
        onClick={() => goto(shiftDay(target, -1))}
      >
        {m.prevMark}
      </button>
      <h1 data-testid="feel-title" style={{ ...titleStyle(c), margin: 0, flex: 1, textAlign: "center" }}>
        <time dateTime={target}>{m.title(dateLabel(target))}</time>
      </h1>
      <button
        type="button"
        aria-label={m.nextDay}
        data-testid="feel-next"
        {...{ [FOCUS_ATTR]: "" }}
        style={controlStyle(c)}
        onClick={() => goto(shiftDay(target, 1))}
      >
        {m.nextMark}
      </button>
    </div>
  );

  if (load.at === "loading") {
    return (
      <main data-testid="feel-view" data-scheme={scheme} data-ready="false" style={pageStyle(c)}>
        {title}
        <p data-testid="feel-loading">{m.loading}</p>
      </main>
    );
  }

  const day = (load.at === "ok" ? load.value.days.find((d) => d.date === target)?.day : undefined) ?? EMPTY_DAY;
  const dayTarget = { kind: "day", date: target } as const;
  const phase = phaseOf(dayTarget, now);
  const otherPhase = phase === "before" ? "after" : "before";
  const latest: Feeling | null = day.latest[phase];
  const other: Feeling | null = day.latest[otherPhase];
  const otherValence = scaleValue(other, "valence");

  const curV = pick?.v ?? scaleValue(latest, "valence");
  const curE = pick?.e ?? scaleValue(latest, "energy");
  const note = draft ?? latest?.note ?? "";
  const lastNote = sentNote.current ?? latest?.note ?? "";
  const length = countNoteChars(note);
  const over = length > NOTE_MAX_CHARS;

  /** 組んだ原文を送る。結果は「保存した」「断られた」「届かなかった」の 3 つに分ける（ST19 の D9 と同じ読み方）。 */
  const post = async (p: Pending, restoreNote: string | null): Promise<void> => {
    setNotice(null);
    try {
      const res = await fetch("/api/ingest", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify([ingestFeelingItem(p.built)]),
      });
      const outcome = await readIngestResponse(res);
      if (outcome.at === "accepted") {
        setPending((all) => without(all, p.key));
        const value = `${signed(p.v)} ${valenceWord(p.v)}`;
        setNotice({
          bad: false,
          text: p.stayRange === undefined ? m.saved(value, atLabel(p.at), target) : m.savedStay(value, atLabel(p.at), p.stayRange),
        });
        setTick((t) => t + 1);
        return;
      }
      sentNote.current = restoreNote;
      if (outcome.at === "rejected") {
        setPending((all) => without(all, p.key));
        const kind = outcome.kind;
        setNotice({
          bad: true,
          text:
            kind === "invalid_note"
              ? m.rejection.invalid_note(numberText(NOTE_MAX_CHARS))
              : kind === "invalid_feeling_target"
                ? m.rejection.invalid_feeling_target
                : m.rejectedOther(kind),
        });
        return;
      }
      setPending((all) => ({ ...all, [p.key]: p }));
      setNotice({ bad: true, text: m.unreachable });
    } catch {
      sentNote.current = restoreNote;
      setPending((all) => ({ ...all, [p.key]: p }));
      setNotice({ bad: true, text: m.unreachable });
    }
  };

  /** 新しい操作 = 新しい原文。前の原文は届いていないので 1 件も増えていない。 */
  const submit = (v: number, e: number | null): void => {
    const scales: Scale[] = [{ id: "valence", version: 1, value: v }];
    if (e !== null) scales.push({ id: "energy", version: 1, value: e });
    const at = new Date();
    const built = buildFeeling(dayTarget, scales, note, at);
    const previous = sentNote.current;
    sentNote.current = note;
    void post({ key: "day", built, v, note, at }, previous);
  };

  /** 上限を超えている間は送らない。押した値もひとことも残す（切り詰めない）。 */
  const hold = (v: number | null, e: number | null): void => {
    if (v !== null || e !== null) setPick({ v, e });
    setPending((all) => without(all, "day"));
    setNotice({ bad: true, text: m.held(numberText(NOTE_MAX_CHARS)) });
  };

  /** 滞在の行を押す。送る原文の始まり・終わりは `/stays` の区間の値。ひとことは持たない。 */
  const pressStay = (row: StayRow, v: number): void => {
    const at = new Date();
    const built = buildFeeling({ kind: "stay", stay: row.id, start: row.start, end: row.end }, [{ id: "valence", version: 1, value: v }], null, at);
    setStayPicks((p) => ({ ...p, [row.id]: v }));
    void post({ key: row.id, built, v, note: "", at, stayRange: m.stayRange(clock(row.start, target), clock(row.end, target)) }, sentNote.current);
  };

  const pressValence = (v: number): void => {
    if (over) return hold(v, curE);
    setPick({ v, e: curE });
    submit(v, curE);
  };
  const pressEnergy = (e: number): void => {
    if (over) return hold(curV, e);
    if (curV === null) {
      // 快–不快が無いうちは元気さだけを送らない（画面の状態だけ変える）
      setPick({ v: null, e });
      return;
    }
    setPick({ v: curV, e });
    submit(curV, e);
  };
  /** 欄を離れたとき・「ひとことを残す」を押したとき。最後に送ったひとことと違うときだけ送る。 */
  const keepNote = (): void => {
    if (norm(note) === norm(lastNote)) return;
    if (over) return hold(curV, curE);
    // 快–不快を押していなければ送らずに持っておき、押したときに一緒に送る
    if (curV === null) return;
    submit(curV, curE);
  };

  const sectionFace = c.surface1;
  const records = day.records;
  return (
    <main data-testid="feel-view" data-scheme={scheme} data-ready="true" style={pageStyle(c)}>
      {title}
      <p style={{ margin: `0 0 ${SPACE.rowGap}px`, color: tone(c.muted) }}>{m.valenceQuestion}</p>
      <div role="status" data-testid="feel-notice" style={{ minHeight: 0 }}>
        {load.at === "failed" && <p style={{ margin: "0 0 8px" }}>{m.loadFailed}</p>}
        {notice !== null && (
          <p data-testid={notice.bad ? "feel-notice-bad" : "feel-notice-ok"} style={{ margin: "0 0 8px", fontWeight: 600 }}>
            {notice.text}
            {Object.keys(pending).length > 0 && (
              <button
                type="button"
                data-testid="feel-resend"
                {...{ [FOCUS_ATTR]: "" }}
                style={{ ...controlStyle(c), marginLeft: 8 }}
                onMouseDown={keepFocus}
                onClick={() => {
                  setHistoryOpen(false);
                  void (async () => {
                    for (const p of Object.values(pending)) await post(p, sentNote.current);
                  })();
                }}
              >
                {m.resend}
              </button>
            )}
          </p>
        )}
      </div>

      <section style={sectionStyle(c, sectionFace)}>
        <ScaleRow
          group={m.valenceGroup}
          pressed={curV}
          label={(value) => m.valenceButton(signed(value), valenceWord(value))}
          onPress={pressValence}
          withWords
          testid="valence"
        />
        {other !== null && otherValence !== null && (
          <p data-testid="feel-other" style={{ margin: "8px 0 0", color: tone(c.muted) }}>
            {m.otherSide(m.phaseWord[otherPhase] ?? "", `${signed(otherValence)} ${valenceWord(otherValence)}`)}
          </p>
        )}

        <label htmlFor="feel-note" style={{ display: "block", margin: "12px 0 4px", color: tone(c.muted), font: font(TYPE.caption) }}>
          {m.noteLabel}
        </label>
        <textarea
          id="feel-note"
          data-testid="feel-note"
          rows={2}
          value={note}
          placeholder={m.notePlaceholder}
          aria-describedby={over ? "feel-note-over" : undefined}
          {...{ [FOCUS_ATTR]: "" }}
          style={{ ...inputStyle(c), display: "block", width: "100%", margin: 0, resize: "vertical" }}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={keepNote}
        />
        {over && (
          <p
            id="feel-note-over"
            data-testid="feel-note-over"
            role="status"
            style={{ margin: "4px 0 0", padding: "2px 6px", border: `1px dashed ${tone(c.text)}`, borderRadius: 4, fontWeight: 600 }}
          >
            {m.noteOver(numberText(length), numberText(NOTE_MAX_CHARS))}
          </p>
        )}
        <div style={{ textAlign: "right", margin: "4px 0 0" }}>
          <button
            type="button"
            data-testid="feel-keep-note"
            {...{ [FOCUS_ATTR]: "" }}
            style={controlStyle(c)}
            onMouseDown={keepFocus}
            onClick={keepNote}
          >
            {m.keepNote}
          </button>
        </div>

        <span style={{ display: "block", margin: "8px 0 4px", color: tone(c.muted), font: font(TYPE.caption) }}>{m.energyLabel}</span>
        <ScaleRow
          group={m.energyGroup}
          pressed={curE}
          label={(value) => m.energyButton(signed(value))}
          onPress={pressEnergy}
          withWords={false}
          testid="energy"
        />

        {records.length >= 2 && (
          <div style={{ marginTop: 12 }}>
            <button
              type="button"
              data-testid="feel-history-toggle"
              aria-expanded={historyOpen}
              {...{ [FOCUS_ATTR]: "" }}
              style={{ ...linkStyle(c), background: "transparent", border: "none" }}
              onClick={() => setHistoryOpen((o) => !o)}
            >
              {m.history(records.length)}
            </button>
            {historyOpen && (
              <ul data-testid="feel-history" style={{ margin: "4px 0 0", paddingLeft: 18 }}>
                {records.map((r) => (
                  <li key={r.id} data-testid="feel-history-row">
                    {historyLine(r)}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>
      <StayRows target={target} load={stays} picks={stayPicks} onPress={pressStay} />
      <Calendar dates={days} target={target} load={calendar} onPick={goto} />
      <ReminderSetting date={target} />
    </main>
  );
}
