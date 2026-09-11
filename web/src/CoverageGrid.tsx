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
import { BAND, MIN_TARGET_PX, SURFACE, TEXT, tone } from "./tokens";

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
  const weeks = foldIntoWeeks(source.days);
  const shown = visibleWeeks(weeks, expanded);
  const selectedWeek = weeks.find((w) => w.start === selected) ?? null;

  return (
    <section
      aria-label={source.display_name}
      data-source={source.logical_source}
      style={{
        background: tone(SURFACE.surface2),
        borderRadius: 15,
        padding: 12,
        marginBottom: 24,
      }}
    >
      {/* **ソース名の文字**。色を意味の担い手にしない（ui-direction の宿題 1 / 第 4 回 Q15） */}
      <h2 style={{ color: tone(TEXT.normal), font: "600 15px/1.3 system-ui, sans-serif", margin: "0 0 8px" }}>
        {source.display_name}
      </h2>

      <div role="grid" data-testid={`grid-${source.logical_source}`} data-weeks={shown.length}>
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
        {expanded ? "直近だけにする" : "1 年ぶんを見る"}
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
    <button
      type="button"
      role="row"
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
        padding: 1,
        background: selected ? tone(SURFACE.surface1) : "transparent",
        border: "none",
        borderRadius: 4,
        cursor: "pointer",
      }}
    >
      {week.days.map((cell, i) => (
        <span
          // 週の中の位置は固定（日曜〜土曜）なので、位置そのものが鍵になる
          key={`${week.start}-${i}`}
          role="gridcell"
          data-band={cell === null ? "empty" : bandOf(cell.state)}
          data-day={cell?.day ?? ""}
          aria-hidden="true"
          style={{
            flex: 1,
            // **セルは表示専用**（Q16）。高さは帯に従い、幅は 7 等分
            minHeight: MIN_TARGET_PX - 2,
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
