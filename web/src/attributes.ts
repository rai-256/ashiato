// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 個人属性（`GET /attributes`。ST19 / design D9）の型と、主張の原文の組み立て。
 *
 * **原文は画面が組む**（design D1 / D4）—— サーバは受け取ったまま保存するので、
 * サーバが乱数を足すと原文が変わって「1 バイトも変わらずに残る」が成り立たない。
 */

export type Precision = "year" | "month" | "day" | "unknown";

export interface ValidFrom {
  precision: Precision;
  /** `2019` / `2019-10` / `2019-10-01`。精度が `unknown` なら `null` */
  date: string | null;
}

export interface Claim {
  id: string;
  /** `null` が「なし」（その属性が終わった） */
  value: string | null;
  valid_from: ValidFrom;
  /** RFC 3339（地域のずれつき）。**押したときだけ出す**（Q2） */
  asserted_at: string;
  ingested_at: string;
  supersedes: string | null;
  superseded_by: string | null;
  note: string | null;
}

export interface KindView {
  id: string;
  name: string;
  current: Claim | null;
  upcoming: Claim[];
  claims: Claim[];
  superseded: Claim[];
}

export interface AttributesView {
  today: string;
  kinds: KindView[];
}

const PRECISIONS: Precision[] = ["year", "month", "day", "unknown"];

/** 応答が個人属性の形をしているか。**形が違えば失敗として出す** —— 型の宣言だけで通すと描画で落ちて画面が白くなる。 */
export function isAttributesView(v: unknown): v is AttributesView {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return typeof o.today === "string" && Array.isArray(o.kinds) && o.kinds.every(isKindView);
}

function isKindView(v: unknown): boolean {
  if (typeof v !== "object" || v === null) return false;
  const k = v as Record<string, unknown>;
  const lists = ["upcoming", "claims", "superseded"] as const;
  return (
    typeof k.id === "string" &&
    typeof k.name === "string" &&
    (k.current === null || isClaim(k.current)) &&
    lists.every((name) => Array.isArray(k[name]) && (k[name] as unknown[]).every(isClaim))
  );
}

function isClaim(v: unknown): boolean {
  if (typeof v !== "object" || v === null) return false;
  const c = v as Record<string, unknown>;
  const vf = c.valid_from as Record<string, unknown> | undefined;
  return (
    typeof c.id === "string" &&
    (c.value === null || typeof c.value === "string") &&
    typeof c.asserted_at === "string" &&
    typeof vf === "object" &&
    vf !== null &&
    PRECISIONS.includes(vf.precision as Precision) &&
    (vf.date === null || typeof vf.date === "string")
  );
}

/** 「いつから」を読める文にする。**精度を丸めない**（深掘り C3）—— 年だけの主張を「1 月 1 日」と書かない。 */
export function validFromLabel(vf: ValidFrom): string {
  if (vf.precision === "unknown" || vf.date === null) return "いつからかは分からない";
  const [y, m, d] = vf.date.split("-").map(Number);
  if (vf.precision === "year") return `${y} 年から`;
  if (vf.precision === "month") return `${y} 年 ${m} 月から`;
  return `${y} 年 ${m} 月 ${d} 日から`;
}

/** 「書いた日時」を読める文にする。**押したときだけ出る**（Q2）。 */
export function assertedAtLabel(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  const f = new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  return `${f.format(at)} に書いた`;
}

/** 値の書き方。`null`（「なし」）を「まだ書いていない」と混ぜない。 */
export function valueLabel(value: string | null): string {
  return value === null ? "なし" : value;
}

/**
 * 主張ごとの乱数（design D4 / 深掘り C12）。**128 bit を base64url で 22 文字**。
 *
 * **識別子から導かない** —— `id` の列は消去の後も残るので、`id` から導けた瞬間に
 * 乱数は鍵を守らなくなる。`crypto.getRandomValues` で毎回新しく引く。
 */
export function newNonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** 「書く」で本人が入れたもの。原文を組む入力。 */
export interface ClaimInput {
  kind: string;
  /** `null` が「なし」 */
  value: string | null;
  precision: Precision;
  /** 精度ごとの欄。`unknown` なら使わない */
  year: string;
  month: string;
  day: string;
  note: string;
  /** 「前の書き込みが間違っていた」で選んだ取り消す主張 */
  supersedes: string | null;
}

/** 入力の精度と欄から「いつから」を組む。**空の欄は `null`**（サーバが `invalid_valid_from` で断る）。 */
export function validFromOf(input: ClaimInput): ValidFrom {
  const { precision, year, month, day } = input;
  if (precision === "unknown") return { precision, date: null };
  const y = year.padStart(4, "0");
  if (precision === "year") return { precision, date: year === "" ? null : y };
  const m = month.padStart(2, "0");
  if (precision === "month") {
    return { precision, date: year === "" || month === "" ? null : `${y}-${m}` };
  }
  const d = day.padStart(2, "0");
  return {
    precision,
    date: year === "" || month === "" || day === "" ? null : `${y}-${m}-${d}`,
  };
}

