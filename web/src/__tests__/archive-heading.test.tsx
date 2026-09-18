// Scenario: 書庫のソースの見出しに最終日と何日前が出る
// Scenario: 何日前は日本時間の今日から数える
import { describe, expect, it } from "vitest";
import { archiveLastEventLabel } from "../archives";

describe("archiveLastEventLabel", () => {
  it("日本時間の今日から最終日までの日数を出す", () => {
    expect(archiveLastEventLabel("2026-09-12", new Date("2026-09-15T00:00:00Z"))).toBe("2026-09-12 まで（3 日前）");
  });

  it("UTCの日付ではなく日本時間の日付で数える", () => {
    expect(archiveLastEventLabel("2026-09-12", new Date("2026-09-14T16:00:00Z"))).toBe("2026-09-12 まで（3 日前）");
  });
});
