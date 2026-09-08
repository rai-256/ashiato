import { useEffect, useState } from "react";

/** 取り込み口が返す 1 件。形は crates/server/src/ingest.rs が正典。 */
type EventRow = {
  id: string;
  logical_source: string;
  event_time: string;
  tz_id: string;
  origin: string;
};

/** V-01 の骨格。主表現は時系列 × リスト × 滞在（docs/ui-direction.md）。 */
export function App(): React.ReactElement {
  const [rows, setRows] = useState<EventRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch("/api/events")
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then(setRows)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "unknown"));
  }, []);

  if (error !== null) return <p>読み出せませんでした（{error}）</p>;
  return (
    <ul>
      {rows.map((r) => (
        <li key={r.id}>
          {r.event_time} — {r.logical_source}
        </li>
      ))}
    </ul>
  );
}
