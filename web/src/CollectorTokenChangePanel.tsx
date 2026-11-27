// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { useState } from "react";
import { buildSetupString } from "./collectorSetup";
import { msg } from "./messages";
import { FOCUS_ATTR } from "./scheme";
import { inputStyle, primaryStyle, sectionStyle } from "./surface";
import { font, SPACE, type SchemeColors, TYPE } from "./tokens";

const m = msg("secretChange");

export function CollectorTokenChangePanel({ colors: c }: { colors: SchemeColors }): React.ReactElement {
  const [password, setPassword] = useState("");
  const [value, setValue] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copyNote, setCopyNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (event: React.FormEvent): Promise<void> => {
    event.preventDefault(); if (busy) return;
    const given = password; setPassword(""); setValue(null); setError(null); setCopyNote(null); setBusy(true);
    try {
      const response = await fetch("/api/collector-token", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ password: given }) });
      const body: unknown = await response.json().catch(() => null);
      if (response.ok && typeof body === "object" && body !== null && typeof (body as { token?: unknown }).token === "string") setValue(buildSetupString(`${window.location.origin}/api`, (body as { token: string }).token));
      else setError(errorMessage(response.status, body));
    } catch { setError(m.failed); } finally { setBusy(false); }
  };
  const copy = async (): Promise<void> => { if (value === null) return; try { await navigator.clipboard.writeText(value); setCopyNote(m.copied); } catch { setCopyNote(m.copyFailed); } };
  return <section data-testid="collector-token-change-section" aria-labelledby="collector-token-change-title" style={sectionStyle(c, c.surface1)}>
    <h2 id="collector-token-change-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>{m.tokenTitle}</h2>
    <form onSubmit={(event) => void submit(event)} style={{ display: "grid", gap: 6 }}>
      <label style={{ display: "grid", gap: 2 }}>{m.currentPassword}<input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} {...{ [FOCUS_ATTR]: "" }} style={inputStyle(c)} /></label>
      <button type="submit" disabled={busy} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>{m.change}</button>
    </form>
    {error !== null && <p role="alert" style={{ margin: "8px 0 0" }}>{error}</p>}
    {value !== null && <div style={{ display: "grid", gap: 6, marginTop: 8 }}>
      <p style={{ margin: 0 }}>{m.setupLead}</p>
      <label style={{ display: "grid", gap: 2 }}>{m.valueLabel}<input data-testid="collector-token-change-value" readOnly value={value} style={{ ...inputStyle(c), minWidth: 0 }} /></label>
      <div style={{ display: "flex", flexWrap: "wrap", gap: SPACE.targetGap }}><button type="button" onClick={() => void copy()} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>{m.copy}</button>{copyNote !== null && <span role="status">{copyNote}</span>}</div>
    </div>}
  </section>;
}

function errorMessage(status: number, body: unknown): string {
  const error = typeof body === "object" && body !== null && typeof (body as { error?: unknown }).error === "string" ? (body as { error: string }).error : "";
  if (status === 403 && error === "wrong_password") return m.wrongPassword;
  if (status === 409 && error === "not_changeable") return m.notChangeable;
  if (status === 500 && error === "not_saved") return m.notSavedToken;
  if (status === 429) return m.throttled;
  return m.failed;
}
