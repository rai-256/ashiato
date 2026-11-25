// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { useState } from "react";
import { msg } from "./messages";
import { FOCUS_ATTR } from "./scheme";
import { inputStyle, primaryStyle, sectionStyle } from "./surface";
import { font, type SchemeColors, TYPE } from "./tokens";

const m = msg("secretChange");

export function WebPasswordChangePanel({ colors: c }: { colors: SchemeColors }): React.ReactElement {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [again, setAgain] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: React.FormEvent): Promise<void> => {
    event.preventDefault();
    if (busy) return;
    const values = { password: current, new_password: next, confirmation: again };
    setCurrent(""); setNext(""); setAgain(""); setMessage(null); setError(null); setBusy(true);
    try {
      const response = await fetch("/api/web-password", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(values) });
      const body: unknown = await response.json().catch(() => null);
      if (response.status === 204) setMessage(m.changedPassword);
      else setError(errorMessage(response.status, body));
    } catch { setError(m.failed); }
    finally { setBusy(false); }
  };

  return <section data-testid="web-password-change-section" aria-labelledby="web-password-change-title" style={sectionStyle(c, c.surface1)}>
    <h2 id="web-password-change-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>{m.webTitle}</h2>
    <form onSubmit={(event) => void submit(event)} style={{ display: "grid", gap: 6 }}>
      <label style={{ display: "grid", gap: 2 }}>{m.currentPassword}<input type="password" autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} {...{ [FOCUS_ATTR]: "" }} style={inputStyle(c)} /></label>
      <label style={{ display: "grid", gap: 2 }}>{m.newPassword}<input type="password" autoComplete="new-password" value={next} onChange={(e) => setNext(e.target.value)} {...{ [FOCUS_ATTR]: "" }} style={inputStyle(c)} /></label>
      <label style={{ display: "grid", gap: 2 }}>{m.newPasswordAgain}<input type="password" autoComplete="new-password" value={again} onChange={(e) => setAgain(e.target.value)} {...{ [FOCUS_ATTR]: "" }} style={inputStyle(c)} /></label>
      <button type="submit" disabled={busy} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>{m.change}</button>
    </form>
    {message !== null && <p role="status" style={{ margin: "8px 0 0" }}>{message}</p>}
    {error !== null && <p role="alert" style={{ margin: "8px 0 0" }}>{error}</p>}
  </section>;
}

function errorMessage(status: number, body: unknown): string {
  const error = typeof body === "object" && body !== null && typeof (body as { error?: unknown }).error === "string" ? (body as { error: string }).error : "";
  if (status === 403 && error === "wrong_password") return m.wrongPassword;
  if (status === 409 && error === "not_changeable") return m.notChangeable;
  if (status === 500 && error === "not_saved") return m.notSavedPassword;
  if (status === 429) return m.throttled;
  return ({ too_short: m.tooShort, placeholder: m.placeholder, same_as_api_token: m.sameAsApiToken, same_as_session_key: m.sameAsSessionKey, mismatch: m.mismatch } as Record<string, string>)[error] ?? m.failed;
}
