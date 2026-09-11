// SPDX-License-Identifier: AGPL-3.0-only
import { useState } from "react";
import {
  bandOf,
  foldIntoWeeks,
  STATE_NAME,
  visibleWeeks,
  type SourceCoverage,
  type Week,
} from "./coverage";
import { BAND, MIN_TARGET_PX, SECTION_GAP_PX, SECTION_PAD_PX, SURFACE, TEXT, tone } from "./tokens";

/**
 * ソース 1 本ぶんの格子（FR-54 / 深掘り Q6, 第 4 回 Q15, 第 5 回 Q20/Q21, 第 6 回 Q25, 第 7 回 Q28）。
 *
 * - **縦長**（1 行 = 1 週、上から下へ、新しい週が上）
 * - **ソースごとに格子を分け、ソース名の文字を添える**（色は意味の担い手にしない）
 * - **セルは表示専用。選ぶ単位は週**（NFR-19 の 24 × 24 px を割らないため）
 * - 格子のセルが担うのは **3 段**。7 状態の区別は**週を選んだときの文字**
 */
export function CoverageGrid({ source }: { source: SourceCoverage }): React.ReactElement {
  const [expanded, setExpanded] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  // **退役したソースは既定で畳む**（ST03 の R63 / 第 8 回 Q30）—— 退役は 1 本きりではなく
  // 増えるので、行を出したままだと **Must の 5 本が 1 画面から押し出される**。
  // 消しはしない（退役したことも稼働状況の一部）。開けば同じ格子が出る。
  const retired = source.retired_on !== null;
  const weeks = foldIntoWeeks(source.days);
  const shown = retired && !expanded ? [] : visibleWeeks(weeks, expanded);
  // **見えている週から選ぶ**（review/code.md の R37 / I11）。`weeks` から探していたときは、
  // 1 年ぶんに伸ばして 30 週目を選んでから畳み戻すと、**行は消えるのに日付リストだけ残り**、
  // どの行にも選択の印が立っていない状態で閉じる手段が無くなった。
  const selectedWeek = shown.find((w) => w.start === selected) ?? null;

  return (
    <section
      aria-label={source.display_name}
      data-source={source.logical_source}
      data-retired={source.retired_on ?? ""}
      style={{
        background: tone(SURFACE.surface2),
        borderRadius: 15,
        // **ひとスクロールの勘定に効く**（第 8 回 Q30）。`one-scroll.test.tsx` が
        // ここに宣言されている値を DOM から積んで、固定の予算と突き合わせる
        padding: SECTION_PAD_PX,
        marginBottom: SECTION_GAP_PX,
      }}
    >
      {/* **ソース名の文字**。色を意味の担い手にしない（ui-direction の宿題 1 / 第 4 回 Q15） */}
      <h2 style={{ color: tone(TEXT.normal), font: "600 15px/1.3 system-ui, sans-serif", margin: "0 0 8px" }}>
        {source.display_name}
        {retired && (
          <span data-testid={`retired-${source.logical_source}`} style={{ color: tone(TEXT.muted), font: "400 13px/1.3 system-ui, sans-serif" }}>
            {` — ${source.retired_on} に退役`}
          </span>
        )}
      </h2>

      <div data-testid={`grid-${source.logical_source}`} data-weeks={shown.length} data-role="grid">
        {shown.map((week) => (
          <WeekRow
            key={week.start}
            week={week}
            selected={week.start === selected}
            onSelect={() => setSelected(week.start === selected ? null : week.start)}
          />
        ))}
      </div>

      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        style={{
          minHeight: MIN_TARGET_PX,
          minWidth: MIN_TARGET_PX,
          marginTop: 8,
          background: "transparent",
          border: `1px solid ${tone(TEXT.muted)}`,
          borderRadius: 8,
          color: tone(TEXT.normal),
          font: "400 13px/1.4 system-ui, sans-serif",
          padding: "4px 12px",
        }}
      >
        {expanded ? "直近だけにする" : retired ? "退役したソースを見る" : "1 年ぶんを見る"}
      </button>

      {selectedWeek !== null && <WeekDetail week={selectedWeek} />}
    </section>
  );
}

