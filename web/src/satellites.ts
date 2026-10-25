// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 衛星と資格情報の口（`/satellites` の 5 本。ST29 / design D11 / D12）の型と呼び手。
 *
 * **形が違えば失敗として出す**（`export.ts` と同じ形）。**資格情報の値は `issueCredential` の戻り値にだけ載る** ——
 * 保存領域・URL・ログには載せない（D6 / D12）。
 */

/** 400 / 409 の本文の種別（サーバの `SatelliteError`。`satellite_admin.rs`）。画面が日本語の理由に引く。 */
export const REFUSAL_KINDS = [
  "malformed",
  "reserved",
  "body_prefix",
  "name_invalid",
  "identifier_taken",
  "name_taken",
  "builtin",
  "removed",
] as const;

export type RefusalKind = (typeof REFUSAL_KINDS)[number];

export interface Credential {
  id: string;
  label: string;
  issued_at: string;
  last_used_at: string | null;
}

export interface Satellite {
  id: string;
  identifier: string;
  name: string;
  builtin: boolean;
  registered_at: string;
  removed_at: string | null;
  credentials: Credential[];
}

/** 発行の応答。`value` はこの 1 回だけ載る。 */
export interface Issued {
  id: string;
  label: string;
  issued_at: string;
  value: string;
}

/** 口の結果。400 / 409 は種別つき、それ以外の断りは種別なし。 */
export type Outcome<T> =
  | { at: "ok"; value: T }
  | { at: "refused"; status: number; kind: RefusalKind | null }
  | { at: "unreachable" };

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null;
const isStr = (v: unknown): v is string => typeof v === "string";

export function isRefusalKind(v: unknown): v is RefusalKind {
  return REFUSAL_KINDS.some((k) => k === v);
}

function isCredential(v: unknown): v is Credential {
  return (
    isObj(v) && isStr(v.id) && isStr(v.label) && isStr(v.issued_at) && (v.last_used_at === null || isStr(v.last_used_at))
  );
}

export function isSatellite(v: unknown): v is Satellite {
  return (
    isObj(v) &&
    isStr(v.id) &&
    isStr(v.identifier) &&
    isStr(v.name) &&
    typeof v.builtin === "boolean" &&
    isStr(v.registered_at) &&
    (v.removed_at === null || isStr(v.removed_at)) &&
    Array.isArray(v.credentials) &&
    v.credentials.every(isCredential)
  );
}

function isIssued(v: unknown): v is Issued {
  return isObj(v) && isStr(v.id) && isStr(v.label) && isStr(v.issued_at) && isStr(v.value);
}

/** 外していない衛星と外した衛星の一覧。読めなければ投げる。 */
export async function listSatellites(): Promise<Satellite[]> {
  const res = await fetch("/api/satellites");
  if (!res.ok) throw new Error(`status_${res.status}`);
  const body: unknown = await res.json();
  if (!Array.isArray(body) || !body.every(isSatellite)) throw new Error("unexpected_shape");
  return body;
}

async function call<T>(method: "POST" | "DELETE", path: string, body: unknown, ok: (v: unknown) => v is T): Promise<Outcome<T>> {
  try {
    const res = await fetch(path, {
      method,
      headers: body === undefined ? {} : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    if (!res.ok) {
      let kind: RefusalKind | null = null;
      try {
        const err: unknown = await res.json();
        if (isObj(err) && isRefusalKind(err.error)) kind = err.error;
      } catch {
        // 本体が読めなくても、状態の番号で種類は分かる
      }
      return { at: "refused", status: res.status, kind };
    }
    if (res.status === 204) return { at: "ok", value: undefined as T };
    const value: unknown = await res.json();
    return ok(value) ? { at: "ok", value } : { at: "unreachable" };
  } catch {
    return { at: "unreachable" };
  }
}

const isNothing = (v: unknown): v is undefined => v === undefined;

export function registerSatellite(name: string, identifier: string): Promise<Outcome<Satellite>> {
  return call("POST", "/api/satellites", { name, identifier }, isSatellite);
}

export function removeSatellite(id: string): Promise<Outcome<undefined>> {
  return call("DELETE", `/api/satellites/${id}`, undefined, isNothing);
}

export function issueCredential(satellite: string, label: string): Promise<Outcome<Issued>> {
  return call("POST", `/api/satellites/${satellite}/credentials`, { label }, isIssued);
}

export function revokeCredential(satellite: string, credential: string): Promise<Outcome<undefined>> {
  return call("DELETE", `/api/satellites/${satellite}/credentials/${credential}`, undefined, isNothing);
}
