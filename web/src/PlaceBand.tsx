// SPDX-License-Identifier: AGPL-3.0-only
import { msg } from "./messages";
import { hourLevels } from "./places";
import { SCHEMES, tone, type Hsl, type Scheme } from "./tokens";

const t = msg("master").places;

/** 2 つの色の間（`level` = 0 で `a`、1 で `b`）。面と控えめの文字は同じ色相・彩度なので、明るさだけが動く。 */
function mix(a: Hsl, b: Hsl, level: number): Hsl {
  return [a[0] + (b[0] - a[0]) * level, a[1] + (b[1] - a[1]) * level, a[2] + (b[2] - a[2]) * level];
}

/** 24 区分の帯。濃さ = その時刻台の分 ÷ 最大。**0 の区分も枠は描く**（D12）。 */
export function Band({ hours, scheme }: { hours: number[]; scheme: Scheme }): React.ReactElement {
  const c = SCHEMES[scheme];
  const levels = hourLevels(hours);
  return (
    <div
      role="img"
      aria-label={t.band}
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
            background: tone(mix(c.surface2, c.muted, level)),
          }}
        />
      ))}
    </div>
  );
}
