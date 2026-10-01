// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所（`GET /places` / `GET /places/candidates`。ST21 / design D12 / D15）の型と形の検査、
 * 時間・日付・座標の書き方、記録の原文の組み立て（D13）。
 *
 * **原文は画面が組む**（design D2 / D3）—— サーバは受け取ったまま保存するので、サーバが乱数を足すと原文が変わる。
 * 場所の識別子は応答の型として持つだけで、画面には出さない。
 */
import { newNonce, type ValidFrom } from "./attributes";

/** 場所の記録の 2 つの時刻。RFC 3339（地域のずれつき / UTC）。 */
export interface RecordTimes {
  written_at: string;
  ingested_at: string;
}

export type CoordChange = "first" | "fix" | "move";
export type PreviousCoordState = "fixed" | "before_move" | "upcoming";

export interface PlaceCoord extends RecordTimes {
  record_id: string;
  lat: number;
  lon: number;
  change: CoordChange;
  valid_from: ValidFrom | null;
  supersedes: string | null;
}

export interface PreviousCoord extends PlaceCoord {
  state: PreviousCoordState;
  fixed_by: string | null;
}

export interface PreviousName extends RecordTimes {
  record_id: string;
  name: string;
}

/** 当たった滞在の勘定。`hours` は 0〜23 時台の分（24 個）。 */
export interface PlaceStays {
  count: number;
  minutes: number;
  /** 当たった滞在が無ければ `null` */
  last_day: string | null;
  hours: number[];
}

export interface Place {
  id: string;
  name: string;
  note: string | null;
  radius_m: number;
  name_record: RecordTimes & { record_id: string };
  coord: PlaceCoord;
  stays: PlaceStays;
  previous_names: PreviousName[];
  previous_coords: PreviousCoord[];
}

export interface PlacesData {
  today: string;
  places: Place[];
}

/** 名前の無い居た所（保存しない。D10）。 */
export interface Candidate {
  lat: number;
  lon: number;
  stays: { count: number; minutes: number; first_day: string; last_day: string; hours: number[] };
}

export interface CandidatesView {
  candidates: Candidate[];
}

const CHANGES: CoordChange[] = ["first", "fix", "move"];
const STATES: PreviousCoordState[] = ["fixed", "before_move", "upcoming"];
const PRECISIONS = ["year", "month", "day", "unknown"];

type Obj = Record<string, unknown>;
const obj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);
const str = (v: unknown): v is string => typeof v === "string";
const num = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const strOrNull = (v: unknown): boolean => v === null || str(v);
const list = (v: unknown, each: (x: unknown) => boolean): boolean => Array.isArray(v) && v.every(each);
const isHours = (v: unknown): boolean => Array.isArray(v) && v.length === 24 && v.every(num);

function isValidFrom(v: unknown): boolean {
  return obj(v) && PRECISIONS.includes(v.precision as string) && strOrNull(v.date);
}

function isTimes(v: Obj): boolean {
  return str(v.written_at) && str(v.ingested_at);
}

function isCoord(v: unknown): v is PlaceCoord {
  return (
    obj(v) &&
    str(v.record_id) &&
    num(v.lat) &&
    num(v.lon) &&
    CHANGES.includes(v.change as CoordChange) &&
    (v.valid_from === null || isValidFrom(v.valid_from)) &&
    strOrNull(v.supersedes) &&
    isTimes(v)
  );
}

function isPreviousCoord(v: unknown): v is PreviousCoord {
  return isCoord(v) && STATES.includes((v as unknown as Obj).state as PreviousCoordState) && strOrNull((v as unknown as Obj).fixed_by);
}

function isPlace(v: unknown): v is Place {
  if (!obj(v)) return false;
  const s = v.stays;
  return (
    str(v.id) &&
    str(v.name) &&
    strOrNull(v.note) &&
    num(v.radius_m) &&
    obj(v.name_record) &&
    str(v.name_record.record_id) &&
    isTimes(v.name_record) &&
    isCoord(v.coord) &&
    obj(s) &&
    num(s.count) &&
    num(s.minutes) &&
    strOrNull(s.last_day) &&
    isHours(s.hours) &&
    list(v.previous_names, (n) => obj(n) && str(n.record_id) && str(n.name) && isTimes(n)) &&
    list(v.previous_coords, isPreviousCoord)
  );
}

/** `GET /places` の応答が読める形か。**形が違えば失敗として出す** —— 型の宣言だけで通すと描画で落ちて画面が白くなる。 */
export function isPlacesData(v: unknown): v is PlacesData {
  return obj(v) && str(v.today) && list(v.places, isPlace);
}