/** 送る 1 件（原文と、エンベロープの 2 つの時刻）。 */
export interface BuiltClaim {
  id: string;
  raw: string;
  /** 主張した日時（「積む」を押した時刻。深掘り C4） */
  assertedAt: string;
  tzOffsetMin: number;
  tzId: string;
}

/**
 * 原文を組む（design D1 の形）。
 *
 * **押した時点で `id` / 乱数 / 主張した日時が決まる**（深掘り C4）——
 * 本人が主張した日時を入力する欄は置かない。ずらせると、2 つの時刻を分けた意味が消える。
 *
 * **入力を変えずに押し直したときは、これをもう一度呼ばずに同じ `BuiltClaim` を送る**
 * （組み直すと乱数が変わり、サーバは畳めずに 2 件になる）。
 */
export function buildClaim(input: ClaimInput, now: Date, id: string, nonce: string): BuiltClaim {
  const raw = JSON.stringify({
    claim: id,
    nonce,
    kind: input.kind,
    value: input.value,
    valid_from: validFromOf(input),
    supersedes: input.supersedes,
    note: input.note.trim() === "" ? null : input.note,
  });
  // **地域は端末のもの**（design D1 の `tz_offset_min` / `tz_id`）。
  // `getTimezoneOffset` は「UTC − 現地」なので符号を反転する。
  const tzOffsetMin = -now.getTimezoneOffset();
  const tzId = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return { id, raw, assertedAt: now.toISOString(), tzOffsetMin, tzId };
}

/** 取り込み口へ送る 1 件（`POST /api/ingest` の本文の要素）。 */
export function ingestItem(built: BuiltClaim, userId?: string): unknown {
  return {
    id: built.id,
    user_id: userId ?? "00000000-0000-0000-0000-000000000000",
    logical_source: "s01-attribute",
    external_id: null,
    device_id: null,
    origin: "authored",
    event_time: built.assertedAt,
    tz_offset_min: built.tzOffsetMin,
    tz_id: built.tzId,
    schema_version: 1,
    raw: built.raw,
    payload: {},
  };
}

/** 送った結果。**「断られた」と「届かなかった」を分ける**（design D9 / spec-review R19）。 */
export type SendOutcome =
  | { at: "accepted" }
  | { at: "rejected"; kind: string }
  | { at: "unreachable" };

/**
 * `/ingest` の応答を読む（design D9）。
 *
 * **1 件だけ送って断られると `/ingest` は HTTP 400 を返す**（1 件も受け付けなかったとき）。
 * 既存の読み出しの形（`if (!res.ok) throw`）を写すと、**断られた理由が全部「届かなかった」に化ける**。
 * **200 でも 400 でも本文の 1 件ごとの結果を読む。**
 *
 * 「届かなかった」にするのは、本文が読めない・5xx・401・`fetch` が投げたときだけ。
 */
export async function readIngestResponse(res: Response): Promise<SendOutcome> {
  if (res.status >= 500 || res.status === 401) return { at: "unreachable" };
  let body: unknown;
  try {
    body = await res.json();
  } catch {
    return { at: "unreachable" };
  }
  if (!Array.isArray(body) || body.length === 0) return { at: "unreachable" };
  const first = body[0] as { accepted?: unknown; error?: unknown };
  if (first.accepted === true) return { at: "accepted" };
  return { at: "rejected", kind: typeof first.error === "string" ? first.error : "unknown" };
}

/** 断られた理由を、本人に読める文にする（design D9 の表）。 */
export function rejectionMessage(kind: string): string {
  switch (kind) {
    case "invalid_claim_value":
      return "値が空です。値を入れるか「なし」を選んでください";
    case "invalid_valid_from":
      return "「いつから」の日付が読めません";
    case "unknown_attribute_kind":
      return "この種類が見つかりません。画面を読み直してください";
    case "invalid_supersedes":
      return "取り消す主張が見つかりません。画面を読み直してください";
    default:
      return `受け付けられませんでした（${kind}）`;
  }
}

/** サーバに届かなかったときの文。**断られたときの文と違うもの**（spec）。 */
export const UNREACHABLE_MESSAGE = "サーバに届きませんでした。入力はそのまま残っています";

/** 「書く」の初期値。 */
export function emptyInput(kind: string): ClaimInput {
  return {
    kind,
    value: "",
    precision: "month",
    year: "",
    month: "",
    day: "",
    note: "",
    supersedes: null,
  };
}
