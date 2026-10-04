import { describe, expect, it } from "vitest";
import { isDayView } from "../stays";

describe("1 日の応答の形", () => {
  const view = {
    date: "2026-09-29",
    criteria: [],
    entries: [{ kind: "erased", start: "2026-09-29T00:00:00Z", end: "2026-09-29T01:00:00Z", stay_ids: ["stay-1"] }],
  };

  it("消した種類と滞在識別子を通す", () => {
    expect(isDayView(view)).toBe(true);
  });

  it("種類や識別子の形が違えば失敗として出す", () => {
    expect(isDayView({ ...view, entries: [{ ...view.entries[0], kind: "unknown" }] })).toBe(false);
    expect(isDayView({ ...view, entries: [{ ...view.entries[0], stay_ids: [1] }] })).toBe(false);
  });
});
