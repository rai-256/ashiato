// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useRef, useState } from "react";
import {
  dayLabel,
  isExportStatus,
  isSummary,
  MIN_PASSWORD_CHARS,
  momentLabel,
  passwordChars,
  sizeLabel,
  startExport,
  type ExportStatus,
  type Summary,
} from "./export";
import { msg } from "./messages";
import { FOCUS_ATTR, useFocusRule, useScheme } from "./scheme";
import { controlStyle, inputStyle, linkStyle, pageStyle, primaryStyle, sectionStyle, titleStyle } from "./surface";
import { font, SCHEMES, SPACE, TYPE, tone } from "./tokens";

const m = msg("settings");
const common = msg("common");
const LOCALE = msg("locale");

/** 走っている間、状態を読み直す間隔（design D13 の仮）。前の応答が返ってから数える（重ねて読まない）。 */
const POLL_MS = 1000;

type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed" };

async function read<T>(path: string, ok: (v: unknown) => v is T): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`status_${res.status}`);
  const body: unknown = await res.json();
  if (!ok(body)) throw new Error("unexpected_shape");
  return body;
}

const num = (n: number): string => n.toLocaleString(LOCALE);

/**
 * S-7 設定（ST33 / design D13）。いまある節は「全記録の書き出し」だけ。
 *
 * **合言葉は state にだけ持つ**（Q9）。送ったら（失敗でも）空にして入力ごと外す。
 * URL・`localStorage`・`sessionStorage` に書かない。ダウンロード・保存・`file://` のリンクは置かない（Q1）。
 */
