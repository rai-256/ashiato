import { useEffect, useState } from "react";
import { bannerItems, isUpdateStatus, type UpdateStatus } from "./update";
import { msg } from "./messages";
import { useScheme } from "./scheme";
import { font, SCHEMES, SPACE, tone, TYPE } from "./tokens";

export function UpdateBand(): React.ReactElement | null {
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const m = msg("settings").update;
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [procedure, setProcedure] = useState(false);
  useEffect(() => {
    let alive = true;
    fetch("/api/update-status").then(async (response) => {
      if (!response.ok) throw new Error(`status_${response.status}`);
      const body: unknown = await response.json();
      if (!isUpdateStatus(body)) throw new Error("unexpected_shape");
      if (alive) setStatus(body);
    }).catch(() => { if (alive) setStatus(null); });
    return () => { alive = false; };
  }, []);
  const lines = status === null ? [] : bannerItems(status);
  if (lines.length === 0) return null;
  const available = status !== null && status.latest !== null && (status.state === "available" || status.state === "available_stale");
  const release = status?.latest === "nightly" ? "nightly" : `v${status?.latest ?? ""}`;
  return <div role="status" data-testid="update-band" style={{ background: tone(c.surface2), color: tone(c.text), font: font(TYPE.body, 600), padding: `${SPACE.rowY}px ${SPACE.page}px`, boxSizing: "border-box", width: "100%", overflowWrap: "anywhere" }}>
    {lines.map((line) => line.action === "procedure" ?
      <button key={line.text} type="button" data-testid="update-available" onClick={() => setProcedure(true)} style={{ display: "block", border: 0, padding: 0, background: "transparent", color: "inherit", font: "inherit", textAlign: "left", width: "100%", cursor: "pointer" }}>{line.text}</button> :
      <div key={line.text}>{line.text}</div>)}
    {procedure && available && <section data-testid="update-procedure" aria-label={m.procedureTitle} style={{ marginTop: SPACE.rowY, fontWeight: 400 }}>
      <h2 style={{ font: font(TYPE.body, 600), margin: "0 0 4px" }}>{m.procedureTitle}</h2>
      <p style={{ margin: "0 0 4px" }}>{status.latest}</p>
      <ol style={{ margin: "0 0 4px", paddingLeft: 22 }}>{m.procedureSteps.map((step) => <li key={step}>{step}</li>)}</ol>
      <p style={{ margin: "0 0 4px" }}>{m.rollbackNote}</p>
      <a href={`https://github.com/rai-256/ashiato/releases/tag/${release}`} target="_blank" rel="noreferrer">{m.releaseLink(status.latest!)}</a>
    </section>}
  </div>;
}
