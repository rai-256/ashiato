// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所（`GET /places` / `GET /places/candidates`。ST21 / design D12 / D15）の型と形の検査、
 * 時間・日付・座標の書き方、記録の原文の組み立て（D13）。
 *
 * **原文は画面が組む**（design D2 / D3）—— サーバは受け取ったまま保存するので、サーバが乱数を足すと原文が変わる。
 * 場所の識別子は応答の型として持つだけで、画面には出さない。
 */
import {
  newNonce,
  readKindResponse,
  UNREACHABLE_MESSAGE,
  type SendOutcome,
  type ValidFrom,
} from "./attributes";

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
 * 移る前の「〜YYYY-MM」は、その座標の次の版（「いつから」の順で直後の「移った」の座標）の「いつから」
 * （応答は移る前の座標自身には移った日を持たない）。分からないときは日付を足さない。
 */
export function previousCoordLabel(prev: PreviousCoord, place: Place): string {
  if (prev.state === "fixed") return "直した";
  if (prev.state === "upcoming") {
    const date = prev.valid_from?.date;
    return date ? `予定（${date} から）` : "予定";
  }
  // サーバは前の座標を書いた順に返す。ここで「いつから」（valid_from）の順に並べ直して、直後の移転を引く
  const from = prev.valid_from?.date ?? "";
  const next = [...place.previous_coords.filter((c) => c.state !== "fixed" && c !== prev), place.coord]
    .filter((c) => c.change === "move" && c.valid_from?.date && c.valid_from.date > from)
    .sort((a, b) => (a.valid_from?.date ?? "").localeCompare(b.valid_from?.date ?? ""))[0];
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

/** 広さの選択肢（m。D14。API は 5,000 m まで受ける）。 */
export const RADIUS_CHOICES = [50, 100, 200, 300];
/** 登録の広さの最初（`places::PLACE_DEFAULT_RADIUS_M` と同じ） */
export const DEFAULT_RADIUS_M = 100;
/** 名前の無い居た所を最初に出す件数（D10 / D14）。残りは「残り N か所」 */
export const CANDIDATE_TOP = 10;

/** 器を作る求めの断り。押し直しで識別子を作り直す印（D13） */
export const PLACE_ID_TAKEN = "place_id_taken";

/** 断られた理由を、本人に読める文にする（design D13 の表。種別ごとに文を分ける）。 */
export function placeRejectionMessage(kind: string): string {
  switch (kind) {
    case "invalid_place_name":
      return "名前が空です。名前を入れてください";
    case "invalid_radius":
      return "広さが範囲の外です";
    case "invalid_valid_from":
      return "「いつから」の日付が読めません";
    case "unknown_place":
      return "この場所が見つかりません。画面を読み直してください";
    case "invalid_coord_supersedes":
      return "直す座標が見つかりません。画面を読み直してください";
    case "invalid_coord_change":
      return "この場所の座標の変え方が合いません。画面を読み直してください";
    case "invalid_coordinate":
      return "座標が読めません";
    case PLACE_ID_TAKEN:
      return "場所を作れませんでした。もう一度「登録する」を押してください";
    default:
      return `受け付けられませんでした（${kind}）`;
  }
}

/** 送った結果を本人に見せる文。`accepted` には文が無い。 */
export function outcomeMessage(outcome: Exclude<SendOutcome, { at: "accepted" }>): string {
  return outcome.at === "rejected" ? placeRejectionMessage(outcome.kind) : UNREACHABLE_MESSAGE;
}

/**
 * 記録の束への応答を読む（D13）。**200 でも 400 でも本文の 1 件ごとの結果を読む**。
 * 1 件でも受理されなかったものがあれば、その理由を返す（束は冪等なので、押し直しで受理済みは畳まれる）。
 * 本文が読めない・5xx・401、**結果の件数が送った件数（`sent`）と違う**ときは「届かなかった」。
 * サーバは 1 件ごとに 1 結果を送った順に返すので、件数が違うのは途中の経路が本文を切ったときだけ
 * —— 受理と読むと、返らなかった項目が送れていないかもしれないまま入力を捨てる（R21）。
 * 401 を「届かなかった」と読むのは Gate の外の話で、実際の組み立てでは ST28 の Gate が先に
 * 画面全体をログインへ戻し、開いていたフォームの入力は消える（design D13（仮））。
 */
export async function readPlaceIngestResponse(res: Response, sent: number): Promise<SendOutcome> {
  if (res.status >= 500 || res.status === 401) return { at: "unreachable" };
  let body: unknown;
  try {
    body = await res.json();
  } catch {
    return { at: "unreachable" };
  }
  if (!Array.isArray(body)) return { at: "unreachable" };
  if (body.length === 0) return { at: "rejected", kind: "unknown" };
  if (body.length !== sent) return { at: "unreachable" };
  const refused = (body as { accepted?: unknown; error?: unknown }[]).find((r) => r.accepted !== true);
  if (refused === undefined) return { at: "accepted" };
  return { at: "rejected", kind: typeof refused.error === "string" ? refused.error : "unknown" };
}

/** 場所の記録の束を `POST /api/ingest` へ送る。 */
export async function sendPlaceRecords(records: BuiltPlaceRecord[]): Promise<SendOutcome> {
  const res = await fetch("/api/ingest", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(records.map((r) => ingestPlaceItem(r))),
  });
  return readPlaceIngestResponse(res, records.length);
}

