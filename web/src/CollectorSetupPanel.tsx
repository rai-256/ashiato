// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { useState } from "react";
import { buildSetupString } from "./collectorSetup";
import { msg } from "./messages";
import { FOCUS_ATTR } from "./scheme";
import { inputStyle, primaryStyle, sectionStyle } from "./surface";
import { font, SPACE, TYPE, tone, type SchemeColors } from "./tokens";

const m = msg("collectorSetup");

export function CollectorSetupPanel({ colors: c }: { colors: SchemeColors }): React.ReactElement {
  const [password, setPassword] = useState("");
  const [value, setValue] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copyNote, setCopyNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: React.FormEvent): Promise<void> => {
    event.preventDefault();
    if (busy) return;
    const given = password;
    setPassword("");
    setValue(null);
    setError(null);
    setCopyNote(null);
    setBusy(true);
    try {
      const response = await fetch("/api/collector-setup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password: given }),
      });
      const body: unknown = await response.json().catch(() => null);
      if (response.status === 403 && typeof body === "object" && body !== null && (body as { error?: unknown }).error === "wrong_password") {
        setError(m.wrongPassword);
      } else if (!response.ok || typeof body !== "object" || body === null || typeof (body as { token?: unknown }).token !== "string") {
        setError(m.failed);
      } else {
        setValue(buildSetupString(`${window.location.origin}/api`, (body as { token: string }).token));
      }
    } catch {
      setError(m.failed);
    } finally {
      setBusy(false);
    }
  };

  const copy = async (): Promise<void> => {
    if (value === null) return;
    try {
      await navigator.clipboard.writeText(value);
      setCopyNote(m.copied);
    } catch {
      setCopyNote(m.copyFailed);
    }
  };

  return (
    <section data-testid="collector-setup-section" aria-labelledby="collector-setup-title" style={sectionStyle(c, c.surface1)}>
      <h2 id="collector-setup-title" style={{ font: font(TYPE.body, 600), margin: "0 0 6px" }}>
        {m.sectionTitle}
      </h2>
      <p style={{ margin: "0 0 8px", color: tone(c.text) }}>{m.lead}</p>
      <form onSubmit={(event) => void submit(event)} style={{ display: "grid", gap: 6 }}>
        <label style={{ display: "grid", gap: 2 }}>
          {m.password}
          <input
            type="password"
            autoComplete="off"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            {...{ [FOCUS_ATTR]: "" }}
            style={inputStyle(c)}
          />
        </label>
        <button type="submit" disabled={busy} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>
          {m.show}
        </button>
      </form>
      {error !== null && <p role="alert" style={{ margin: "8px 0 0" }}>{error}</p>}
      {value !== null && (
        <div style={{ display: "grid", gap: 6, marginTop: 8 }}>
          <label style={{ display: "grid", gap: 2 }}>
            {m.valueLabel}
            <input data-testid="collector-setup-value" readOnly value={value} style={{ ...inputStyle(c), minWidth: 0 }} />
          </label>
          <div style={{ display: "flex", flexWrap: "wrap", gap: SPACE.targetGap }}>
            <button type="button" onClick={() => void copy()} {...{ [FOCUS_ATTR]: "" }} style={primaryStyle(c)}>
              {m.copy}
            </button>
            {copyNote !== null && <span role="status">{copyNote}</span>}
          </div>
        </div>
      )}
    </section>
  );
}
