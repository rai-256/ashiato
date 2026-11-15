import { describe, expect, it } from "vitest";
import { bannerLines, isUpdateStatus, lastUpdateLine, type UpdateStatus } from "../update";

const status = (overrides: Partial<UpdateStatus>): UpdateStatus => ({
  state: "available",
  installed: "0.4.0",
  latest: "0.5.0",
  found_at: "2026-11-10T09:12:00Z",
  checked_at: "2026-11-10T09:12:00Z",
  last_ok_at: "2026-11-10T09:12:00Z",
  reason: null,
  last_update: null,
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

  it("戻した更新と入らなかった更新を短い理由つきで帯の行にする", () => {
    expect(
      bannerLines(status({
        state: "latest",
        latest: "0.4.0",
        last_update: {
          at: "2026-11-11T09:12:00Z",
          result: "rolled_back",
          to_version: "0.5.0",
          from_version: "0.4.0",
          reason: { kind: "aborted", failed_version: "V1", sqlstate: "23505" },
        },
      })),
    ).toEqual(["更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました（V1・23505（制約に反した））"]);
    expect(
      bannerLines(status({
        state: "latest",
        latest: "0.4.0",
        last_update: {
          at: "2026-11-11T09:12:00Z",
          result: "not_applied",
          to_version: "0.5.0",
          from_version: "0.4.0",
          reason: { kind: "db_ahead", failed_version: null, sqlstate: null },
        },
      })),
    ).toEqual(["更新 0.5.0 は入りませんでした（DB に新しい版の知らない版がある）。0.4.0 のまま動いています"]);
  });

  it("理由の全種別とSQLSTATEの級を画面向けの短い文にする", () => {
    const kinds = [
      ["db_ahead", "DB に新しい版の知らない版がある"],
      ["ledger_unreadable", "版の記録を読めない"],
      ["channel_mismatch", "DB の系統が違う"],
      ["db_not_loopback", "DB の接続先が手元でない"],
      ["backup_failed", "移行の前の退避に失敗した"],
      ["connect_failed", "DB に繋げない"],
      ["server_not_listening", "新しい版のサーバが起きない"],
      ["aborted", "移行が途中で終わった"],
      ["pg_major_mismatch", "同梱の PostgreSQL の大きい版が違う"],
    ] as const;
    for (const [kind, text] of kinds) {
      expect(bannerLines(status({ state: "latest", latest: "0.4.0", last_update: {
        at: "2026-11-11T09:12:00Z", result: "not_applied", to_version: "0.5.0", from_version: "0.4.0",
        reason: { kind, failed_version: null, sqlstate: null },
      } }))[0]).toContain(text);
    }
    expect(bannerLines(status({ state: "latest", latest: "0.4.0", last_update: {
      at: "2026-11-11T09:12:00Z", result: "not_applied", to_version: "0.5.0", from_version: "0.4.0",
      reason: { kind: "connect_failed", failed_version: null, sqlstate: "08006" },
    } }))[0]).toContain("08006（繋がらない）");
    for (const [sqlstate, text] of [["23505", "制約に反した"], ["42P01", "定義が当たらない"], ["53100", "資源が足りない（空き・記憶）"], ["57014", "止められた"]] as const) {
      expect(bannerLines(status({ state: "latest", latest: "0.4.0", last_update: {
        at: "2026-11-11T09:12:00Z", result: "not_applied", to_version: "0.5.0", from_version: "0.4.0",
        reason: { kind: "aborted", failed_version: null, sqlstate },
      } }))[0]).toContain(`${sqlstate}（${text}）`);
    }
    expect(bannerLines(status({ state: "latest", latest: "0.4.0", last_update: {
      at: "2026-11-11T09:12:00Z", result: "not_applied", to_version: "0.5.0", from_version: "0.4.0",
      reason: { kind: "connect_failed", failed_version: null, sqlstate: "99999" },
    } }))[0]).toContain("99999（DB の誤り）");
  });

  it("理由が無い戻しは更新係が動いていないとは表示しない", () => {
    expect(lastUpdateLine({
      at: "2026-11-11T09:12:00Z",
      result: "rolled_back",
      to_version: "0.5.0",
      from_version: "0.4.0",
      reason: null,
    })).toBe("更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました");
    expect(lastUpdateLine({
      at: "2026-11-11T09:12:00Z",
      result: "not_applied",
      to_version: "0.5.0",
      from_version: "0.4.0",
      reason: null,
    })).toBe("更新 0.5.0 は入りませんでした。0.4.0 のまま動いています");
  });
});
