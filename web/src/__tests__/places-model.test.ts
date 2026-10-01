// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所の読み出しの型と形の検査、書き方、原文の組み立て（ST21 / tasks 8.1 / design D12 / D13）。
 * 応答を固定して見る。何を返すかは Rust の結合テストが見る。
 */
import { describe, expect, it } from "vitest";
import {
  buildPlaceRecord,
  coordLabel,
  durationLabel,
  hourLevels,
  ingestPlaceItem,
  isCandidatesView,
  isPlacesData,
  previousCoordLabel,
  type Place,
  type PlaceCoord,
  type PreviousCoord,
} from "../places";

const HOURS = Array.from({ length: 24 }, () => 0);

function place(over: Partial<Place> = {}): Place {
  return {
    id: "p1",
    name: "職場",
    note: null,
    radius_m: 100,
    name_record: { record_id: "r1", written_at: "2026-09-01T10:00:00+09:00", ingested_at: "2026-09-01T01:00:00Z" },
    coord: {
      record_id: "r2",
      lat: 35.68,
      lon: 139.75,
      change: "first",
      valid_from: null,
      supersedes: null,
      written_at: "2026-09-01T10:00:00+09:00",
      ingested_at: "2026-09-01T01:00:00Z",
    },
    stays: { count: 0, minutes: 0, last_day: null, hours: HOURS },
    previous_names: [],
    previous_coords: [],
    ...over,
  };
}

describe("places-model: 形の検査", () => {
  it("読み出しの形のとおりなら通り、欄が欠けるか型が違えば通さない", () => {
    expect(isPlacesData({ today: "2026-10-01", places: [place()] })).toBe(true);
    expect(isPlacesData({ today: "2026-10-01", places: [] })).toBe(true);
    expect(isPlacesData(null)).toBe(false);
    expect(isPlacesData({ today: "2026-10-01" })).toBe(false);
    expect(isPlacesData({ today: "2026-10-01", places: [{ ...place(), name: 3 }] })).toBe(false);
    expect(isPlacesData({ today: "2026-10-01", places: [{ ...place(), stays: { count: 1 } }] })).toBe(false);
    expect(isPlacesData({ today: "2026-10-01", places: [{ ...place(), stays: { ...place().stays, hours: [1, 2] } }] })).toBe(false);
    expect(isPlacesData({ today: "2026-10-01", places: [{ ...place(), previous_coords: [{ lat: 1 }] }] })).toBe(false);
  });

  it("居た所の形も検査する", () => {
    const c = { lat: 35.1, lon: 139.1, stays: { count: 3, minutes: 90, first_day: "2026-09-01", last_day: "2026-09-03", hours: HOURS } };
    expect(isCandidatesView({ candidates: [c] })).toBe(true);
    expect(isCandidatesView({ candidates: [{ ...c, stays: { ...c.stays, first_day: null } }] })).toBe(false);
    expect(isCandidatesView({ candidates: "x" })).toBe(false);
  });
});

describe("places-model: 書き方", () => {
  it("合計は 1 時間以上なら N 時間、未満なら N 分", () => {
    expect(durationLabel(59)).toBe("59 分");
    expect(durationLabel(60)).toBe("1 時間");
    expect(durationLabel(119)).toBe("1 時間");
    expect(durationLabel(61200)).toBe("1020 時間");
    expect(durationLabel(0)).toBe("0 分");
  });

  it("座標は小数 4 桁", () => {
    expect(coordLabel(35.681236, 139.767125)).toBe("35.6812, 139.7671");
  });

  it("帯の濃さは最大の区分を 1 にした割合で、全部 0 なら全部 0", () => {
    expect(hourLevels([...HOURS.slice(0, 22), 30, 60])).toEqual([...HOURS.slice(0, 22), 0.5, 1]);
    expect(hourLevels(HOURS)).toEqual(HOURS);
  });

  const prev = (over: Partial<PreviousCoord>): PreviousCoord => ({
    record_id: "x",
    lat: 1,
    lon: 2,
    change: "first",
    state: "fixed",
    valid_from: null,
    supersedes: null,
    written_at: "2026-09-01T10:00:00+09:00",
    ingested_at: "2026-09-01T01:00:00Z",
    fixed_by: null,
    ...over,
  });
  const move = (date: string, written: string): PlaceCoord => ({
    record_id: "m",
    lat: 3,
    lon: 4,
    change: "move",
    valid_from: { precision: "month", date },
    supersedes: null,
    written_at: written,
    ingested_at: written,
  });

  it("前の座標は直した・移る前（移った日）・予定（移る日）を文字で言う", () => {
    const p = place({ coord: move("2026-04", "2026-09-15T10:00:00+09:00") });
    expect(previousCoordLabel(prev({ state: "fixed", fixed_by: "f" }), p)).toBe("直した");
    expect(previousCoordLabel(prev({ state: "before_move" }), p)).toBe("移る前（〜2026-04）");
    expect(
      previousCoordLabel(prev({ state: "upcoming", change: "move", valid_from: { precision: "month", date: "2027-01" } }), p),
    ).toBe("予定（2027-01 から）");
  });

  it("移った日が分からないときは日付を足さない", () => {
    const p = place({ coord: { ...move("2026-04", "2026-09-15T10:00:00+09:00"), valid_from: { precision: "unknown", date: null } } });
    expect(previousCoordLabel(prev({ state: "before_move" }), p)).toBe("移る前");
  });
});

