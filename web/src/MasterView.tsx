// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FOCUS_ATTR, focusRule, useScheme } from "./DayView";
import {
  assertedAtLabel,
  buildClaim,
  emptyInput,
  ingestItem,
  isAttributesView,
  kindRejectionMessage,
  newNonce,
  readIngestResponse,
  readKindResponse,
  rejectionMessage,
  UNREACHABLE_MESSAGE,
  validFromLabel,
  valueLabel,
  type AttributesView,
  type BuiltClaim,
  type Claim,
  type ClaimInput,
  type KindView,
  type Precision,
  type SendOutcome,
} from "./attributes";
import { MIN_TARGET_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** 読み出しの状態。**「読み込み中」「失敗」「まだ書いていない」を分ける**（spec）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

/** 精度の選択肢。**先に選ぶ**（Q2。本人が proto で決めた）。 */
const PRECISIONS: { value: Precision; label: string }[] = [
  { value: "year", label: "年" },
  { value: "month", label: "年月" },
  { value: "day", label: "年月日" },
  { value: "unknown", label: "分からない" },
];

/**
 * S-6 マスタ管理 —— 個人属性（ST19 / design D9）。
 *
 * 構造は本人が proto で決めたとおり（Q2 の逐語。`deep.md`）:
 * 種類ごとのカードに**積んだ主張を常に全部**・「いつから」の新しい順 /
 * **書いた日時は主張を押したときだけ** / 訂正で取り消した主張は畳む /
 * カードごとに「書く」1 つ / 精度を先に選ぶ。
 *
 * **画面が長い（10 年後の量で 4.3 画面）のは本人が承知で選んだ。畳む形に戻さない。**
 */
export function MasterView(): React.ReactElement {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [data, setData] = useState<Load<AttributesView>>({ at: "loading" });
  const [adding, setAdding] = useState(false);
  /**
   * 「積めた」の知らせ（review/code.md R18）。**読み直しの結果で上書きしない** ——
   * 主張はサーバに確かに入っているのに、直後の `GET /attributes` が落ちると
   * 画面は「読み出せませんでした」だけになり、**本人から見て積めたのかがどこにも書いていない**。
   * 分からないまま打ち直すと、乱数も識別子も別なので**2 件目が入る**（畳まれない。深掘り C2）。
   */
  const [stored, setStored] = useState<string | null>(null);

  const load = useCallback(async (): Promise<void> => {
    try {
      const res = await fetch("/api/attributes");
      if (!res.ok) throw new Error(`status_${res.status}`);
      const body: unknown = await res.json();
      if (!isAttributesView(body)) throw new Error("unexpected_shape");
      setData({ at: "ok", value: body });
    } catch (e: unknown) {
      setData({ at: "failed", why: e instanceof Error ? e.message : "unknown" });
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <main
      data-testid="master-view"
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
      <nav style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <a href="#/" {...{ [FOCUS_ATTR]: "" }} style={{ ...link(scheme) }}>
          稼働状況へ
        </a>
      </nav>
      <h1 style={{ font: "600 18px/1.3 system-ui, sans-serif", margin: "0 0 8px" }}>マスタ管理</h1>

      {/*
        **タブは「個人属性」の 1 つだけ**（design D9）。押しても何も無いタブを置かない ——
        人物（ST20）と場所（ST21）が中身とともに足す。
      */}
      <div role="tablist" aria-label="マスタ管理" style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button
          type="button"
          role="tab"
          aria-selected="true"
          {...{ [FOCUS_ATTR]: "" }}
          style={{ ...control(scheme), background: tone(c.surface2) }}
        >
          個人属性
        </button>
      </div>

      {stored !== null && (
        <p role="status" data-testid="master-stored" style={{ fontSize: 14 }}>
          {stored}
        </p>
      )}
      {data.at === "loading" && <p data-testid="master-loading">読み込み中…</p>}
      {/*
        **読み出しの失敗と「まだ書いていない」を混ぜない**（spec）——
        混ぜると、サーバが落ちている間ずっと「属性が 1 つも無い」と読める。
      */}
      {data.at === "failed" && (
        <p role="alert" data-testid="master-failed">
          個人属性を読み出せませんでした（{data.why}）。
        </p>
      )}
      {data.at === "ok" && (
        <>
          {data.value.kinds.map((k) => (
            <KindCard key={k.id} kind={k} scheme={scheme} onChanged={load} onStored={setStored} />
          ))}
          <div style={{ marginTop: 12 }}>
            {adding ? (
              <NameForm
                label="種類を足す"
                scheme={scheme}
                onCancel={() => setAdding(false)}
                onSubmit={async (name) => {
                  const res = await fetch("/api/attributes/kinds", {
                    method: "POST",
                    headers: { "content-type": "application/json" },
                    body: JSON.stringify({ name }),
                  });
                  const outcome = await readKindResponse(res);
                  if (outcome.at !== "accepted") return outcome;
                  setAdding(false);
                  await load();
                  return outcome;
                }}
              />
            ) : (
              <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={() => setAdding(true)}>
                種類を足す
              </button>
            )}
          </div>
        </>
      )}
    </main>
  );
}

/** 種類 1 つぶんのカード（design D9）。 */
function KindCard({
  kind,
  scheme,
  onChanged,
  onStored,
}: {
  kind: KindView;
  scheme: Scheme;
  onChanged: () => Promise<void>;
  /** 「積めた」を画面の上に出す（読み直しが落ちても消えない。review/code.md R18） */
  onStored: (message: string | null) => void;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const [writing, setWriting] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [supOpen, setSupOpen] = useState(false);
  // **「予定かどうか」はサーバが決める**（review/code.md R10）。
  // 画面で `valid_from.date > today` を文字列比較で組み直していたときは、**導出の規則が
  // 2 か所に割れていた** —— D6（仮）の反転条件（年をその年の初めから有効とみなすのをやめる、など）が
  // 満たされたとき、サーバだけ直すと画面の「（予定）」が黙ってずれる。
  // 並べ替えと同じく「画面は受け取ったものを描くだけ」に揃える。
  const upcomingIds = useMemo(() => new Set(kind.upcoming.map((u) => u.id)), [kind.upcoming]);

  return (
    <section
      data-testid="kind-card"
      data-kind={kind.id}
      style={{
        background: tone(c.surface1),
        borderRadius: 15,
        padding: 12,
        marginBottom: 12,
        boxSizing: "border-box",
      }}
    >
      {/* 種類の名前。**押すと名前を変える欄が出る**（既定では出さない —— 書き換えの欄を常に置かない） */}
      {renaming ? (
        <NameForm
          label={`「${kind.name}」の名前を変える`}
          scheme={scheme}
          initial={kind.name}
          onCancel={() => setRenaming(false)}
          onSubmit={async (name) => {
            const res = await fetch(`/api/attributes/kinds/${kind.id}/names`, {
              method: "POST",
              headers: { "content-type": "application/json" },
              body: JSON.stringify({ name }),
            });
            const outcome = await readKindResponse(res);
            if (outcome.at !== "accepted") return outcome;
            setRenaming(false);
            await onChanged();
            return outcome;
          }}
        />
      ) : (
        <button
          type="button"
          {...{ [FOCUS_ATTR]: "" }}
          onClick={() => setRenaming(true)}
          style={{ ...control(scheme), background: "transparent", border: "none", font: "600 17px/1.3 system-ui, sans-serif", padding: 0 }}
        >
          {kind.name}
        </button>
      )}

      {/* いまの値。**「まだ書いていない」と「なし」を分ける**（深掘り C10） */}
      <div data-testid="current-value" style={{ font: "600 20px/1.4 system-ui, sans-serif", marginTop: 4 }}>
        {kind.current === null ? "まだ書いていない" : valueLabel(kind.current.value)}
      </div>
      {kind.current !== null && (
        <div style={{ color: tone(c.muted), fontSize: 14 }}>{validFromLabel(kind.current.valid_from)}</div>
      )}

      {/* 予定（「いつから」が今日より後。深掘り C6） */}
      {kind.upcoming.map((u) => (
        <div key={u.id} data-testid="upcoming" style={{ color: tone(c.muted), fontSize: 14, marginTop: 4 }}>
          {valueLabel(u.value)}（予定）・{validFromLabel(u.valid_from)}
        </div>
      ))}

      {/* **書く入口はカードごとに 1 つ**（Q2）。開いてから「変わった / 間違っていた」を選ぶ */}
      <div style={{ marginTop: 8 }}>
        {writing ? (
          <WriteForm
            kind={kind}
            scheme={scheme}
            onDone={async () => {
              setWriting(false);
              // **受理の事実を先に立てる。** この後の読み直しが落ちても消さない
              onStored(`「${kind.name}」に主張を積みました。`);
              await onChanged();
            }}
            onCancel={() => setWriting(false)}
          />
        ) : (
          <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={() => setWriting(true)}>
            書く
          </button>
        )}
      </div>

      {/*
        **積んだ主張を常に全部**（Q2）。並びはサーバが「いつから」の新しい順で返す ——
        画面は受け取った順に描くだけ（並べ替えの規則が 2 か所に割れない）。
      */}
      <div data-testid="claims" style={{ marginTop: 8 }}>
        {kind.claims.map((claim) => (
          <ClaimRow key={claim.id} claim={claim} scheme={scheme} upcoming={upcomingIds} />
        ))}
      </div>

      {/* **訂正で取り消した主張は畳む**（Q2）。押すと出る */}
      {kind.superseded.length > 0 && (
        <div style={{ marginTop: 4 }}>
          <button
            type="button"
            aria-expanded={supOpen}
            {...{ [FOCUS_ATTR]: "" }}
            style={{ ...control(scheme), background: "transparent", color: tone(c.muted), fontSize: 14 }}
            onClick={() => setSupOpen((v) => !v)}
          >
            訂正で取り消した {kind.superseded.length} 件
          </button>
          {supOpen && (
            <div data-testid="superseded">
              {kind.superseded.map((claim) => (
                <ClaimRow key={claim.id} claim={claim} scheme={scheme} upcoming={upcomingIds} superseded />
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  );
}

/**
 * 主張 1 件の行（design D9 / D13）。
 *
 * **値・「いつから」・補足は常に見え、押すと主張した日時が出る**（Q2 / D13）——
 * 本人が押したときだけにしたのは**主張した日時だけ**なので、補足はそれに合わせない。
 */
function ClaimRow({
  claim,
  scheme,
  upcoming,
  superseded = false,
}: {
  claim: Claim;
  scheme: Scheme;
  /** サーバが「予定」とした主張の識別子（`kind.upcoming`）。**画面では判定し直さない** */
  upcoming: ReadonlySet<string>;
  superseded?: boolean;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const [open, setOpen] = useState(false);
  const future = upcoming.has(claim.id);
  return (
    <button
      type="button"
      data-testid="claim-row"
      aria-expanded={open}
      {...{ [FOCUS_ATTR]: "" }}
      onClick={() => setOpen((v) => !v)}
      style={{
        ...control(scheme),
        display: "block",
        width: "100%",
        textAlign: "left",
        background: "transparent",
        border: "none",
        borderTop: `1px solid ${tone(c.surface2)}`,
        borderRadius: 0,
        padding: "6px 0",
      }}
    >
      <span style={{ display: "block" }}>
        {valueLabel(claim.value)}
        {superseded && <span style={{ color: tone(c.muted), fontSize: 13 }}>（訂正で取り消し）</span>}
        {future && <span style={{ color: tone(c.muted), fontSize: 13 }}>（予定）</span>}
      </span>
      <span style={{ display: "block", color: tone(c.muted), fontSize: 14 }}>{validFromLabel(claim.valid_from)}</span>
      {/* **補足は押さずに見える**（D13（仮）。本人が見た proto で行に常に出ていた） */}
      {claim.note !== null && (
        <span data-testid="claim-note" style={{ display: "block", color: tone(c.muted), fontSize: 14 }}>
          補足: {claim.note}
        </span>
      )}
      {/* **書いた日時は押したときだけ**（Q2）。保存も読み出しも 2 つの時刻を別々に持つ（FR-45） */}
      {open && (
        <span data-testid="asserted-at" style={{ display: "block", color: tone(c.muted), fontSize: 14 }}>
          {assertedAtLabel(claim.asserted_at)}
        </span>
      )}
    </button>
  );
}

/**
 * 「書く」のフォーム（design D9 / spec「個人属性の画面から主張を書く」）。
 *
 * **入力を変えずに押し直したら同じ原文を送る**（spec）—— 組み直すと乱数が変わり、
 * サーバは畳めずに同じ主張が 2 件になる。
 */
function WriteForm({
  kind,
  scheme,
  onDone,
  onCancel,
}: {
  kind: KindView;
  scheme: Scheme;
  onDone: () => Promise<void>;
  onCancel: () => void;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  // **取り消す主張の既定は「主張した日時が最も新しい」**（spec）。
  // 「いつから」の新しい順で並ぶ `claims` の先頭とは限らない
  // **絶対時刻で比べる**（review/code.md R11）。`asserted_at` は地域のずれつきの
  // RFC 3339（`…+09:00`）で、C4 のとおり地域は端末のものなので、本人が移動すれば
  // 主張ごとに違うオフセットが混ざる。**文字列の辞書順は絶対時刻の順と食い違う** ——
  // `2026-09-15T01:00:00+09:00`（= 14 日 16:00Z）と `2026-09-14T20:00:00-05:00`（= 15 日 01:00Z）で
  // 逆になり、**後に書いた主張でないものが「取り消す主張」の既定に選ばれる**。
  const newest = useMemo(
    () =>
      kind.claims.reduce<Claim | null>(
        (best, x) =>
          best === null || Date.parse(x.asserted_at) > Date.parse(best.asserted_at) ? x : best,
        null,
      ),
    [kind.claims],
  );
  const [mode, setMode] = useState<"change" | "fix">("change");
  const [input, setInput] = useState<ClaimInput>(() => emptyInput(kind.id));
  const [isNone, setIsNone] = useState(false);
  const [target, setTarget] = useState<string | null>(newest?.id ?? null);
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  /** 組んだ原文を抱えておく。**入力が同じなら同じものを送る** */
  const built = useRef<{ key: string; claim: BuiltClaim } | null>(null);

  /** 「間違っていた」なのに取り消す主張が決まっていない（選択肢が 0 個のときに起きる）。 */
  const cannotSupersede = mode === "fix" && target === null;

  const current: ClaimInput = {
    ...input,
    value: isNone ? null : input.value,
    supersedes: mode === "fix" ? target : null,
  };
  const key = JSON.stringify(current);

  const submit = async (): Promise<void> => {
    setSending(true);
    setProblem(null);
    // **押し直しで組み直さない**（乱数が変われば 2 件になる）
    if (built.current === null || built.current.key !== key) {
      built.current = {
        key,
        claim: buildClaim(current, new Date(), crypto.randomUUID(), newNonce()),
      };
    }
    try {
      const res = await fetch("/api/ingest", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify([ingestItem(built.current.claim)]),
      });
      const outcome = await readIngestResponse(res);
      if (outcome.at === "accepted") {
        // **受理ならフォームを閉じて読み直す**（spec）
        await onDone();
        return;
      }
      // **入力は消さない**（spec）—— 断られた入力を捨てると、本人は打ち直しになる
      setProblem(outcome.at === "rejected" ? rejectionMessage(outcome.kind) : UNREACHABLE_MESSAGE);
    } catch {
      setProblem(UNREACHABLE_MESSAGE);
    } finally {
      setSending(false);
    }
  };

  const field = { ...control(scheme), width: "100%", boxSizing: "border-box" as const };
  return (
    <div data-testid="write-form" style={{ background: tone(c.surface2), borderRadius: 15, padding: 8 }}>
      {/* **開いてから「変わった / 間違っていた」を選ぶ**（Q2。入口はカードごとに 1 つ） */}
      <fieldset style={{ border: "none", padding: 0, margin: "0 0 8px" }}>
        <legend style={{ fontSize: 14, color: tone(c.muted) }}>何を書くか</legend>
        {(
          [
            ["change", "変わった"],
            ["fix", "前の書き込みが間違っていた"],
          ] as const
        ).map(([v, label]) => (
          <label key={v} style={{ display: "inline-flex", alignItems: "center", gap: 4, marginRight: 12, minHeight: MIN_TARGET_PX }}>
            <input
              type="radio"
              name={`mode-${kind.id}`}
              checked={mode === v}
              {...{ [FOCUS_ATTR]: "" }}
              style={{ minWidth: MIN_TARGET_PX, minHeight: MIN_TARGET_PX }}
              onChange={() => setMode(v)}
            />
            {label}
          </label>
        ))}
      </fieldset>

      {/* **取り消す主張を指して積む**（深掘り C5）。指さずに積むと古い開始が導出に残り続ける */}
      {mode === "fix" && (
        <label style={{ display: "block", marginBottom: 8, fontSize: 14 }}>
          どの主張を取り消すか
          <select
            aria-label="取り消す主張"
            value={target ?? ""}
            {...{ [FOCUS_ATTR]: "" }}
            style={field}
            onChange={(e) => setTarget(e.target.value === "" ? null : e.target.value)}
          >
            {kind.claims.map((x) => (
              <option key={x.id} value={x.id}>
                {valueLabel(x.value)} ／ {validFromLabel(x.valid_from)}
              </option>
            ))}
          </select>
        </label>
      )}

      <label style={{ display: "block", marginBottom: 4, fontSize: 14 }}>
        値
        <input
          aria-label="値"
          value={input.value ?? ""}
          disabled={isNone}
          {...{ [FOCUS_ATTR]: "" }}
          style={field}
          onChange={(e) => setInput({ ...input, value: e.target.value })}
        />
      </label>
      {/* 「なし」（その属性が終わった。深掘り C10）。**値が無いことではない** */}
      <label style={{ display: "inline-flex", alignItems: "center", gap: 4, marginBottom: 8, minHeight: MIN_TARGET_PX }}>
        <input
          type="checkbox"
          checked={isNone}
          {...{ [FOCUS_ATTR]: "" }}
          style={{ minWidth: MIN_TARGET_PX, minHeight: MIN_TARGET_PX }}
          onChange={(e) => setIsNone(e.target.checked)}
        />
        なし（その属性が終わった）
      </label>

      {/* **精度を先に選ぶ**（Q2）。選んだ精度の欄だけを出す */}
      {/*
        **精度の選択肢（年 / 年月 / 年月日）と、日付の欄（年・月・日）は名前を分ける** ——
        同じ「年」だと、読み上げでも検査でもどちらを指しているか決まらない。
      */}
      <fieldset style={{ border: "none", padding: 0, margin: "0 0 4px" }}>
        <legend style={{ fontSize: 14, color: tone(c.muted) }}>いつから</legend>
        {PRECISIONS.map((p) => (
          <label key={p.value} style={{ display: "inline-flex", alignItems: "center", gap: 4, marginRight: 10, minHeight: MIN_TARGET_PX }}>
            <input
              type="radio"
              name={`precision-${kind.id}`}
              checked={input.precision === p.value}
              {...{ [FOCUS_ATTR]: "" }}
              style={{ minWidth: MIN_TARGET_PX, minHeight: MIN_TARGET_PX }}
              onChange={() => setInput({ ...input, precision: p.value })}
            />
            {p.label}
          </label>
        ))}
      </fieldset>
      <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
        {input.precision !== "unknown" && (
          <input aria-label="いつから（年）" value={input.year} {...{ [FOCUS_ATTR]: "" }} style={{ ...control(scheme), width: 90 }} onChange={(e) => setInput({ ...input, year: e.target.value })} />
        )}
        {(input.precision === "month" || input.precision === "day") && (
          <input aria-label="いつから（月）" value={input.month} {...{ [FOCUS_ATTR]: "" }} style={{ ...control(scheme), width: 64 }} onChange={(e) => setInput({ ...input, month: e.target.value })} />
        )}
        {input.precision === "day" && (
          <input aria-label="いつから（日）" value={input.day} {...{ [FOCUS_ATTR]: "" }} style={{ ...control(scheme), width: 64 }} onChange={(e) => setInput({ ...input, day: e.target.value })} />
        )}
      </div>

      <label style={{ display: "block", marginBottom: 8, fontSize: 14 }}>
        補足（任意）
        <input aria-label="補足" value={input.note} {...{ [FOCUS_ATTR]: "" }} style={field} onChange={(e) => setInput({ ...input, note: e.target.value })} />
      </label>

      {cannotSupersede && (
        <p role="alert" data-testid="write-problem" style={{ fontSize: 14, margin: "0 0 8px" }}>
          取り消す主張を選んでください（この種類にはまだ主張がありません）
        </p>
      )}
      {problem !== null && (
        <p role="alert" data-testid="write-problem" style={{ fontSize: 14, margin: "0 0 8px" }}>
          {problem}
        </p>
      )}
      <div style={{ display: "flex", gap: 8 }}>
        {/*
          **送っている間は押せなくする**（spec）—— 押せると同じ主張が 2 件になる。
          **取り消す主張を選べていないまま「間違っていた」で積ませない**（review/code.md R15）——
          `supersedes: null` の普通の主張として受理され、**本人は訂正したつもりで、
          記録には訂正でないものが残る**（主張を持たない種類では選択肢が 0 個になる）。
        */}
        <button
          type="button"
          disabled={sending || cannotSupersede}
          {...{ [FOCUS_ATTR]: "" }}
          style={control(scheme)}
          onClick={() => void submit()}
        >
          積む
        </button>
        <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={onCancel}>
          やめる
        </button>
      </div>
    </div>
  );
}

/** 種類を足す・名前を変える欄（design D7 の 2 つの口）。 */
function NameForm({
  label,
  scheme,
  initial = "",
  onSubmit,
  onCancel,
}: {
  label: string;
  scheme: Scheme;
  initial?: string;
  onSubmit: (name: string) => Promise<SendOutcome>;
  onCancel: () => void;
}): React.ReactElement {
  const [name, setName] = useState(initial);
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  return (
    <div data-testid="name-form" style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
      <label style={{ fontSize: 14 }}>
        {label}
        <input aria-label={label} value={name} {...{ [FOCUS_ATTR]: "" }} style={{ ...control(scheme), marginLeft: 6 }} onChange={(e) => setName(e.target.value)} />
      </label>
      <button
        type="button"
        disabled={sending}
        {...{ [FOCUS_ATTR]: "" }}
        style={control(scheme)}
        onClick={() => {
          setSending(true);
          setProblem(null);
          // **断られたことと届かなかったことを分ける**（review/code.md R9）。
          // 一律の文にしていたときは、401 も 500 も「その名前は重なっています」に化けた
          void onSubmit(name)
            .then((outcome) => {
              if (outcome.at === "rejected") setProblem(kindRejectionMessage(outcome.kind));
              else if (outcome.at === "unreachable") setProblem(UNREACHABLE_MESSAGE);
            })
            .catch(() => setProblem(UNREACHABLE_MESSAGE))
            .finally(() => setSending(false));
        }}
      >
        決める
      </button>
      <button type="button" {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={onCancel}>
        やめる
      </button>
      {problem !== null && (
        <p role="alert" style={{ fontSize: 14, margin: 0, width: "100%" }}>
          {problem}
        </p>
      )}
    </div>
  );
}

/** 操作できるものの見た目。**24 × 24 CSS px 以上**（NFR-19）。色は `tokens.ts` からだけ引く。 */
function control(scheme: Scheme): React.CSSProperties {
  const c = SCHEMES[scheme];
  return {
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
}

function link(scheme: Scheme): React.CSSProperties {
  return {
    ...control(scheme),
    background: "transparent",
    border: "none",
    display: "inline-flex",
    alignItems: "center",
  };
}
