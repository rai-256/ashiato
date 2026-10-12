// SPDX-License-Identifier: AGPL-3.0-only
import {
  daysAgo,
  INBOX_STALE_DAYS,
  jstMoment,
  unreadableKindLabel,
  type ArchivesStatus,
} from "./archives";
import { msg } from "./messages";
import { useScheme } from "./scheme";
import { font, SCHEMES, tone, TYPE } from "./tokens";

const m = msg("archive");

/**
 * **箱の高さの上限**（design D12。**仮**）。
 *
 * 反転条件: 優先の低い行が常に省かれ、本人が見落としたと分かったとき（上げるときは
 * `collection-coverage` の予算の文も合わせて直す）。
 */
export const ARCHIVE_BOX_MAX_PX = 160;

const LINE_HEIGHT = 1.6;
const FONT = font(TYPE.body, 400, LINE_HEIGHT);
const LINE_PX = TYPE.body * LINE_HEIGHT;
const PAD_PX = 8;
const BORDER_PX = 1;
const MARGIN_BOTTOM_PX = 12;

/**
 * 1 行に収める（code-verify R64）。本物のブラウザは長いファイル名を折り返し、
 * 行数で数えた上限を中身が越えて `overflow: hidden` に切られていた —— 「ほか N 件」と
 * 省かない行（`pinned`）が箱の外に出て見えなかった。収まらない文字は「…」で省いたと示す。
 */
const ONE_LINE = {
  font: FONT,
  margin: 0,
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
} as const;

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
    return [{ key: "failed", text: m.failed }];
  }
  if (status === null) return [{ key: "loading", text: m.loading }];

  const reading = status.reading;
  if (reading !== undefined && reading !== null) {
    rows.push({
      key: "reading",
      text: m.reading(reading.file_name, reading.inner_path, reading.items_read.toLocaleString("en-US"), jstMoment(reading.started_at)),
    });
  }

  const inbox = status.inbox;
  if (inbox !== undefined && inbox !== null && !inbox.capturable) {
    const named = inbox.blockers.map((b) => m.place[b] ?? b);
    rows.push({
      key: "inbox-unreadable",
      text: m.unreadablePlaces(named.length === 0 ? m.anyPlace : named.join(m.placeSeparator)),
    });
  }
  if (inbox === undefined || inbox === null) {
    rows.push({ key: "inbox-never", text: m.inboxNever });
  } else {
    const ago = daysAgo(inbox.emitted_at, now);
    if (ago > INBOX_STALE_DAYS) {
      rows.push({ key: "inbox-stale", text: m.inboxStale(ago) });
    }
  }

  const pending = status.pending_shape;
  if (pending !== undefined && pending !== null) {
    rows.push({
      key: "pending-shape",
      text: m.pendingShape(pending.archives),
    });
  }

  const latest = status.latest_archive;
  if (latest === undefined || latest === null) {
    rows.push({ key: "empty", text: m.empty });
    return rows;
  }
  const name = latest.file_name ?? m.unnamed;
  const seen = jstMoment(latest.first_seen_at);
  switch (latest.outcome) {
    case "unreadable":
      rows.push({
        key: "latest",
        pinned: true,
        text: m.unreadable(name, seen, unreadableKindLabel(latest.unreadable_kind)),
      });
      break;
    case "store_failed":
      rows.push({
        key: "latest",
        pinned: true,
        text: m.storeFailed(name, seen),
      });
      break;
    case "already_read":
      rows.push({
        key: "latest",
        text: m.alreadyRead(name, seen, latest.previously_read_at === null ? null : jstMoment(latest.previously_read_at)),
      });
      break;
    case "pending_shape":
      rows.push({
        key: "latest",
        text: m.pendingLatest(name, seen),
      });
      break;
    default:
      rows.push({
        key: "latest",
        text: m.read(name, seen, latest.inserted, latest.duplicate, latest.unreadable),
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
  // **OS の明暗に追従する**（NFR-17）
  const c = SCHEMES[useScheme()];
  const { shown, hidden } = fitRows(archiveBoxRows(status, now, failed));
  return (
    <section
      data-testid="latest-archive"
      style={{
        font: FONT,
        // 上限は枠と内側の余白を含めた外寸（R64。既定の content-box だと 178 px になっていた）
        boxSizing: "border-box",
        maxHeight: ARCHIVE_BOX_MAX_PX,
        overflow: "hidden",
        borderStyle: "solid",
        color: tone(c.text),
        borderColor: tone(c.muted),
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
      <strong style={{ ...ONE_LINE, display: "block" }}>{m.title}</strong>
      {shown.map((row) => (
        <p
          key={row.key}
          data-testid={`archive-row-${row.key}`}
          title={row.text}
          style={ONE_LINE}
        >
          {row.text}
        </p>
      ))}
      {hidden > 0 && (
        <p data-testid="archive-row-more" style={ONE_LINE}>
          {m.more(hidden)}
        </p>
      )}
    </section>
  );
}
