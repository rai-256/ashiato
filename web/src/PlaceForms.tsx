// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useRef, useState } from "react";
import { UNREACHABLE_MESSAGE, emptyInput, validFromOf, type ClaimInput, type Precision, type SendOutcome } from "./attributes";
import { FOCUS_ATTR } from "./DayView";
import { control } from "./controls";
import { Band } from "./PlaceBand";
import {
  CANDIDATE_TOP,
  DEFAULT_RADIUS_M,
  RADIUS_CHOICES,
  buildPlaceRecord,
  buildRegistration,
  coordLabel,
  durationLabel,
  isCandidatesView,
  outcomeMessage,
  PLACE_ID_TAKEN,
  sendPlaceContainer,
  sendPlaceRecords,
  type BuiltPlaceRecord,
  type Candidate,
  type Place,
  type PlaceField,
  type Registration,
} from "./places";
import { MIN_TARGET_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** 読み出しの状態。**「読み込み中」「失敗」「居た所が無い」を分ける**（spec）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

const PRECISIONS: { value: Precision; label: string }[] = [
  { value: "year", label: "年" },
  { value: "month", label: "年月" },
  { value: "day", label: "年月日" },
  { value: "unknown", label: "分からない" },
];

const focus = { [FOCUS_ATTR]: "" };
const sameCoord = (a: { lat: number; lon: number }, b: { lat: number; lon: number }): boolean => a.lat === b.lat && a.lon === b.lon;

/**
 * 名前の無い居た所（`GET /places/candidates`。D10）を読む。`active` の間、`reloadKey` が変わるたびに読み直す
 * （登録・変更が受理された後に、登録した所が消えて見えるように）。
 */
export function useCandidates(active: boolean, reloadKey: number): Load<Candidate[]> {
  const [state, setState] = useState<Load<Candidate[]>>({ at: "loading" });
  useEffect(() => {
    if (!active) return;
    let alive = true;
    void (async () => {
      try {
        const res = await fetch("/api/places/candidates");
        if (!res.ok) throw new Error(`status_${res.status}`);
        const body: unknown = await res.json();
        if (!isCandidatesView(body)) throw new Error("unexpected_shape");
        if (alive) setState({ at: "ok", value: body.candidates });
      } catch (e: unknown) {
        if (alive) setState({ at: "failed", why: e instanceof Error ? e.message : "unknown" });
      }
    })();
    return () => {
      alive = false;
    };
  }, [active, reloadKey]);
  return state;
}

/** 読み出し中・失敗・居た所が無い、の表示。出すものが無ければ `null`。 */
function CandidatesNote({ state, shown }: { state: Load<Candidate[]>; shown: number }): React.ReactElement | null {
  if (state.at === "loading") return <p data-testid="candidates-loading">読み込み中…</p>;
  if (state.at === "failed") {
    return (
      <p role="alert" data-testid="candidates-failed">
        居た所を読み出せませんでした（{state.why}）。
      </p>
    );
  }
  return shown === 0 ? <p data-testid="candidates-empty">居た所がまだありません。</p> : null;
}

/** 上位 10 件と「残り N か所」。押すと全部出る。 */
function useTop<T>(items: T[]): { shown: T[]; rest: number; showAll: () => void } {
  const [all, setAll] = useState(false);
  const shown = all ? items : items.slice(0, CANDIDATE_TOP);
  return { shown, rest: items.length - shown.length, showAll: () => setAll(true) };
}

/** 「残り N か所」。 */
function RestButton({ rest, onClick, scheme }: { rest: number; onClick: () => void; scheme: Scheme }): React.ReactElement | null {
  return rest > 0 ? (
    <button type="button" {...focus} style={control(scheme)} onClick={onClick}>
      残り {rest} か所
    </button>
  ) : null;
}

/** 送る操作の共通: 送っている間は押せない・受理で閉じて読み直す・断りと届かなかったを分ける（D13）。 */
function useSender(onDone: () => Promise<void>): {
  sending: boolean;
  problem: string | null;
  send: (run: () => Promise<SendOutcome>) => Promise<void>;
} {
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const send = async (run: () => Promise<SendOutcome>): Promise<void> => {
    setSending(true);
    setProblem(null);
    try {
      const outcome = await run();
      if (outcome.at === "accepted") {
        await onDone();
        return;
      }
      // **入力は消さない** —— 断られた入力を捨てると、本人は打ち直しになる
      setProblem(outcomeMessage(outcome));
    } catch {
      setProblem(UNREACHABLE_MESSAGE);
    } finally {
      setSending(false);
    }
  };
  return { sending, problem, send };
}

function Problem({ text }: { text: string | null }): React.ReactElement | null {
  return text === null ? null : (
    <p role="alert" data-testid="place-problem" style={{ fontSize: 14, margin: "0 0 8px" }}>
      {text}
    </p>
  );
}

function Radio({
  name,
  label,
  checked,
  onChange,
}: {
  name: string;
  label: string;
  checked: boolean;
  onChange: () => void;
}): React.ReactElement {
  return (
    <label style={{ display: "inline-flex", alignItems: "center", gap: 4, marginRight: 10, minHeight: MIN_TARGET_PX }}>
      <input type="radio" name={name} checked={checked} {...focus} style={{ minWidth: MIN_TARGET_PX, minHeight: MIN_TARGET_PX }} onChange={onChange} />
      {label}
    </label>
  );
}

function RadiusChoice({ name, value, onChange }: { name: string; value: number; onChange: (m: number) => void }): React.ReactElement {
  return (
    <fieldset style={{ border: "none", padding: 0, margin: "0 0 8px" }}>
      <legend style={{ fontSize: 14 }}>広さ</legend>
      {RADIUS_CHOICES.map((m) => (
        <Radio key={m} name={name} label={`${m} m`} checked={value === m} onChange={() => onChange(m)} />
      ))}
    </fieldset>
  );
}

/** 入力が同じなら同じものを返す。**押し直しで組み直すと乱数が変わり、記録が 2 件になる**（D13）。 */
function useBuilt<T>(): { get: (key: string, make: () => T) => T; forget: () => void } {
  const slot = useRef<{ key: string; value: T } | null>(null);
  return {
    get: (key, make) => {
      if (slot.current === null || slot.current.key !== key) slot.current = { key, value: make() };
      return slot.current.value;
    },
    forget: () => {
      slot.current = null;
    },
  };
}

/**
 * 「場所を足す」のパネル（D13）: 名前の無い居た所（上位 10 件と「残り N か所」）→「名前を付ける」のフォーム。
 * **座標を入れる欄も端末の位置を使う操作も置かない**（居た所から選ぶだけ。Q1）。
 */
export function AddPanel({
  scheme,
  reloadKey,
  onDone,
}: {
  scheme: Scheme;
  reloadKey: number;
  onDone: () => Promise<void>;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const state = useCandidates(true, reloadKey);
  const [chosen, setChosen] = useState<Candidate | null>(null);
  const list = state.at === "ok" ? state.value : [];
  const { shown, rest, showAll } = useTop(list);
  return (
    <div data-testid="place-add-panel" style={{ marginBottom: 8 }}>
      <CandidatesNote state={state} shown={list.length} />
      {shown.map((cand) => (
        <div key={`${cand.lat},${cand.lon}`} data-testid="place-candidate" style={{ background: tone(c.surface1), borderRadius: 8, padding: 8, marginBottom: 8 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: 8 }}>
            <span data-testid="candidate-total">{durationLabel(cand.stays.minutes)}</span>
            <span data-testid="candidate-coord" style={{ fontSize: 14, color: tone(c.muted) }}>
              {coordLabel(cand.lat, cand.lon)}
            </span>
          </div>
          <Band hours={cand.stays.hours} scheme={scheme} />
          <p data-testid="candidate-meta" style={{ margin: "0 0 4px", fontSize: 14, color: tone(c.muted) }}>
            {cand.stays.count} 件 · {cand.stays.first_day} 〜 {cand.stays.last_day}
          </p>
          {chosen !== null && sameCoord(chosen, cand) ? (
            <AddForm
              scheme={scheme}
              candidate={cand}
              onDone={async () => {
                setChosen(null);
                await onDone();
              }}
              onCancel={() => setChosen(null)}
            />
          ) : (
            <button type="button" {...focus} style={control(scheme)} onClick={() => setChosen(cand)}>
              名前を付ける
            </button>
          )}
        </div>
      ))}
      <RestButton rest={rest} onClick={showAll} scheme={scheme} />
    </div>
  );
}

/** 「名前を付ける」のフォーム。 */
function AddForm({
  scheme,
  candidate,
  onDone,
  onCancel,
}: {
  scheme: Scheme;
  candidate: Candidate;
  onDone: () => Promise<void>;
  onCancel: () => void;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const [name, setName] = useState("");
  const [radius, setRadius] = useState(DEFAULT_RADIUS_M);
  const [note, setNote] = useState("");
  const { sending, problem, send } = useSender(onDone);
  // 前に押したときに組んだもの。器の識別子を保ち、変わらない項目は同じ記録を送り直す（D13（仮））
  const last = useRef<Registration | null>(null);
  const blank = name.trim() === "";

  const submit = (): Promise<void> =>
    send(async () => {
      const reg = buildRegistration({ name, lat: candidate.lat, lon: candidate.lon, radius_m: radius, note }, new Date(), last.current);
      last.current = reg;
      const container = await sendPlaceContainer(reg.placeId);
      if (container.at !== "accepted") {
        // 乱数の衝突でしか起きない。同じ識別子を送り直しても通らないので、次の押し直しは組み直す
        if (container.at === "rejected" && container.kind === PLACE_ID_TAKEN) last.current = null;
        return container;
      }
      return sendPlaceRecords(reg.records);
    });

  const field = { ...control(scheme), width: "100%", boxSizing: "border-box" as const };
  return (
    <div data-testid="place-add-form" style={{ background: tone(c.surface2), borderRadius: 8, padding: 8 }}>
      <label style={{ display: "block", marginBottom: 4, fontSize: 14 }}>
        名前
        <input aria-label="名前" value={name} {...focus} style={field} onChange={(e) => setName(e.target.value)} />
      </label>
      {/* 座標は居た所の中心。**書き換えられない**ので入力欄にしない */}
      <p data-testid="place-add-coord" style={{ margin: "0 0 8px", fontSize: 14 }}>
        {coordLabel(candidate.lat, candidate.lon)}
      </p>
      <RadiusChoice name="add-radius" value={radius} onChange={setRadius} />
      <label style={{ display: "block", marginBottom: 8, fontSize: 14 }}>
        補足（任意）
        <input aria-label="補足" value={note} {...focus} style={field} onChange={(e) => setNote(e.target.value)} />
      </label>
      <Problem text={problem} />
      <div style={{ display: "flex", gap: 8 }}>
        {/* **送っている間は押せなくする**（押せると同じ場所が 2 つになる）。**名前が空白だけのうちも送らない**
            （名前だけ断られて、位置と補足の記録が名前の無い器に残る。D13（仮）） */}
        <button type="button" disabled={sending || blank} {...focus} style={control(scheme)} onClick={() => void submit()}>
          登録する
        </button>
        <button type="button" {...focus} style={control(scheme)} onClick={onCancel}>
          やめる
        </button>
      </div>
    </div>
  );
}

export type ChangeKind = "name" | "radius" | "coord";

/** 変える 1 項目のフォームの枠。 */
function ChangeFrame({
  scheme,
  disabled,
  sending,
  problem,
  onSend,
  onCancel,
  children,
}: {
  scheme: Scheme;
  disabled: boolean;
  sending: boolean;
  problem: string | null;
  onSend: () => void;
  onCancel: () => void;
  children: React.ReactNode;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  return (
    <div data-testid="place-change-form" style={{ background: tone(c.surface2), borderRadius: 8, padding: 8, marginTop: 4 }}>
      {children}
      <Problem text={problem} />
      <div style={{ display: "flex", gap: 8 }}>
        <button type="button" disabled={disabled || sending} {...focus} style={control(scheme)} onClick={onSend}>
          変える
        </button>
        <button type="button" {...focus} style={control(scheme)} onClick={onCancel}>
          やめる
        </button>
      </div>
    </div>
  );
}

/**
 * 場所 1 つの「変える」（名前・広さ・座標）。1 回の操作で 1 記録を送る（D2）。
 * 押し直しは同じ原文（入力を変えたら組み直す）。受理で閉じて読み直す。
 */
export function ChangeForm({
  kind,
  place,
  registered,
  scheme,
  reloadKey,
  onDone,
  onCancel,
}: {
  kind: ChangeKind;
  place: Place;
  /** 登録した場所のいまの座標（座標の選択肢に出さない。D14） */
  registered: { lat: number; lon: number }[];
  scheme: Scheme;
  reloadKey: number;
  onDone: () => Promise<void>;
  onCancel: () => void;
}): React.ReactElement {
  const { sending, problem, send } = useSender(onDone);
  const built = useBuilt<BuiltPlaceRecord>();
  const [name, setName] = useState(place.name);
  const [radius, setRadius] = useState(place.radius_m);
  const [pick, setPick] = useState<Candidate | null>(null);
  const [how, setHow] = useState<"fix" | "move" | null>(null);
  const [when, setWhen] = useState<ClaimInput>(() => emptyInput(""));
  const candidates = useCandidates(kind === "coord", reloadKey);
  const options = candidates.at === "ok" ? candidates.value.filter((cand) => !registered.some((r) => sameCoord(r, cand))) : [];
  const { shown, rest, showAll } = useTop(options);

  // **いまと同じ値のうちは送らない**（同じ値の記録が増え、「前の名前: <いまと同じ名前>」が出る）。
  // 名前が空・空白だけのうちも送らない（登録と同じ。サーバに断らせて打ち直させない。R17）
  let spec: PlaceField | null;
  if (kind === "name") spec = name === place.name || name.trim() === "" ? null : { field: "name", name };
  else if (kind === "radius") spec = radius === place.radius_m ? null : { field: "radius", radius_m: radius };
  else if (pick === null || how === null) spec = null;
  else if (how === "fix") spec = { field: "coord", lat: pick.lat, lon: pick.lon, change: "fix", supersedes: place.coord.record_id };
  else spec = { field: "coord", lat: pick.lat, lon: pick.lon, change: "move", valid_from: validFromOf(when) };

  const submit = (): Promise<void> =>
    send(() => {
      if (spec === null) return Promise.resolve({ at: "unreachable" });
      const s = spec;
      const record = built.get(JSON.stringify(s), () => buildPlaceRecord(s, new Date(), crypto.randomUUID(), place.id));
      return sendPlaceRecords([record]);
    });

  const field = { ...control(scheme), width: "100%", boxSizing: "border-box" as const };
  return (
    <ChangeFrame scheme={scheme} disabled={spec === null} sending={sending} problem={problem} onSend={() => void submit()} onCancel={onCancel}>
      {kind === "name" && (
        <label style={{ display: "block", marginBottom: 8, fontSize: 14 }}>
          名前
          <input aria-label="名前" value={name} {...focus} style={field} onChange={(e) => setName(e.target.value)} />
        </label>
      )}
      {kind === "radius" && <RadiusChoice name={`radius-${place.id}`} value={radius} onChange={setRadius} />}
      {kind === "coord" && (
        <>
          <fieldset style={{ border: "none", padding: 0, margin: "0 0 8px" }}>
            <legend style={{ fontSize: 14 }}>新しい座標（名前の無い居た所から）</legend>
            <CandidatesNote state={candidates} shown={options.length} />
            {shown.map((cand) => (
              <label
                key={`${cand.lat},${cand.lon}`}
                data-testid="coord-option"
                style={{ display: "flex", alignItems: "center", gap: 4, minHeight: MIN_TARGET_PX }}
              >
                <input
                  type="radio"
                  name={`coord-${place.id}`}
                  checked={pick !== null && sameCoord(pick, cand)}
                  {...focus}
                  style={{ minWidth: MIN_TARGET_PX, minHeight: MIN_TARGET_PX }}
                  onChange={() => setPick(cand)}
                />
                {coordLabel(cand.lat, cand.lon)} · {durationLabel(cand.stays.minutes)} · 最後に居た日 {cand.stays.last_day}
              </label>
            ))}
            <RestButton rest={rest} onClick={showAll} scheme={scheme} />
          </fieldset>
          {/* **どちらも選ばないうちは送らせない**（FR-49。直すか移ったかの区別を残す） */}
          <fieldset style={{ border: "none", padding: 0, margin: "0 0 8px" }}>
            <legend style={{ fontSize: 14 }}>座標を変える理由</legend>
            <Radio name={`how-${place.id}`} label="前の座標が間違っていた" checked={how === "fix"} onChange={() => setHow("fix")} />
            <Radio name={`how-${place.id}`} label="この場所が移った" checked={how === "move"} onChange={() => setHow("move")} />
          </fieldset>
          {how === "move" && <MoveWhen scheme={scheme} placeName={place.name} placeId={place.id} value={when} onChange={setWhen} />}
        </>
      )}
    </ChangeFrame>
  );
}

/** 「いつから」。**精度を先に選び、選んだ精度の欄だけを出す**（ST19 の「書く」と同じ組み方）。 */
function MoveWhen({
  scheme,
  placeName,
  placeId,
  value,
  onChange,
}: {
  scheme: Scheme;
  placeName: string;
  placeId: string;
  value: ClaimInput;
  onChange: (v: ClaimInput) => void;
}): React.ReactElement {
  return (
    <>
      <fieldset style={{ border: "none", padding: 0, margin: "0 0 4px" }}>
        <legend style={{ fontSize: 14 }}>いつから</legend>
        {PRECISIONS.map((p) => (
          <Radio key={p.value} name={`precision-${placeId}`} label={p.label} checked={value.precision === p.value} onChange={() => onChange({ ...value, precision: p.value })} />
        ))}
      </fieldset>
      <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
        {value.precision !== "unknown" && (
          <input aria-label="いつから（年）" value={value.year} {...focus} style={{ ...control(scheme), width: 90 }} onChange={(e) => onChange({ ...value, year: e.target.value })} />
        )}
        {(value.precision === "month" || value.precision === "day") && (
          <input aria-label="いつから（月）" value={value.month} {...focus} style={{ ...control(scheme), width: 64 }} onChange={(e) => onChange({ ...value, month: e.target.value })} />
        )}
        {value.precision === "day" && (
          <input aria-label="いつから（日）" value={value.day} {...focus} style={{ ...control(scheme), width: 64 }} onChange={(e) => onChange({ ...value, day: e.target.value })} />
        )}
      </div>
      <p data-testid="move-note" style={{ margin: "0 0 8px", fontSize: 14 }}>
        前の座標で居た時間も「{placeName}」のまま
      </p>
    </>
  );
}

/** カードごとの「変える」の入口を、開いている 1 つにまとめる状態。 */
export function useChangeOpen(): { open: ChangeKind | null; toggle: (k: ChangeKind) => void; close: () => void } {
  const [open, setOpen] = useState<ChangeKind | null>(null);
  const toggle = useCallback((k: ChangeKind) => setOpen((o) => (o === k ? null : k)), []);
  const close = useCallback(() => setOpen(null), []);
  return { open, toggle, close };
}
