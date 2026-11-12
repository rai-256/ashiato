import { describe, expect, it } from "vitest";
import { bannerLines, isUpdateStatus, type UpdateStatus } from "../update";

const status = (overrides: Partial<UpdateStatus>): UpdateStatus => ({
  state: "available",
  installed: "0.4.0",
  latest: "0.5.0",
  found_at: "2026-11-10T09:12:00Z",
  checked_at: "2026-11-10T09:12:00Z",
  last_ok_at: "2026-11-10T09:12:00Z",
  reason: null,
  ...overrides,
});

describe("更新状態", () => {
  it("状態の形を検証し、更新ありだけを帯の行にする", () => {
    expect(isUpdateStatus(status({}))).toBe(true);
    expect(isUpdateStatus({ ...status({}), state: "unknown" })).toBe(false);
    expect(bannerLines(status({}))).toEqual(["更新あり 0.5.0（11/10 に見つけた）"]);
    expect(bannerLines(status({ state: "latest", latest: "0.4.0" }))).toEqual([]);
  });

  it("7日以上確かめられていない行を、最後に成功した日と理由で作る", () => {
    expect(
      bannerLines(
        status({
          state: "available_stale",
          checked_at: "2026-11-11T23:00:00Z",
          last_ok_at: "2026-11-03T09:12:00Z",
          reason: "unreachable",
        }),
      ),
    ).toEqual([
      "更新あり 0.5.0（11/10 に見つけた）",
      "更新を 7 日確かめられていない（最後に確かめた 11/03・配布元に届かない）",
    ]);
  });
});
