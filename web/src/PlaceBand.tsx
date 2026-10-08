// SPDX-License-Identifier: AGPL-3.0-only
import { hourLevels } from "./places";
import { SCHEMES, tone, type Scheme } from "./tokens";

/** 24 区分の帯。濃さ = その時刻台の分 ÷ 最大。**0 の区分も枠は描く**（D12）。 */
export function Band({ hours, scheme }: { hours: number[]; scheme: Scheme }): React.ReactElement {
  const c = SCHEMES[scheme];
  const levels = hourLevels(hours);
  return (
    <div
      role="img"
      aria-label="24 区分の帯（0 時台から 23 時台）"
      data-testid="place-band"
      style={{ display: "flex", gap: 1, margin: "4px 0" }}
    >
      {levels.map((level, h) => (
        <span
          key={h}
          data-hour={h}
          data-level={level}
          style={{
            flex: 1,
            height: 14,
            boxSizing: "border-box",
            border: `1px solid ${tone(c.muted)}`,
            background: tone(c.surface2 + (c.muted - c.surface2) * level),
          }}
        />
      ))}
    </div>
  );
}
