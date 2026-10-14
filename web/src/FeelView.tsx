// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useRef, useState } from "react";
import { todayInTz } from "./App";
import { readIngestResponse } from "./attributes";
import {
  atLabel,
  buildFeeling,
  countNoteChars,
  ingestFeelingItem,
  isFeelingsView,
  NOTE_MAX_CHARS,
  phaseOf,
  scaleValue,
  signed,
  valenceWord,
  type BuiltFeeling,
  type DayBucket,
  type Feeling,
  type FeelingsView,
  type Scale,
} from "./feelings";
import { msg } from "./messages";
import { FOCUS_ATTR, useFocusRule, useScheme } from "./scheme";
import { controlStyle, inputStyle, linkStyle, pageStyle, primaryStyle, sectionStyle, titleStyle } from "./surface";
import { dateLabel, shiftDay } from "./stays";
import { font, MIN_TARGET_PX, SCHEMES, SPACE, TYPE, tone } from "./tokens";

const m = msg("feel");

/** 尺度の並び（-2〜+2）。 */
const LEVELS = [-2, -1, 0, 1, 2] as const;

type Load = { at: "loading" } | { at: "ok"; value: FeelingsView } | { at: "failed" };

/** いま選ばれている値（押した値が、読み出した最新より優先する）。 */
interface Pick {
  v: number | null;
  e: number | null;
}

/** 届かなかった 1 件。**送り直しは組み直さずこの原文を送る**（乱数が変わると 2 件になる）。 */
interface Pending {
  built: BuiltFeeling;
  v: number;
  note: string;
  at: Date;
}

interface Notice {
  bad: boolean;
  text: string;
}

const EMPTY_DAY: DayBucket = { latest: { before: null, after: null }, records: [] };

/** 空白だけのひとことは「無い」と同じ（原文の組み立てと同じ規則）。 */
const norm = (note: string): string => (note.trim() === "" ? "" : note);

const numberText = (n: number): string => n.toLocaleString("en-US");

/**
 * 押しても欄のフォーカスを奪わない。奪うと「欄を離れた」で先に 1 件送られ、その知らせが出て
 * 画面が下へずれ、押したはずのボタンに指が届かない（押し込みと離しが別の要素になって押しが消える）。
 * 押した値はその操作が、ひとことは同じ記録に一緒に載せて送る。
 */
const keepFocus = (e: React.MouseEvent): void => {
  e.preventDefault();
};

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

function ScaleRow({
  group,
  pressed,
  label,
  onPress,
  withWords,
  testid,
}: {
  group: string;
  pressed: number | null;
  label: (value: number) => string;
  onPress: (value: number) => void;
  withWords: boolean;
  testid: string;
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
              minHeight: withWords ? 56 : MIN_TARGET_PX + 8,
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
  const [pending, setPending] = useState<Pending | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
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
        setPending(null);
        setNotice({ bad: false, text: m.saved(`${signed(p.v)} ${valenceWord(p.v)}`, atLabel(p.at), target) });
        setTick((t) => t + 1);
        return;
      }
      sentNote.current = restoreNote;
      if (outcome.at === "rejected") {
        setPending(null);
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
      setPending(p);
      setNotice({ bad: true, text: m.unreachable });
    } catch {
      sentNote.current = restoreNote;
      setPending(p);
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
    void post({ built, v, note, at }, previous);
  };

  /** 上限を超えている間は送らない。押した値もひとことも残す（切り詰めない）。 */
  const hold = (v: number | null, e: number | null): void => {
    if (v !== null || e !== null) setPick({ v, e });
    setPending(null);
    setNotice({ bad: true, text: m.held(numberText(NOTE_MAX_CHARS)) });
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
            {pending !== null && (
              <button
                type="button"
                data-testid="feel-resend"
                {...{ [FOCUS_ATTR]: "" }}
                style={{ ...controlStyle(c), marginLeft: 8 }}
                onMouseDown={keepFocus}
                onClick={() => {
                  setHistoryOpen(false);
                  void post(pending, sentNote.current);
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
              style={linkStyle(c)}
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
    </main>
  );
}

/** 書き直した履歴の 1 行（値・ひとこと・記入の日時・前/後）。読むだけ。 */
function historyLine(r: Feeling): string {
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
