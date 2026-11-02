import { readFileSync } from "node:fs";

type StorageState = { cookies?: { name: string; value: string }[] };

/** Secure cookie は Node の http request へ自動では付かないため、storageState の印を明示する。 */
export function readAuthHeaders(): { cookie: string } {
  const state = JSON.parse(readFileSync(new URL("./.auth/state.json", import.meta.url), "utf8")) as StorageState;
  const cookies = state.cookies ?? [];
  if (cookies.length === 0) throw new Error("storageState にログインの印が無い");
  return { cookie: cookies.map(({ name, value }) => `${name}=${value}`).join("; ") };
}
