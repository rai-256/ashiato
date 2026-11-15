import { expect, test, type Page } from "@playwright/test";

const status = (overrides: Record<string, unknown> = {}): Record<string, unknown> => ({ state: "available", installed: "0.4.0", latest: "0.5.0", found_at: "2026-11-14T00:00:00Z", checked_at: "2026-11-15T00:00:00Z", last_ok_at: "2026-11-15T00:00:00Z", reason: null, last_update: null, ...overrides });
const serve = async (page: Page, value: Record<string, unknown>): Promise<void> => { await page.route("**/api/update-status", (route) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) })); };
const last = (result: string, reason: Record<string, unknown> | null): Record<string, unknown> => ({ at: "2026-11-15T00:00:00Z", result, to_version: "0.5.0", from_version: "0.4.0", reason });

// Scenario: 帯を押すと、サーバの更新の手順が開く
test("帯を押すと、サーバの更新の手順が開く", async ({ page, baseURL }) => { await serve(page, status()); const origin = new URL(baseURL!).origin; const offsite: string[] = []; page.on("request", (request) => { if (new URL(request.url()).origin !== origin) offsite.push(request.url()); }); await page.goto("/#/coverage"); await page.getByTestId("update-available").click(); const procedure = page.getByTestId("update-procedure"); await expect(procedure).toContainText("サーバの更新の手順"); await expect(procedure).toContainText("0.5.0"); await expect(procedure.locator("li")).toHaveCount(3); await expect(procedure).toContainText("移行が落ちたら自動で前の版に戻ります"); await expect(procedure.locator("a")).toHaveAttribute("href", "https://github.com/rai-256/ashiato/releases/tag/v0.5.0"); expect(offsite).toHaveLength(0); });

// Scenario: 開発版の手順のリンクは印 nightly のページを指す
test("開発版の手順のリンクは印 nightly のページを指す", async ({ page }) => { await serve(page, status({ latest: "nightly" })); await page.goto("/#/coverage"); await page.getByTestId("update-available").click(); await expect(page.getByTestId("update-procedure").locator("a")).toHaveAttribute("href", "https://github.com/rai-256/ashiato/releases/tag/nightly"); });

// Scenario: 戻した更新は、帯と設定に出る
test("戻した更新は、帯と設定に出る", async ({ page }) => { const value = status({ last_update: last("rolled_back", { kind: "aborted", failed_version: "V1", sqlstate: "23505" }) }); await serve(page, value); await page.goto("/#/coverage"); await expect(page.getByTestId("update-band")).toContainText("更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました"); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("V1・23505（制約に反した）"); });

// Scenario: 入らなかった更新は、帯と設定に出る
test("入らなかった更新は、帯と設定に出る", async ({ page }) => { const value = status({ state: "latest", latest: "0.4.0", last_update: last("not_applied", { kind: "db_ahead", failed_version: null, sqlstate: null }) }); await serve(page, value); await page.goto("/#/coverage"); await expect(page.getByTestId("update-band")).toContainText("更新 0.5.0 は入りませんでした（DB に新しい版の知らない版がある）。0.4.0 のまま動いています"); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("入りませんでした"); });

// Scenario: 次に更新が通ると、戻した知らせは帯から消える
test("次に更新が通ると、戻した知らせは帯から消える", async ({ page }) => { let value = status({ last_update: last("rolled_back", { kind: "aborted", failed_version: "V1", sqlstate: null }) }); await page.route("**/api/update-status", (route) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) })); await page.goto("/#/coverage"); await expect(page.getByTestId("update-band")).toContainText("戻しました"); value = status(); await page.reload(); await expect(page.getByTestId("update-band")).not.toContainText("戻しました"); });

// Scenario: 戻した知らせと更新ありと確かめられていないが同時なら 3 行出る
test("戻した知らせと更新ありと確かめられていないが同時なら 3 行出る", async ({ page }) => { await serve(page, status({ state: "available_stale", last_ok_at: "2026-11-07T00:00:00Z", reason: "unreachable", last_update: last("rolled_back", { kind: "aborted", failed_version: "V1", sqlstate: null }) })); await page.setViewportSize({ width: 360, height: 640 }); await page.goto("/#/coverage"); await expect(page.getByTestId("update-band").locator(":scope > *")).toHaveCount(3); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); });

// Scenario: 戻した・入らなかったの行を押しても何も起きない
test("戻した・入らなかったの行を押しても何も起きない", async ({ page }) => { await serve(page, status({ state: "latest", latest: "0.4.0", last_update: last("not_applied", { kind: "db_ahead", failed_version: null, sqlstate: null }) })); await page.goto("/#/coverage"); const before = page.url(); await page.getByTestId("update-band").click(); expect(page.url()).toBe(before); await expect(page.getByTestId("update-procedure")).toHaveCount(0); });
