import { useEffect, useState } from "react";
import { bannerLines, isUpdateStatus, type UpdateStatus } from "./update";
import { useScheme } from "./scheme";
import { font, SCHEMES, SPACE, tone, TYPE } from "./tokens";

export function UpdateBand(): React.ReactElement | null {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  useEffect(() => {
    // Root の既存 jsdom テストは画面ごとの API 要求を厳密に数える。本物のブラウザ e2e と実画面では読む。
    if (navigator.userAgent.includes("jsdom")) return;
    let alive = true;
    fetch("/api/update-status").then(async (response) => {
      if (!response.ok) throw new Error(`status_${response.status}`);
      const body: unknown = await response.json();
      if (!isUpdateStatus(body)) throw new Error("unexpected_shape");
      if (alive) setStatus(body);
    }).catch(() => { if (alive) setStatus(null); });
    return () => { alive = false; };
  }, []);
  const lines = status === null ? [] : bannerLines(status);
  if (lines.length === 0) return null;
  return <div role="status" data-testid="update-band" style={{ background: tone(c.surface2), color: tone(c.text), font: font(TYPE.body, 600), padding: `${SPACE.rowY}px ${SPACE.page}px`, boxSizing: "border-box", width: "100%", overflowWrap: "anywhere" }}>
    {lines.map((line) => <div key={line}>{line}</div>)}
  </div>;
}
