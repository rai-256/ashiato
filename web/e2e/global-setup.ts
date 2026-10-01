// SPDX-License-Identifier: AGPL-3.0-only
import { request, type FullConfig } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };

/** 全 project の既定の `storageState`（ログイン済み）の置き場。git の管理外。 */
export const STORAGE_STATE = "./e2e/.auth/state.json";

/**
 * 1 回だけログインして印を `storageState` に残す（ST28 / design D10）。既存の e2e は既定の `page` を使うだけで、
 * ログインを知らずに通る。ログインそのものの e2e は `test.use({ storageState: { cookies: [], origins: [] } })` で未ログインから始める。
 * 合言葉は `.env` の `WEB_PASSWORD`（`tools/stack.sh` と同じ値）。
 */
export default async function globalSetup(config: FullConfig): Promise<void> {
  const password = process.env.WEB_PASSWORD;
  if (!password) throw new Error("WEB_PASSWORD が無い（set -a; . ./.env; set +a の後で走らせる）");
  const baseURL = config.projects[0]?.use.baseURL;
  const ctx = await request.newContext({ baseURL });
  const res = await ctx.post("/api/session", { data: { password } });
  if (res.status() !== 204) throw new Error(`ログインできない: status ${res.status()}`);
  await ctx.storageState({ path: STORAGE_STATE });
  await ctx.dispose();
}
