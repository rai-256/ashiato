// SPDX-License-Identifier: AGPL-3.0-only
import { useState } from "react";
import {
  bandOf,
  dropMarkTone,
  dropNotes,
  foldIntoWeeks,
  hasDropMark,
  isRetired,
  STATE_NAME,
  visibleWeeks,
  type SourceCoverage,
  type Week,
} from "./coverage";
import { msg } from "./messages";
import { FOCUS_ATTR, useScheme } from "./scheme";
import { controlStyle, sectionStyle } from "./surface";
import { font, MIN_TARGET_PX, RADIUS, SCHEMES, SPACE, TYPE, tone, type Scheme, type SchemeColors } from "./tokens";

const m = msg("grid");

/**
 * ソース 1 本ぶんの格子（FR-54 / 深掘り Q6, 第 4 回 Q15, 第 5 回 Q20/Q21, 第 6 回 Q25, 第 7 回 Q28）。
 *
 * - **縦長**（1 行 = 1 週、上から下へ、新しい週が上）
 * - **ソースごとに格子を分け、ソース名の文字を添える**（色は意味の担い手にしない）
 * - **セルは表示専用。選ぶ単位は週**（NFR-19 の 24 × 24 px を割らないため）
 * - 格子のセルが担うのは **3 段**。8 状態の区別は**週を選んだときの文字**
 */
export function CoverageGrid({ source }: { source: SourceCoverage }): React.ReactElement {
  // **OS の明暗に追従する**（NFR-17。2026-10-12 まで S-1 はダークしか無かった）
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  const [expanded, setExpanded] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  // **退役したソースは既定で畳む**（ST03 の R63 / 第 8 回 Q30）—— 退役は 1 本きりではなく
  // 増えるので、行を出したままだと **Must の 5 本が 1 画面から押し出される**。
  // 消しはしない（退役したことも稼働状況の一部）。開けば同じ格子が出る。
  const retired = isRetired(source);
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
      // **ひとスクロールの勘定に効く**（第 8 回 Q30）。`one-scroll.test.tsx` が
      // ここに宣言されている値（余白・枠線）を DOM から積んで、固定の予算と突き合わせる
      style={sectionStyle(c, c.surface2)}
    >
      {/* **ソース名の文字**。色を意味の担い手にしない（ui-direction の宿題 1 / 第 4 回 Q15） */}
      <h2 style={{ color: tone(c.text), font: font(TYPE.body, 600, 1.3), margin: "0 0 8px" }}>
        {source.display_name}
        {/* **乗り換えが起きたことを文字で出す**（review/code-r2.md の I3）。
            名指しした名前と先端が違うのに黙っていると、本人には
            「c02-window が消えて知らない名前が増えた」ようにしか見えない */}
        {source.named_source !== source.logical_source && (
          <span data-testid={`succeeded-${source.named_source}`} style={{ color: tone(c.muted), font: font(TYPE.caption, 400, 1.3) }}>
            {m.succeeded(source.named_source)}
          </span>
        )}
        {retired && (
          <span data-testid={`retired-${source.logical_source}`} style={{ color: tone(c.muted), font: font(TYPE.caption, 400, 1.3) }}>
            {m.retired(source.retired_on)}
          </span>
        )}
      </h2>

      <div data-testid={`grid-${source.logical_source}`} data-weeks={shown.length} data-role="grid">
        {shown.map((week) => (
          <WeekRow
            key={week.start}
            week={week}
            selected={week.start === selected}
            scheme={scheme}
            onSelect={() => setSelected(week.start === selected ? null : week.start)}
          />
        ))}
      </div>

      <button
        type="button"
        {...{ [FOCUS_ATTR]: "" }}
        onClick={() => setExpanded(!expanded)}
        style={{ ...controlStyle(c), font: font(TYPE.caption, 400, 1.4), marginTop: 8 }}
      >
        {expanded ? m.collapse : retired ? m.showRetired : m.expand}
      </button>

      {selectedWeek !== null && <WeekDetail week={selectedWeek} c={c} />}
    </section>
  );
}

/** 破棄の印（右下の三角）の辺の長さ（design D10）。セルは最小 24 px なので半分に収まる。 */
const DROP_MARK_PX = 12;

/**
 * 1 週ぶんの帯。**これが操作対象**（深掘り 第 4 回 Q16 / 第 5 回 Q20）。
 *
 * 幅いっぱい × 高さ `MIN_TARGET_PX` 以上を取れるので、360 px 幅でも NFR-19 を満たす。
 * **セルにはイベントハンドラを付けない。**
 */
