// SPDX-License-Identifier: AGPL-3.0-only
/** 書き出しの口の応答の読み手（ST33 / tasks 8.1）。形が違えば弾く。 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { isExportStatus, isSummary, passwordChars, startExport } from "../export";

const summary = {
  sources: [{ logical_source: "c01-gps", display_name: "GPS", count: 3, first: "2026-01-01T00:00:00Z", last: "2026-01-02T00:00:00Z" }],
  estimate: { rows: 3, bytes: 1000, seconds: 1 },
};

describe("export の読み手", () => {
  it("正しい summary を通し、壊れたものを弾く", () => {
    expect(isSummary(summary)).toBe(true);
    expect(isSummary({ ...summary, estimate: { rows: "3", bytes: 1, seconds: 1 } })).toBe(false);
    expect(isSummary({ ...summary, sources: [{ logical_source: "x" }] })).toBe(false);
    expect(isSummary(null)).toBe(false);
  });

  it("正しい状態を通し、壊れたものを弾く", () => {
    expect(isExportStatus({ leftover: [] })).toBe(true);
    expect(
      isExportStatus({
        leftover: ["a"],
        running: { export_id: "u", started_at: "t", written: 1, total: 2 },
        last: { finished_at: "t", rows: 1, bytes: 2, file: "f", erased_after: 0 },
        last_failure: null,
        interrupted: { started_at: "t", file: null },
      }),
    ).toBe(true);
    expect(isExportStatus({})).toBe(false);
    expect(isExportStatus({ leftover: [], running: { export_id: "u" } })).toBe(false);
    expect(isExportStatus({ leftover: [], last: { finished_at: "t", rows: "1", bytes: 2, file: "f", erased_after: 0 } })).toBe(false);
    expect(isExportStatus({ leftover: [1] })).toBe(false);
  });

  it("文字数は符号位置で数える", () => {
    expect(passwordChars("😀".repeat(11))).toBe(11);
    expect(passwordChars("あいう")).toBe(3);
  });
});

describe("startExport", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("合言葉は本体にだけ載り、URL に載らない", async () => {
    const f = vi.fn().mockResolvedValue({ ok: true, status: 202, json: () => Promise.resolve({}) });
    vi.stubGlobal("fetch", f);
    expect(await startExport("pw-pw-pw-pw-1")).toEqual({ at: "started" });
    const [url, init] = f.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/api/export");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ password: "pw-pw-pw-pw-1" });
  });

  it("断られた種別と、通信の失敗を分ける", async () => {
    vi.stubGlobal("fetch", () => Promise.resolve({ ok: false, status: 409, json: () => Promise.resolve({ error: "export_running" }) }));
    expect(await startExport("x")).toEqual({ at: "refused", status: 409, error: "export_running" });
    vi.stubGlobal("fetch", () => Promise.reject(new Error("down")));
    expect(await startExport("x")).toEqual({ at: "unreachable" });
  });
});
