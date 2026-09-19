// SPDX-License-Identifier: AGPL-3.0-only
import {
  daysAgo,
  INBOX_STALE_DAYS,
  jstMoment,
  unreadableKindLabel,
  type ArchivesStatus,
} from "./archives";
import { TEXT, tone } from "./tokens";

/**
 * **箱の高さの上限**（design D12。**仮**）。
 *
 * 反転条件: 優先の低い行が常に省かれ、本人が見落としたと分かったとき（上げるときは
 * `collection-coverage` の予算の文も合わせて直す）。
 */
export const ARCHIVE_BOX_MAX_PX = 160;

const FONT = "400 14px/1.6 system-ui, sans-serif";
const LINE_PX = 14 * 1.6;
const PAD_PX = 8;
const BORDER_PX = 1;
const MARGIN_BOTTOM_PX = 12;

/**
 * 上限に収まる行数。**宣言している値から数える** —— 定数を 2 か所に書いて
 * 突き合わせると、どちらかを直し忘れても緑のままになる（design D27）。
 *
 * **下の余白も引く** —— 予算を数える `layout.ts` の `declaredHeight` は余白まで
 * 積むので、ここで引かないと「箱は 160 px を超えない」が測り方の側で落ちる。
 * 見出しの 1 行を除いた残りが、本文に使える行数。
 */
export const BOX_MAX_ROWS =
  Math.floor(
    (ARCHIVE_BOX_MAX_PX - 2 * PAD_PX - 2 * BORDER_PX - MARGIN_BOTTOM_PX) / LINE_PX,
  ) - 1;

/** 箱に積む 1 行。`pinned` は**省いてはいけない行**（spec R14）。 */
type Row = { key: string; text: string; pinned?: boolean };

/**
 * 箱に出す行を、優先順（読んでいる途中 → 置き場・取り込み器 → 形の確認待ち →
 * 直近の書庫の結果）に作る。
 *
 * **直近に置いた書庫を読めなかった・格納に失敗したことは省かない** —— 置いたのに
 * 入っていないことに気づけないまま、書庫が約 7 日で失効する。
 */
export function archiveBoxRows(
  status: ArchivesStatus | null,
  now: Date,
  failed = false,
): Row[] {
  const rows: Row[] = [];
  // **「読み出せていない」と「置かれていない」を混ぜない**（ST02 の R19 と同じ型。R12）。
  // 混ぜると、API が落ちているだけのときに「置いた書庫が無い」と断言してしまう。
  if (failed) {
    return [{ key: "failed", text: "書庫の状態を読み出せませんでした（置いた書庫が無いのではありません）" }];
  }
  if (status === null) return [{ key: "loading", text: "書庫の状態を読み込み中…" }];

  const reading = status.reading;
  if (reading !== undefined && reading !== null) {
    rows.push({
      key: "reading",
      text: `読んでいます: ${reading.file_name} ${reading.inner_path} ${reading.items_read.toLocaleString("en-US")} 件まで（${jstMoment(reading.started_at)} から）`,
    });
  }

  const inbox = status.inbox;
  if (inbox !== undefined && inbox !== null && !inbox.capturable) {
    const places: Record<string, string> = {
      dedicated_inbox_unreadable: "専用のフォルダ",
      downloads_unreadable: "ダウンロードのフォルダ",
    };
    const named = inbox.blockers.map((b) => places[b] ?? b);
    rows.push({
      key: "inbox-unreadable",
      text: `${named.length === 0 ? "置き場" : named.join("・")} が読めません`,
    });
  }
  if (inbox === undefined || inbox === null) {
    rows.push({ key: "inbox-never", text: "取り込み器はまだ一度も動いていません" });
  } else {
    const ago = daysAgo(inbox.emitted_at, now);
    if (ago > INBOX_STALE_DAYS) {
      rows.push({ key: "inbox-stale", text: `取り込み器の最後の確認: ${ago} 日前` });
    }
  }

  const pending = status.pending_shape;
  if (pending !== undefined && pending !== null) {
    rows.push({
      key: "pending-shape",
      text: `形の確認を待っている書庫が ${pending.archives} 冊あります（tools/archive-shape.sh で形を見て印を置く）`,
    });
  }

  const latest = status.latest_archive;
  if (latest === undefined || latest === null) {
    rows.push({ key: "empty", text: "まだ書庫が置かれていません" });
    return rows;
  }
  const name = latest.file_name ?? "名前の分からない書庫";
  const seen = jstMoment(latest.first_seen_at);
  switch (latest.outcome) {
    case "unreadable":
      rows.push({
        key: "latest",
        pinned: true,
        text: `${name}（${seen}）— 読めなかった書庫です（${unreadableKindLabel(latest.unreadable_kind)}）`,
      });
      break;
    case "store_failed":
      rows.push({
        key: "latest",
        pinned: true,
        text: `${name}（${seen}）— 格納に失敗しています（1 時間ごとに読み直します）`,
      });
      break;
    case "already_read":
      rows.push({
        key: "latest",
        text: `${name}（${seen}）— 既に読んだ書庫です（${latest.previously_read_at === null ? "前に読んだ時刻が分かりません" : `${jstMoment(latest.previously_read_at)} に読んだものと同じ中身`}）`,
      });
      break;
    case "pending_shape":
      rows.push({
        key: "latest",
        text: `${name}（${seen}）— 形の確認を待っています`,
      });
      break;
    default:
      rows.push({
        key: "latest",
        text: `${name}（${seen}）— 入った ${latest.inserted} · 既にあった ${latest.duplicate} · 読めなかった ${latest.unreadable}`,
      });
  }
  return rows;
}