function WeekRow({
  week,
  selected,
  scheme,
  onSelect,
}: {
  week: Week;
  selected: boolean;
  scheme: Scheme;
  onSelect: () => void;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  return (
    // **`role="row"` を付けない**（review/code.md の R35 / I6）。`<button>` に付けると
    // 暗黙の button ロールを上書きし、**この画面で唯一の操作対象が支援技術から消える**。
    // `aria-pressed` も ARIA 1.2 では button ロールにしかサポートされる状態が無いので、
    // 選択中であることが伝わらなくなっていた。行であることは `data-week` で足りる。
    <button
      type="button"
      aria-label={m.week(week.start)}
      aria-pressed={selected}
      data-week={week.start}
      {...{ [FOCUS_ATTR]: "" }}
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
        // 帯は文字を持たない。地の本文の行の高さ（15.5 px × 1.7 = 26.35 px）を継がせると、
        // 空のセルの箱が 24 px より高く積まれる（ひとスクロールの勘定。第 8 回 Q30）
        font: font(TYPE.caption, 400, 1),
        // **選択を面の明るさで表さない**（review/code.md の R34 / I5 / F12）。
        // 面を変えると、いちばん見たい週で格子がいちばん読めなくなる。輪郭なら段の色に触らない
        // （`muted` は面の 3 段すべてに 4.5:1 以上）。
        // **輪郭は内側の影で描く**（2026-10-12）。`outline` に書くと、フォーカスの輪郭
        // （`[data-focus-ring]:focus-visible` の outline）を style 属性が上書きして消す（NFR-22）
        background: "transparent",
        border: "none",
        boxShadow: selected ? `inset 0 0 0 2px ${tone(c.muted)}` : "none",
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
            borderRadius: RADIUS.cell,
            // 印を右下に置くための基準（セルの大きさは変えない）
            position: "relative",
            overflow: "hidden",
            background: cell === null ? "transparent" : tone(c.cell[bandOf(cell.state)]),
            // 「それ以外」の段は節の面に近いので、控えめな文字の色の縁で広がりを読ませる（NFR-23）
            boxShadow: cell !== null && bandOf(cell.state) === "other" ? `inset 0 0 0 1px ${tone(c.muted)}` : "none",
          }}
        >
          {cell !== null && hasDropMark(cell) && (
            // **一部を破棄した日の印**（ST04 / 深掘り Q3）。色ではなく**形**で持たせる ——
            // 右下を三角に欠く。意味は週の詳細の文字が担うので、支援技術には出さない
            <span
              data-drop-mark="true"
              aria-hidden="true"
              style={{
                position: "absolute",
                right: 0,
                bottom: 0,
                width: 0,
                height: 0,
                borderStyle: "solid",
                borderWidth: `0 0 ${DROP_MARK_PX}px ${DROP_MARK_PX}px`,
                borderColor: `transparent transparent ${tone(dropMarkTone(bandOf(cell.state), scheme))} transparent`,
              }}
            />
          )}
        </span>
      ))}
    </button>
  );
}

/**
 * 選んだ週の 7 日ぶんを、**8 状態それぞれの名前で文字で**出す（第 5 回 Q20 / Q21）。
 *
 * **格子で「それ以外」に畳まれた日も、ここでどの状態だったかが分かる** ——
 * 明度だけでは 7 段を分けられない（3^6 = 729:1 > sRGB の 21:1）ので、
 * 区別の担い手はこの文字。
 */
function WeekDetail({ week, c }: { week: Week; c: SchemeColors }): React.ReactElement {
  return (
    <dl
      data-testid="week-detail"
      style={{
        color: tone(c.text),
        font: font(TYPE.caption, 400, 1.6),
        margin: "12px 0 0",
      }}
    >
      {/* 端の週は 7 日ぶん揃わない。**無い日は出さない**（無い日に状態は無い） */}
      {week.days
        .filter((cell): cell is NonNullable<typeof cell> => cell !== null)
        .map((cell) => (
          <div key={cell.day} style={{ display: "flex", gap: SPACE.targetGap }}>
            <dt style={{ color: tone(c.muted), minWidth: "6.5em" }}>{cell.day}</dt>
            <dd style={{ margin: 0 }} data-state={cell.state}>
              {STATE_NAME[cell.state]}
              {/* **期間と件数はここだけに書く**（ST04 / 深掘り Q3）。一覧は作らない */}
              {dropNotes(cell).map((note, i) => (
                <span key={`${i}-${note}`} data-drop-note="true">{` ${note}`}</span>
              ))}
            </dd>
          </div>
        ))}
    </dl>
  );
}
