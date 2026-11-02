import { readFileSync } from "node:fs";

type StorageState = { cookies?: { name: string; value: string }[] };

let cached: { cookie: string } | undefined;

/**
 * Secure cookie は Node の http request へ自動では付かないため、storageState の印を明示する。
 * **初めて呼ばれたときに読み、結果を持つ**（ST44 final review R8）。モジュールの読み込みでは読まない ——
 * `--list`・UI・watch は globalSetup（`.auth/state.json` を作る）の前に試験を読むので、新しい checkout で落ちる。
 * 呼ぶ側も試験の中（fixture・関数）で呼び、モジュールの頭で値にしない。
 */
export function readAuthHeaders(): { cookie: string } {
  if (cached) return cached;
  const state = JSON.parse(readFileSync(new URL("./.auth/state.json", import.meta.url), "utf8")) as StorageState;
  const cookies = state.cookies ?? [];
  if (cookies.length === 0) throw new Error("storageState にログインの印が無い");
  cached = { cookie: cookies.map(({ name, value }) => `${name}=${value}`).join("; ") };
  return cached;
}
