// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };

/**
 * **`Secure` の cookie が `http://127.0.0.1` で Chromium に残るか**（design D10 / Risks）。
 * 残らないなら `playwright.config.ts` の `baseURL` を `http://localhost` にする。
 */
test.use({ storageState: { cookies: [], origins: [] } });

test("Secure のログインの印が http://127.0.0.1 で残る", async ({ page }) => {
  // Node 側の `request` は cookie の扱いが別物なので、**ブラウザの fetch** で確かめる
  await page.goto("/");
  const login = await page.evaluate(async (password) => {
    const r = await fetch("/api/session", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ password }),
    });
    return r.status;
  }, process.env.WEB_PASSWORD);
  expect(login).toBe(204);
  const mark = (await page.context().cookies()).find((c) => c.name === "ashiato_session");
  expect(mark?.secure).toBe(true);
  expect(mark?.httpOnly).toBe(true);
  // 残った印で読み出しが通る（残っていなければ 401）
  const after = await page.evaluate(async () => (await fetch("/api/session")).status);
  expect(after).toBe(200);
});