/** 器を作る（`POST /api/places`。同じ利用者の同じ識別子は 200）。 */
export async function sendPlaceContainer(id: string): Promise<SendOutcome> {
  const res = await fetch("/api/places", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ id }),
  });
  return readKindResponse(res);
}

/** 登録で本人が入れたもの。座標は選んだ居た所の中心（書き換えられない）。 */
export interface RegistrationInput {
  name: string;
  lat: number;
  lon: number;
  radius_m: number;
  note: string;
}

/** 登録の 1 項目の記録と、それを組んだ入力（同じ入力なら同じ記録を送り直す）。 */
interface RegistrationPart {
  key: string;
  record: BuiltPlaceRecord;
}

/** 登録で送るもの: 器の識別子と、名前・初めての座標・広さ（・補足）の記録。 */
export interface Registration {
  placeId: string;
  records: BuiltPlaceRecord[];
  /** 項目ごとの記録（押し直しで、変わらない項目の記録を使い回すため。D13（仮）） */
  parts: Partial<Record<"name" | "coord" | "radius" | "note", RegistrationPart>>;
}

/**
 * 登録の求めを組む（D13（仮））。**押した時点で器の識別子・各記録の `id`・乱数・書いた日時が決まる**。
 * 広さは既定の 100 m でも送る（本人が選んだ値を記録に残す）。補足は入れたときだけ。
 *
 * `prev`（前に押したときに組んだもの）を渡すと、**器の識別子を保ち、入力の変わらない項目は同じ記録を、
 * 変わった項目だけ新しい記録を**組む。束は 1 件ごとに受理されるので、前の押下で受理された項目は
 * 同じ原文で送り直せばサーバで畳まれ、器を組み直すと前の器と受理済みの記録が消せないごみとして残る
 * （final review I1）。前に送った補足を空にしたら、補足を消す記録（`note: null`）にする。
 */
export function buildRegistration(input: RegistrationInput, now: Date, prev: Registration | null = null): Registration {
  const placeId = prev?.placeId ?? crypto.randomUUID();
  const parts: Registration["parts"] = {};
  const part = (spec: PlaceField): BuiltPlaceRecord => {
    const key = JSON.stringify(spec);
    const old = prev?.parts[spec.field];
    const record = old !== undefined && old.key === key ? old.record : buildPlaceRecord(spec, now, crypto.randomUUID(), placeId);
    parts[spec.field] = { key, record };
    return record;
  };
  const records = [
    part({ field: "name", name: input.name }),
    part({ field: "coord", lat: input.lat, lon: input.lon, change: "first" }),
    part({ field: "radius", radius_m: input.radius_m }),
  ];
  if (input.note.trim() !== "") records.push(part({ field: "note", note: input.note }));
  else if (prev?.parts.note !== undefined) records.push(part({ field: "note", note: null }));
  return { placeId, records, parts };
}