export function SettingsView(): React.ReactElement {
  const scheme = useScheme();
  useFocusRule(scheme);
  const c = SCHEMES[scheme];
  const [summary, setSummary] = useState<Load<Summary>>({ at: "loading" });
  const [status, setStatus] = useState<Load<ExportStatus>>({ at: "loading" });
  const [statusStale, setStatusStale] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [pw, setPw] = useState("");
  const [again, setAgain] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const pwRef = useRef<HTMLInputElement>(null);

  const loadStatus = useCallback(async (): Promise<void> => {
    try {
      setStatus({ at: "ok", value: await read("/api/export", isExportStatus) });
      setStatusStale(false);
    } catch {
      // 読み直しの失敗では前の値を残す（running が消えて読み直しが止まらないように）
      setStatus((prev) => (prev.at === "ok" ? prev : { at: "failed" }));
      setStatusStale(true);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
    read("/api/export/summary", isSummary).then(
      (v) => setSummary({ at: "ok", value: v }),
      () => setSummary({ at: "failed" }),
    );
  }, [loadStatus]);

  const st = status.at === "ok" ? status.value : null;
  // 無い（`undefined`）と `null` は同じ「無い」
  const runningView = st?.running ?? undefined;
  const last = st?.last ?? undefined;
  const failure = st?.last_failure ?? undefined;
  const interrupted = st?.interrupted ?? undefined;
  const running = runningView !== undefined;
  useEffect(() => {
    if (!running) return;
    // `setInterval` は前の応答を待たずに重ねて読む（遅い応答が DB の接続を奪い合う。R1）。
    // 応答が返ってから次を予約する連鎖にし、画面を離れるか running が消えたら止める。
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async (): Promise<void> => {
      await loadStatus();
      if (!stopped) timer = setTimeout(() => void tick(), POLL_MS);
    };
    timer = setTimeout(() => void tick(), POLL_MS);
    return () => {
      stopped = true;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [running, loadStatus]);

  useEffect(() => {
    if (confirming) pwRef.current?.focus();
  }, [confirming]);

  const close = (): void => {
    setConfirming(false);
    setPw("");
    setAgain("");
  };

  const chars = passwordChars(pw);
  const reason =
    chars < MIN_PASSWORD_CHARS ? m.tooShort(MIN_PASSWORD_CHARS) : pw !== again ? m.mismatch : null;

  const submit = async (e: React.FormEvent): Promise<void> => {
    e.preventDefault();
    if (reason !== null || sending) return;
    const password = pw;
    setSending(true);
    close();
    const out = await startExport(password);
    setSending(false);
    if (out.at === "started") {
      setRefusal(null);
    } else if (out.at === "unreachable") {
      setRefusal(m.refused.unreachable);
    } else {
      setRefusal(
        out.status === 409
          ? m.refused.export_running
          : out.error === "password_too_short"
            ? m.refused.password_too_short
            : m.refused.other,
      );
    }
    await loadStatus();
  };

  const sum = summary.at === "ok" ? summary.value : null;
  const lines: { key: string; text: string; role?: string }[] = [];
  if (failure !== undefined) lines.push({ key: "failure", text: m.failure(failure.kind) });
  if (interrupted !== undefined) {
    const at = momentLabel(interrupted.started_at);
    const file = interrupted.file ?? undefined;
    lines.push({ key: "interrupted", text: file !== undefined ? m.interruptedFile(at, file) : m.interrupted(at) });
  }
  for (const f of st?.leftover ?? []) lines.push({ key: `leftover-${f}`, text: m.leftover(f) });

  return (
    <main data-testid="settings-view" data-scheme={scheme} style={pageStyle(c)}>
      <header style={{ display: "flex", flexWrap: "wrap", alignItems: "baseline", gap: SPACE.targetGap, marginBottom: SPACE.underTools }}>
        <h1 style={{ ...titleStyle(c), margin: 0, marginRight: "auto" }}>{m.title}</h1>
        <a href="#/" {...{ [FOCUS_ATTR]: "" }} style={linkStyle(c)}>
          {common.toCoverage}
        </a>
      </header>

      <section data-testid="export-section" aria-labelledby="export-title" style={sectionStyle(c, c.surface1)}>
        <h2 id="export-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>
          {m.sectionTitle}
        </h2>
        <p style={{ margin: "0 0 8px" }}>{m.lead}</p>

        {summary.at === "loading" && <p>{common.loading}</p>}
        {summary.at === "failed" && <p role="alert">{m.summaryFailed}</p>}
        {sum !== null && (
          <ul data-testid="export-sources" style={{ margin: "0 0 8px", paddingLeft: 20 }}>
            {sum.sources.map((s) => (
              <li key={s.logical_source}>{m.sourceRow(s.display_name, num(s.count), dayLabel(s.first), dayLabel(s.last))}</li>
            ))}
          </ul>
        )}

        <h3 style={{ font: font(TYPE.caption, 600), margin: "0 0 4px" }}>{m.excludedTitle}</h3>
        <ul data-testid="export-excluded" style={{ margin: "0 0 8px", paddingLeft: 20, font: font(TYPE.caption) }}>
          {m.excluded.map((t) => (
            <li key={t}>{t}</li>
          ))}
        </ul>

        {runningView !== undefined && (
          <p data-testid="export-progress" role="status">
            {m.running(num(runningView.written), num(runningView.total))}
          </p>
        )}
        {(status.at === "failed" || statusStale) && <p role="alert">{m.statusFailed}</p>}
        {refusal !== null && (
          <p data-testid="export-refusal" role="alert" style={{ wordBreak: "break-all" }}>
            {refusal}
          </p>
        )}
        {/* 失敗・途中で止まった・消せなかったファイルは「書き出しの節に」1 行ずつ（spec / D13） */}
        {lines.map((l) => (
          <p key={l.key} data-testid="export-line" style={{ margin: "0 0 4px", wordBreak: "break-all", color: tone(c.text) }}>
            {l.text}
          </p>
        ))}

        {!confirming && (
          <button
            type="button"
            data-testid="export-start"
            disabled={running || sum === null}
            {...{ [FOCUS_ATTR]: "" }}
            style={primaryStyle(c)}
            onClick={() => {
              setRefusal(null);
              setConfirming(true);
            }}
          >
            {m.start}
          </button>
        )}

        {confirming && sum !== null && (
          <div role="dialog" aria-label={m.confirmTitle} data-testid="export-confirm" style={{ marginTop: 8 }}>
            <p style={{ margin: "0 0 4px" }}>
              {m.confirmEstimate(num(sum.estimate.rows), sizeLabel(sum.estimate.bytes), num(sum.estimate.seconds))}
            </p>
            <p style={{ margin: "0 0 4px" }}>{m.confirmPassword}</p>
            {last !== undefined && <p style={{ margin: "0 0 4px" }}>{m.confirmReplace}</p>}
            <form onSubmit={(e) => void submit(e)} style={{ display: "grid", gap: 6 }}>
              <label style={{ display: "grid", gap: 2 }}>
                {m.password}
                <input
                  ref={pwRef}
                  type="password"
                  autoComplete="off"
                  value={pw}
                  onChange={(e) => setPw(e.target.value)}
                  {...{ [FOCUS_ATTR]: "" }}
                  style={inputStyle(c)}
                />
              </label>
              <label style={{ display: "grid", gap: 2 }}>
                {m.again}
                <input
                  type="password"
                  autoComplete="off"
                  value={again}
                  onChange={(e) => setAgain(e.target.value)}
                  {...{ [FOCUS_ATTR]: "" }}
                  style={inputStyle(c)}
                />
              </label>
              <p aria-live="polite" data-testid="export-reason" style={{ margin: 0, font: font(TYPE.caption) }}>
                {reason ?? m.ready}
              </p>
              <div style={{ display: "flex", gap: SPACE.targetGap }}>
                <button type="submit" disabled={reason !== null || sending} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>
                  {m.confirmGo}
                </button>
                <button type="button" onClick={close} {...{ [FOCUS_ATTR]: "" }} style={controlStyle(c)}>
                  {common.cancel}
                </button>
              </div>
            </form>
          </div>
        )}
      </section>

      <section data-testid="export-last" aria-labelledby="last-title" style={sectionStyle(c, c.surface1)}>
        <h2 id="last-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>
          {m.lastTitle}
        </h2>
        {st !== null && last === undefined && <p style={{ margin: 0 }}>{m.lastNone}</p>}
        {last !== undefined && (
          <>
            <p style={{ margin: "0 0 4px" }}>
              {m.last(momentLabel(last.finished_at), num(last.rows), sizeLabel(last.bytes))}
            </p>
            <p style={{ margin: "0 0 4px", wordBreak: "break-all" }}>{m.lastFile(last.file)}</p>
            {last.erased_after >= 1 && (
              <p role="note" data-testid="export-erased-after" style={{ margin: "0 0 4px" }}>
                {m.erasedAfter(num(last.erased_after))}
              </p>
            )}
          </>
        )}
      </section>

      <p style={{ font: font(TYPE.caption), color: tone(c.muted) }}>{m.next}</p>
    </main>
  );
}
