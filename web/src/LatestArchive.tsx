// SPDX-License-Identifier: AGPL-3.0-only
import type { ArchivesStatus } from "./archives";
import { TEXT, tone } from "./tokens";

/** Must の格子より前に置く、書庫の直近結果。 */
export function LatestArchive({ status }: { status: ArchivesStatus | null }): React.ReactElement {
  const latest = status?.latest_archive;
  return <section data-testid="latest-archive" style={{ maxHeight: 160, overflow: "hidden", border: `1px solid ${tone(TEXT.muted)}`, padding: 8, marginBottom: 12 }}>
    <strong>直近に置いた書庫</strong>
    {latest === undefined || latest === null ? <p>まだ書庫が置かれていません</p> : <p>{latest.file_name} — 入った {latest.inserted} · 既にあった {latest.duplicate} · 読めなかった {latest.unreadable}</p>}
    {status?.pending_shape !== undefined && status.pending_shape !== null && <p>形の確認を待っている書庫が {status.pending_shape.archives} 冊あります</p>}
  </section>;
}
