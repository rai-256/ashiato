// SPDX-License-Identifier: AGPL-3.0-only
import type { Achievement } from "./coverage";
import { msg } from "./messages";
import { SECTION_GAP_PX, SECTION_PAD_PX, SURFACE, TEXT, tone } from "./tokens";

/**
 * 成功条件 1 の達成（NFR-13 / 深掘り 第 4 回 Q9, 第 5 回 Q18, 第 7 回 Q27）。
 *
 * - 5 ソースそれぞれの**達成日数と分母**を数値で出す
 * - 合否は **5 本すべてが分母の 95 % 以上**か（絶対値の 350 日ではない）
 * - それが**確定か暫定か**を出し、暫定なら**確定までの残り日数**
 * - 確定する日が定まらないとき（まだ収集を開始していないソースがある）は、その理由
 */
export function AchievementPanel({ data }: { data: Achievement }): React.ReactElement {
  const m = msg("achievement");
  return (
    <section
      aria-label={m.label}
      data-testid="achievement"
      style={{
        background: tone(SURFACE.surface2),
        borderRadius: 15,
        // **ひとスクロールの勘定に効く**（第 8 回 Q30）
        padding: SECTION_PAD_PX,
        marginBottom: SECTION_GAP_PX,
        color: tone(TEXT.normal),
        font: "400 13px/1.4 system-ui, sans-serif",
      }}
    >
      <h2 style={{ font: "600 15px/1.3 system-ui, sans-serif", margin: "0 0 8px" }}>
        {m.title}
      </h2>

      <p data-testid="verdict" data-verdict={data.verdict} data-confirmed={data.confirmed} style={{ margin: "0 0 8px" }}>
        {data.verdict ? m.met : m.notMet}
        {m.separator}
        {/* **確定か暫定か**（第 7 回 Q27）。これが無いと、収集開始 10 日目のソースが
            10 日とも達成していれば 100 % で「達成」に見える */}
        <strong data-testid="confirmation">{data.confirmed ? m.confirmed : m.provisional}</strong>
      </p>

      {!data.confirmed && (
        <p data-testid="until-confirmed" style={{ color: tone(TEXT.muted), margin: "0 0 8px" }}>
          {data.days_until_confirmed === null
            ? // 確定する日が定まらない理由を出す（残り日数の代わり）
              m.undecided(data.not_started)
            : m.daysLeft(data.days_until_confirmed)}
        </p>
      )}

      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <thead>
          <tr style={{ color: tone(TEXT.muted), textAlign: "left" }}>
            <th scope="col">{m.colSource}</th>
            <th scope="col">{m.colAchieved}</th>
            <th scope="col">{m.colDenominator}</th>
            <th scope="col">{m.colThreshold}</th>
          </tr>
        </thead>
        <tbody>
          {data.sources.map((s) => (
            <tr key={s.logical_source} data-source={s.logical_source} data-met={s.met}>
              <th scope="row" style={{ fontWeight: 400, textAlign: "left" }}>
                {s.display_name}
              </th>
              <td data-testid={`achieved-${s.logical_source}`}>{s.achieved_days}</td>
              <td data-testid={`denominator-${s.logical_source}`}>{s.denominator}</td>
              <td style={{ color: tone(TEXT.muted) }}>
                {s.collection_started_on === null ? m.notStarted : s.threshold.toFixed(2)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