/**
 * 1 週ぶんの帯。**これが操作対象**（深掘り 第 4 回 Q16 / 第 5 回 Q20）。
 *
 * 幅いっぱい × 高さ `MIN_TARGET_PX` 以上を取れるので、360 px 幅でも NFR-19 を満たす。
 * **セルにはイベントハンドラを付けない。**
 */
function WeekRow({
  week,
  selected,
  onSelect,
}: {
  week: Week;
  selected: boolean;
  onSelect: () => void;
}): React.ReactElement {
  return (
    // **`role="row"` を付けない**（review/code.md の R35 / I6）。`<button>` に付けると
    // 暗黙の button ロールを上書きし、**この画面で唯一の操作対象が支援技術から消える**。
    // `aria-pressed` も ARIA 1.2 では button ロールにしかサポートされる状態が無いので、
    // 選択中であることが伝わらなくなっていた。行であることは `data-week` で足りる。
    <button
      type="button"
      aria-label={`${week.start} の週`}
      aria-pressed={selected}
      data-week={week.start}
      onClick={onSelect}
      style={{
        display: "flex",
        gap: 2,
        width: "100%",
        // **NFR-19**: 週の帯が 24 × 24 CSS px 以上
        minHeight: MIN_TARGET_PX,
        minWidth: MIN_TARGET_PX,
        // **余白はここに置かない**（第 8 回 Q30 の勘定）。セルどうしの間は `gap` が担い、
        // 行どうしの間は取らない —— 行 1 本あたり 2 px でも 5 ソース × 4 行で 40 px になる
        padding: 0,
        // **選択を面の明るさで表さない**（review/code.md の R34 / I5 / F12）。
        // `surface1`(18%) の上だと、いちばん暗い段（9%）との比が **1.422:1** に落ちて
        // **いちばん見たい週で格子がいちばん読めなくなる**。輪郭なら段の色に触らない
        // （`muted` は surface2 の上で 5.195:1）。
        background: "transparent",
        border: "none",
        outline: selected ? `2px solid ${tone(TEXT.muted)}` : "none",
        outlineOffset: -2,
        borderRadius: 4,
        cursor: "pointer",
      }}
    >
      {week.days.map((cell, i) => (
        <span
          // 週の中の位置は固定（日曜〜土曜）なので、位置そのものが鍵になる
          key={`${week.start}-${i}`}
          data-cell="day"
          data-band={cell === null ? "empty" : bandOf(cell.state)}
          data-day={cell?.day ?? ""}
          aria-hidden="true"
          style={{
            flex: 1,
            // **セルは表示専用**（Q16）。高さは帯に従い、幅は 7 等分
            minHeight: MIN_TARGET_PX,
            borderRadius: 2,
            background: cell === null ? "transparent" : tone(BAND[bandOf(cell.state)]),
          }}
        />
      ))}
    </button>
  );
}

/**
 * 選んだ週の 7 日ぶんを、**7 状態それぞれの名前で文字で**出す（第 5 回 Q20 / Q21）。
 *
 * **格子で「それ以外」に畳まれた日も、ここでどの状態だったかが分かる** ——
 * 明度だけでは 7 段を分けられない（3^6 = 729:1 > sRGB の 21:1）ので、
 * 区別の担い手はこの文字。
 */
function WeekDetail({ week }: { week: Week }): React.ReactElement {
  return (
    <dl
      data-testid="week-detail"
      style={{
        color: tone(TEXT.normal),
        font: "400 13px/1.6 system-ui, sans-serif",
        margin: "12px 0 0",
      }}
    >
      {/* 端の週は 7 日ぶん揃わない。**無い日は出さない**（無い日に状態は無い） */}
      {week.days
        .filter((cell): cell is NonNullable<typeof cell> => cell !== null)
        .map((cell) => (
          <div key={cell.day} style={{ display: "flex", gap: 8 }}>
            <dt style={{ color: tone(TEXT.muted), minWidth: "6.5em" }}>{cell.day}</dt>
            <dd style={{ margin: 0 }} data-state={cell.state}>
              {STATE_NAME[cell.state]}
            </dd>
          </div>
        ))}
    </dl>
  );
}