/** `GET /places/candidates` の応答が読める形か。 */
export function isCandidatesView(v: unknown): v is CandidatesView {
  return (
    obj(v) &&
    list(
      v.candidates,
      (c) =>
        obj(c) &&
        num(c.lat) &&
        num(c.lon) &&
        obj(c.stays) &&
        num(c.stays.count) &&
        num(c.stays.minutes) &&
        str(c.stays.first_day) &&
        str(c.stays.last_day) &&
        isHours(c.stays.hours),
    )
  );
}

/** 合計の書き方。1 時間以上は「N 時間」（切り捨て）、未満は「N 分」。 */
export function durationLabel(minutes: number): string {
  return minutes >= 60 ? `${Math.floor(minutes / 60)} 時間` : `${minutes} 分`;
}

/** 座標の書き方。小数 4 桁（表示だけ。送る値は変えない）。 */
export function coordLabel(lat: number, lon: number): string {
  return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
}

/** 24 区分の帯の濃さ（0〜1）。その時刻台の分 ÷ 24 区分の最大。全部 0 なら全部 0。 */
export function hourLevels(hours: number[]): number[] {
  const max = Math.max(0, ...hours);
  return hours.map((m) => (max === 0 ? 0 : m / max));
}

/**
 * 前の座標に添える文字。**色だけで区別しない**（D12）。
 *
 * 移る前の「〜YYYY-MM」は、その座標の次に書かれた「移った」の座標の「いつから」
 * （応答は移る前の座標自身には移った日を持たない）。分からないときは日付を足さない。
 */
export function previousCoordLabel(prev: PreviousCoord, place: Place): string {
  if (prev.state === "fixed") return "直した";
  if (prev.state === "upcoming") {
    const date = prev.valid_from?.date;
    return date ? `予定（${date} から）` : "予定";
  }
  const at = Date.parse(prev.written_at);
  const next = [...place.previous_coords.filter((c) => c.state !== "fixed"), place.coord]
    .filter((c) => c.change === "move" && Date.parse(c.written_at) > at)
    .sort((a, b) => Date.parse(a.written_at) - Date.parse(b.written_at))[0];
  const date = next?.valid_from?.date;
  return date ? `移る前（〜${date}）` : "移る前";
}

/** 書く 1 項目（D2。1 記録 1 項目）。 */
export type PlaceField =
  | { field: "name"; name: string }
  | { field: "radius"; radius_m: number }
  | { field: "note"; note: string | null }
  | {
      field: "coord";
      lat: number;
      lon: number;
      change: CoordChange;
      /** `move` のときだけ */
      valid_from?: ValidFrom;
      /** `fix` のときだけ（直す座標の記録） */
      supersedes?: string;
    };

/** 送る 1 件（原文と、エンベロープの 2 つの時刻）。 */
export interface BuiltPlaceRecord {
  id: string;
  raw: string;
  /** 書いた日時（押した時刻） */
  writtenAt: string;
  tzOffsetMin: number;
  tzId: string;
}

/**
 * 原文を組む（design D2 の形）。**乱数は呼ぶたびに 128 bit を引き直し、識別子から導かない**（D3）。
 *
 * **押した時点で `id` / 乱数 / 書いた日時が決まる**。入力を変えずに押し直すときは、
 * これをもう一度呼ばずに同じ `BuiltPlaceRecord` を送る（組み直すと乱数が変わり、サーバは畳めずに 2 件になる。D13）。
 */
export function buildPlaceRecord(spec: PlaceField, now: Date, id: string, place: string): BuiltPlaceRecord {
  const head = { record: id, place, nonce: newNonce() };
  let body: Obj;
  switch (spec.field) {
    case "name":
      body = { field: "name", name: spec.name };
      break;
    case "radius":
      body = { field: "radius", radius_m: spec.radius_m };
      break;
    case "note":
      body = { field: "note", note: spec.note };
      break;
    case "coord":
      body = {
        field: "coord",
        lat: spec.lat,
        lon: spec.lon,
        change: spec.change,
        valid_from: spec.valid_from ?? null,
        supersedes: spec.supersedes ?? null,
      };
      break;
  }
  return {
    id,
    raw: JSON.stringify({ ...head, ...body }),
    writtenAt: now.toISOString(),
    // `getTimezoneOffset` は「UTC − 現地」なので符号を反転する
    tzOffsetMin: -now.getTimezoneOffset(),
    tzId: Intl.DateTimeFormat().resolvedOptions().timeZone,
  };
}

/** 取り込み口へ送る 1 件（`POST /api/ingest` の本文の要素）。 */
export function ingestPlaceItem(built: BuiltPlaceRecord, userId?: string): unknown {
  return {
    id: built.id,
    user_id: userId ?? "00000000-0000-0000-0000-000000000000",
    logical_source: "s01-place",
    external_id: null,
    device_id: null,
    origin: "authored",
    event_time: built.writtenAt,
    tz_offset_min: built.tzOffsetMin,
    tz_id: built.tzId,
    schema_version: 1,
    raw: built.raw,
    payload: {},
  };
}
