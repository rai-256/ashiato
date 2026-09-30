// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { MIN_TARGET_PX, SURFACE, TEXT, tone } from "./tokens";

/**
 * 画面のログイン（ST28 / design D10）。
 *
 * 起動時に `GET /api/session`。401 なら合言葉の入力欄、通れば中身。**合言葉は state から出さず、
 * 送ったら空にする**（ブラウザにも `localStorage` にも置かない）。印は `HttpOnly` の cookie で、画面のコードは読めない。
 * 中身のどの読み出しが 401 を返しても入力欄へ戻す —— `fetch` を 1 箇所で見張るので、各面は書き換えない。
 */
type State = "checking" | "in" | "out";

const INPUT_FLOOR = { minHeight: 44, fontSize: 24 } as const;

/** `/api/` への読み出しが 401 を返したら `onUnauthorized` を呼ぶ。外すには戻り値を呼ぶ。 */
function watchUnauthorized(onUnauthorized: () => void): () => void {
  const original = window.fetch;
  window.fetch = async (input, init) => {
    const res = await original(input, init);
    const url = typeof input === "string" ? input : input instanceof URL ? input.pathname : input.url;
    const method = (init?.method ?? "GET").toUpperCase();
    // ログインの試み自身の 401 は「違う合言葉」で、戻す先はもう入力欄
    if (res.status === 401 && url.startsWith("/api/") && !(url === "/api/session" && method === "POST")) {
      onUnauthorized();
    }
    return res;
  };
  return () => {
    window.fetch = original;
  };
}

export function Gate({ children }: { children: React.ReactNode }): React.ReactElement {
  const [state, setState] = useState<State>("checking");

  useEffect(() => {
    let live = true;
    const off = watchUnauthorized(() => live && setState("out"));
    fetch("/api/session")
      .then((res) => live && setState(res.ok ? "in" : "out"))
      .catch(() => live && setState("out"));
    return () => {
      live = false;
      off();
    };
  }, []);

  if (state === "checking") return <main data-testid="session-checking" style={ground} />;
  if (state === "out") return <LoginForm onDone={() => setState("in")} />;
  return (
    <>
      <header style={{ ...ground, minHeight: 0, padding: "4px 12px", textAlign: "right" }}>
        <button
          type="button"
          style={{ minHeight: MIN_TARGET_PX, minWidth: MIN_TARGET_PX, font: "400 14px/1.6 system-ui, sans-serif" }}
          onClick={() => {
            void fetch("/api/session", { method: "DELETE" }).finally(() => setState("out"));
          }}
        >
          ログアウト
        </button>
      </header>
      {children}
    </>
  );
}

const ground: React.CSSProperties = {
  background: tone(SURFACE.ground),
  color: tone(TEXT.normal),
  minHeight: "100vh",
};

function LoginForm({ onDone }: { onDone: () => void }): React.ReactElement {
  const [password, setPassword] = useState("");
  const [refused, setRefused] = useState(false);
  const [sending, setSending] = useState(false);

  const submit = async (e: React.FormEvent): Promise<void> => {
    e.preventDefault();
    if (sending) return;
    setSending(true);
    try {
      const res = await fetch("/api/session", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ password }),
      });
      setPassword("");
      if (res.ok) onDone();
      else setRefused(true);
    } catch {
      setRefused(true);
    } finally {
      setSending(false);
    }
  };

  return (
    <main data-testid="login" style={{ ...ground, padding: 12, font: "400 16px/1.6 system-ui, sans-serif" }}>
      <form onSubmit={(e) => void submit(e)} style={{ display: "grid", gap: 12, maxWidth: 360 }}>
        <label htmlFor="web-password">合言葉</label>
        <input
          id="web-password"
          type="password"
          autoComplete="off"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          style={{ ...INPUT_FLOOR }}
        />
        <button type="submit" disabled={sending} style={{ ...INPUT_FLOOR, minWidth: 44 }}>
          ログイン
        </button>
        {refused && <p role="alert">合言葉が違います。</p>}
      </form>
    </main>
  );
}