describe("places-model: 原文", () => {
  const now = new Date("2026-10-01T03:00:00Z");
  const build = () =>
    buildPlaceRecord({ field: "name", name: "自宅" }, now, "11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222");

  // Scenario: 同じ内容の 2 つの場所の記録は別々の乱数を持つ
  it("識別子・日時・場所・名前が同じでも乱数は記録ごとに違い、識別子と一致しない", () => {
    const a = build();
    const b = build();
    const na = (JSON.parse(a.raw) as { nonce: string }).nonce;
    const nb = (JSON.parse(b.raw) as { nonce: string }).nonce;
    expect(na).not.toBe(nb);
    expect(na).not.toBe(a.id);
    expect(nb).not.toBe(a.id);
    // 128 bit を base64url で 22 文字
    expect(na).toMatch(/^[A-Za-z0-9_-]{22}$/);
    expect(a.raw).not.toBe(b.raw);
  });

  it("4 つの項目の原文は D2 の形で、座標は変え方を持つ", () => {
    const rec = "11111111-1111-4111-8111-111111111111";
    const pl = "22222222-2222-4222-8222-222222222222";
    const raw = (spec: Parameters<typeof buildPlaceRecord>[0]) => JSON.parse(buildPlaceRecord(spec, now, rec, pl).raw) as Record<string, unknown>;
    expect(raw({ field: "name", name: "自宅" })).toMatchObject({ record: rec, place: pl, field: "name", name: "自宅" });
    expect(raw({ field: "radius", radius_m: 200 })).toMatchObject({ field: "radius", radius_m: 200 });
    expect(raw({ field: "note", note: null })).toMatchObject({ field: "note", note: null });
    expect(raw({ field: "coord", lat: 35.5, lon: 139.5, change: "first" })).toMatchObject({
      field: "coord",
      lat: 35.5,
      lon: 139.5,
      change: "first",
      valid_from: null,
      supersedes: null,
    });
    expect(
      raw({ field: "coord", lat: 1, lon: 2, change: "move", valid_from: { precision: "month", date: "2026-04" } }),
    ).toMatchObject({ change: "move", valid_from: { precision: "month", date: "2026-04" }, supersedes: null });
    expect(raw({ field: "coord", lat: 1, lon: 2, change: "fix", supersedes: "abc" })).toMatchObject({ change: "fix", supersedes: "abc", valid_from: null });
  });

  it("送る 1 件は s01-place の authored で、書いた日時と地域を持ち、原文を受け取ったまま渡す", () => {
    const built = build();
    const item = ingestPlaceItem(built, "u1") as Record<string, unknown>;
    expect(item).toMatchObject({
      id: built.id,
      user_id: "u1",
      logical_source: "s01-place",
      origin: "authored",
      event_time: "2026-10-01T03:00:00.000Z",
      schema_version: 1,
      raw: built.raw,
      payload: {},
      external_id: null,
      device_id: null,
    });
    expect(typeof item.tz_offset_min).toBe("number");
    expect(typeof item.tz_id).toBe("string");
  });
});
