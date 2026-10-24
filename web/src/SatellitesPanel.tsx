// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useState } from "react";
import { dayLabel, momentLabel } from "./export";
import { msg } from "./messages";
import {
  issueCredential,
  listSatellites,
  registerSatellite,
  removeSatellite,
  revokeCredential,
  type Credential,
  type Outcome,
  type Satellite,
} from "./satellites";
import { FOCUS_ATTR } from "./scheme";
import { controlStyle, destructiveStyle, inputStyle, primaryStyle, sectionStyle } from "./surface";
import { font, SPACE, TYPE, tone, type SchemeColors } from "./tokens";

const m = msg("satellites");

type Load = { at: "loading" } | { at: "ok"; value: Satellite[] } | { at: "failed" };

/** 小節の中に開くもの。**同時に開くのは 1 つ**（design D12）。`reveal` の値は state にだけ持ち、閉じたら捨てる。 */
type Open =
  | { kind: "issue"; satellite: string }
  | { kind: "remove"; satellite: string }
  | { kind: "register" }
  | { kind: "reveal"; satellite: string; satelliteName: string; label: string; value: string };

const ring = { [FOCUS_ATTR]: "" };

/** 断りの理由。資格情報の名前の長さは、衛星の名前と同じ種別（`name_invalid`）で返るので文脈で引き分ける。 */
function reasonOf<T>(out: Outcome<T>, context: "issue" | "other"): string | null {
  if (out.at === "ok") return null;
  if (out.at === "unreachable") return m.unreachable;
  if (out.kind === null) return m.actionFailed;
  return context === "issue" && out.kind === "name_invalid" ? m.labelInvalid : m.refusal[out.kind];
}

/**
 * S-7 設定の「衛星と資格情報」の節（ST29 / design D12）。
 *
 * **発行した値は state にだけ持つ**（`localStorage` / `sessionStorage` / URL に置かない）。閉じるか画面を離れたら消え、
 * 本体にも値は無いので二度と出せない。発行の欄・外すの確認・登録の欄は小節の中にその場で開く。
 */