/** 上限に収まるぶんだけ残し、省いた数を返す。**`pinned` は必ず残す**。 */
export function fitRows(rows: Row[]): { shown: Row[]; hidden: number } {
  if (rows.length <= BOX_MAX_ROWS) return { shown: rows, hidden: 0 };
  // 溢れるときは 1 行を「ほか N 件」に使う。
  const budget = BOX_MAX_ROWS - 1;
  const pinned = rows.filter((r) => r.pinned === true).slice(0, budget);
  const rest = rows.filter((r) => r.pinned !== true).slice(0, Math.max(0, budget - pinned.length));
  const keep = new Set([...pinned, ...rest]);
  const shown = rows.filter((r) => keep.has(r));
  return { shown, hidden: rows.length - shown.length };
}

/** Must の格子より前に置く、書庫の直近結果（design D12）。 */
export function LatestArchive({
  status,
  now = new Date(),
  failed = false,
}: {
  status: ArchivesStatus | null;
  now?: Date;
  failed?: boolean;
}): React.ReactElement {
  const { shown, hidden } = fitRows(archiveBoxRows(status, now, failed));
  return (
    <section
      data-testid="latest-archive"
      style={{
        font: FONT,
        maxHeight: ARCHIVE_BOX_MAX_PX,
        overflow: "hidden",
        borderStyle: "solid",
        borderColor: tone(TEXT.muted),
        borderTopWidth: BORDER_PX,
        borderBottomWidth: BORDER_PX,
        borderLeftWidth: BORDER_PX,
        borderRightWidth: BORDER_PX,
        paddingTop: PAD_PX,
        paddingBottom: PAD_PX,
        paddingLeft: PAD_PX,
        paddingRight: PAD_PX,
        marginBottom: MARGIN_BOTTOM_PX,
      }}
    >
      <strong style={{ font: FONT, display: "block" }}>直近に置いた書庫</strong>
      {shown.map((row) => (
        <p key={row.key} data-testid={`archive-row-${row.key}`} style={{ font: FONT, margin: 0 }}>
          {row.text}
        </p>
      ))}
      {hidden > 0 && (
        <p data-testid="archive-row-more" style={{ font: FONT, margin: 0 }}>
          ほか {hidden} 件
        </p>
      )}
    </section>
  );
}