export function SatellitesPanel({ colors: c }: { colors: SchemeColors }): React.ReactElement {
  const [list, setList] = useState<Load>({ at: "loading" });
  const [open, setOpen] = useState<Open | null>(null);
  const [label, setLabel] = useState("");
  const [name, setName] = useState("");
  const [identifier, setIdentifier] = useState("");
  const [reason, setReason] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [copyNote, setCopyNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async (): Promise<void> => {
    try {
      setList({ at: "ok", value: await listSatellites() });
    } catch {
      // 読み直しの失敗では前の一覧を残す
      setList((prev) => (prev.at === "ok" ? prev : { at: "failed" }));
      setNotice(m.loadFailed);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const begin = (next: Open): void => {
    setOpen(next);
    setLabel("");
    setName("");
    setIdentifier("");
    setReason(null);
    setNotice(null);
    setCopyNote(null);
  };
  const close = (): void => {
    setOpen(null);
    setReason(null);
    setCopyNote(null);
  };

  /** 口を呼び、断られたら理由を返す。通ったら一覧を読み直す。 */
  const run = async <T,>(call: Promise<Outcome<T>>, context: "issue" | "other"): Promise<Outcome<T>> => {
    setBusy(true);
    const out = await call;
    setBusy(false);
    if (out.at === "ok") await reload();
    setReason(reasonOf(out, context));
    return out;
  };

  const issue = async (sat: Satellite): Promise<void> => {
    const out = await run(issueCredential(sat.id, label.trim()), "issue");
    if (out.at === "ok") {
      begin({ kind: "reveal", satellite: sat.id, satelliteName: sat.name, label: out.value.label, value: out.value.value });
    }
  };
  const remove = async (sat: Satellite): Promise<void> => {
    const out = await run(removeSatellite(sat.id), "other");
    if (out.at === "ok") close();
  };
  const register = async (): Promise<void> => {
    const out = await run(registerSatellite(name, identifier), "other");
    if (out.at === "ok") close();
  };
  const revoke = async (sat: Satellite, cred: Credential): Promise<void> => {
    setNotice(null);
    const out = await revokeCredential(sat.id, cred.id);
    if (out.at === "ok") await reload();
    else setNotice(reasonOf(out, "other"));
  };
  const copy = async (value: string): Promise<void> => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyNote(m.copied);
    } catch {
      setCopyNote(m.copyFailed);
    }
  };

  const all = list.at === "ok" ? list.value : [];
  const live = all.filter((s) => s.removed_at === null);
  const gone = all.filter((s) => s.removed_at !== null);
  // 値を見せている間は他の欄を開かない（値を失う前に別の所を押せないように）
  const revealing = open?.kind === "reveal";
  const row = { display: "flex", flexWrap: "wrap", gap: SPACE.targetGap } as const;
  const field = { display: "grid", gap: 2, minWidth: 0 } as const;
  const wrap = { wordBreak: "break-all", overflowWrap: "anywhere" } as const;
  const sub = { ...sectionStyle(c, c.surface2), marginBottom: SPACE.rowGap, minWidth: 0 };
  const why = (
    <p role="alert" data-testid="satellite-reason" style={{ margin: 0, font: font(TYPE.caption), ...wrap }}>
      {reason}
    </p>
  );

  return (
    <section data-testid="satellites-section" aria-labelledby="satellites-title" style={sectionStyle(c, c.surface1)}>
      <h2 id="satellites-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>
        {m.sectionTitle}
      </h2>
      <p data-testid="satellites-lead" style={{ margin: "0 0 8px" }}>
        {m.lead}
      </p>
      {list.at === "loading" && <p>{m.loading}</p>}
      {list.at === "failed" && <p role="alert">{m.loadFailed}</p>}
      {notice !== null && list.at === "ok" && (
        <p role="alert" data-testid="satellites-notice" style={{ margin: "0 0 8px", ...wrap }}>
          {notice}
        </p>
      )}

      {live.map((sat) => (
        <article key={sat.id} data-testid="satellite" data-identifier={sat.identifier} aria-label={sat.name} style={sub}>
          <h3 style={{ font: font(TYPE.body, 600), margin: 0, ...wrap }}>
            {sat.name}
            {sat.builtin && (
              <span data-testid="satellite-builtin" style={{ ...controlStyle(c), marginLeft: 8, display: "inline-block" }}>
                {m.builtin}
              </span>
            )}
          </h3>
          <p style={{ margin: "0 0 6px", font: font(TYPE.caption), color: tone(c.muted), ...wrap }}>{sat.identifier}</p>

          {sat.credentials.length === 0 && <p style={{ margin: "0 0 6px", font: font(TYPE.caption) }}>{m.noCredentials}</p>}
          {sat.credentials.map((cred) => (
            <div
              key={cred.id}
              data-testid="credential"
              style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.targetGap, margin: "0 0 6px" }}
            >
              <div style={{ flex: "1 1 160px", minWidth: 0, ...wrap }}>
                <div>{m.credentialLine(cred.label)}</div>
                <div style={{ font: font(TYPE.caption) }}>
                  {m.credentialMeta(
                    dayLabel(cred.issued_at),
                    cred.last_used_at === null ? null : momentLabel(cred.last_used_at),
                  )}
                </div>
              </div>
              <button type="button" disabled={busy} {...ring} style={controlStyle(c)} onClick={() => void revoke(sat, cred)}>
                {m.revoke}
              </button>
            </div>
          ))}

          <div style={row}>
            <button
              type="button"
              disabled={revealing || busy}
              {...ring}
              style={controlStyle(c)}
              onClick={() => begin({ kind: "issue", satellite: sat.id })}
            >
              {m.issue}
            </button>
            {!sat.builtin && (
              <button
                type="button"
                disabled={revealing || busy}
                {...ring}
                style={destructiveStyle(c)}
                onClick={() => begin({ kind: "remove", satellite: sat.id })}
              >
                {m.remove}
              </button>
            )}
          </div>

          {open?.kind === "issue" && open.satellite === sat.id && (
            <form
              data-testid="satellite-issue-form"
              aria-labelledby={`issue-${sat.id}`}
              style={{ display: "grid", gap: 6, marginTop: 8 }}
              onSubmit={(e) => {
                e.preventDefault();
                if (!busy) void issue(sat);
              }}
            >
              <h4 id={`issue-${sat.id}`} style={{ font: font(TYPE.body, 600), margin: 0, ...wrap }}>
                {m.issueHeading(sat.name)}
              </h4>
              <label style={field}>
                {m.issueLabel}
                <input autoFocus value={label} onChange={(e) => setLabel(e.target.value)} {...ring} style={inputStyle(c)} />
              </label>
              {reason !== null && why}
              <div style={row}>
                <button type="submit" disabled={busy} {...ring} style={primaryStyle(c)}>
                  {m.issueGo}
                </button>
                <button type="button" {...ring} style={controlStyle(c)} onClick={close}>
                  {msg("common").cancel}
                </button>
              </div>
            </form>
          )}

          {open?.kind === "remove" && open.satellite === sat.id && (
            <div role="dialog" aria-label={m.removeHeading(sat.name)} data-testid="satellite-remove-confirm" style={{ marginTop: 8 }}>
              <p style={{ font: font(TYPE.body, 600), margin: "0 0 4px", ...wrap }}>{m.removeHeading(sat.name)}</p>
              <ul style={{ margin: "0 0 8px", paddingLeft: 20, ...wrap }}>
                <li>{m.removeStops(String(sat.credentials.length))}</li>
                <li>{m.removeKeeps}</li>
                <li>{m.removeAgain(sat.identifier)}</li>
              </ul>
              {reason !== null && why}
              <div style={row}>
                <button type="button" disabled={busy} {...ring} style={destructiveStyle(c)} onClick={() => void remove(sat)}>
                  {m.removeGo}
                </button>
                <button type="button" {...ring} style={destructiveStyle(c)} onClick={close}>
                  {msg("common").cancel}
                </button>
              </div>
            </div>
          )}

          {open?.kind === "reveal" && open.satellite === sat.id && (
            <div role="group" aria-label={m.revealOf(open.satelliteName, open.label)} data-testid="satellite-reveal" style={{ marginTop: 8 }}>
              <p data-testid="satellite-reveal-warn" style={{ margin: "0 0 4px", font: font(TYPE.body, 600) }}>
                {m.revealWarn}
              </p>
              <p style={{ margin: "0 0 4px", ...wrap }}>{m.revealOf(open.satelliteName, open.label)}</p>
              <p data-testid="satellite-credential-value" style={{ margin: "0 0 6px", font: font(TYPE.caption), ...wrap }}>
                {open.value}
              </p>
              {copyNote !== null && (
                <p role="status" style={{ margin: "0 0 6px", font: font(TYPE.caption) }}>
                  {copyNote}
                </p>
              )}
              <div style={row}>
                <button type="button" {...ring} style={primaryStyle(c)} onClick={() => void copy(open.value)}>
                  {m.copy}
                </button>
                <button type="button" {...ring} style={controlStyle(c)} onClick={close}>
                  {m.closeReveal}
                </button>
              </div>
            </div>
          )}
        </article>
      ))}

      {list.at === "ok" && (
        <div style={{ ...row, marginTop: 4 }}>
          <button type="button" disabled={revealing || busy} {...ring} style={controlStyle(c)} onClick={() => begin({ kind: "register" })}>
            {m.register}
          </button>
        </div>
      )}
      {open?.kind === "register" && (
        <form
          data-testid="satellite-register-form"
          aria-labelledby="register-title"
          style={{ display: "grid", gap: 6, marginTop: 8 }}
          onSubmit={(e) => {
            e.preventDefault();
            if (!busy) void register();
          }}
        >
          <h4 id="register-title" style={{ font: font(TYPE.body, 600), margin: 0 }}>
            {m.registerHeading}
          </h4>
          <label style={field}>
            {m.registerName}
            <input autoFocus value={name} onChange={(e) => setName(e.target.value)} {...ring} style={inputStyle(c)} />
          </label>
          <label style={field}>
            {m.registerIdentifier}
            <input
              value={identifier}
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              onChange={(e) => setIdentifier(e.target.value)}
              {...ring}
              style={inputStyle(c)}
            />
          </label>
          {reason !== null && why}
          <div style={row}>
            <button type="submit" disabled={busy} {...ring} style={primaryStyle(c)}>
              {m.registerGo}
            </button>
            <button type="button" {...ring} style={controlStyle(c)} onClick={close}>
              {msg("common").cancel}
            </button>
          </div>
        </form>
      )}

      {gone.length > 0 && (
        <div data-testid="satellites-gone" style={{ marginTop: 10, font: font(TYPE.caption), ...wrap }}>
          <h3 style={{ font: font(TYPE.caption, 600), margin: "0 0 4px" }}>{m.goneTitle}</h3>
          <ul style={{ margin: "0 0 4px", paddingLeft: 20 }}>
            {gone.map((s) => (
              <li key={s.id}>{m.goneLine(s.name, s.identifier)}</li>
            ))}
          </ul>
          <p style={{ margin: 0 }}>{m.goneNote}</p>
        </div>
      )}
    </section>
  );
}
