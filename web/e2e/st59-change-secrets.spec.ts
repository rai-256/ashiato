// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test, type Page } from "@playwright/test";
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { parseSetupString } from "../src/collectorSetup";

declare const process: { env: Record<string, string | undefined> };

const password = "st59-test-password-012345";
const newPassword = "st59-new-password-012345";
const root = "..";
const settingsPort = Number(execFileSync("bash", ["-c", ". ../tools/ports.sh; printf '%s' \"$ASHIATO_HTTP_PORT\""], { encoding: "utf8" })) + 1;

type SettingsServer = { dir: string; settings: string; process: { kill(): void } };
let server: SettingsServer | undefined;

async function startSettingsServer(dir: string): Promise<void> {
  execFileSync("bash", ["../tools/settings-server.sh", "make", dir], { encoding: "utf8" });
  const settings = `${dir}/settings/server.json`;
  const pidFile = `${dir}/settings-server.pid`;
  const child = spawn("bash", ["tools/settings-server.sh", "up", settings], {
    cwd: root,
    env: { ...process.env, ASHIATO_SETTINGS_SERVER_PID_FILE: pidFile },
    stdio: "ignore",
  });
  server = { dir, settings, process: child };
  for (let i = 0; i < 30; i++) {
    try { await fetch(`http://127.0.0.1:${settingsPort}/healthz`); return; } catch { await new Promise((resolve) => setTimeout(resolve, 200)); }
  }
  throw new Error("設定ファイル型サーバが起動しない");
}

function stopSettingsServer(): void {
  if (server === undefined) return;
  execFileSync("bash", ["../tools/settings-server.sh", "down"], { encoding: "utf8", env: { ...process.env, ASHIATO_SETTINGS_SERVER_PID_FILE: `${server.dir}/settings-server.pid` } });
  server.process.kill();
  server = undefined;
}

async function openSettings(page: Page, base = `http://127.0.0.1:${settingsPort}`): Promise<void> {
  await page.goto(`${base}/#/settings`);
  await expect(page.getByLabel("今の合言葉").first()).toBeVisible();
}

async function login(page: Page, value: string, base = `http://127.0.0.1:${settingsPort}`): Promise<void> {
  await page.goto(`${base}/#/coverage`);
  await page.getByLabel("合言葉").fill(value);
  await page.getByRole("button", { name: "ログイン" }).click();
  await expect(page.getByRole("link", { name: "設定" })).toBeVisible();
}

test.use({ storageState: { cookies: [], origins: [] } });
test.beforeEach(async ({ browserName }, testInfo) => startSettingsServer(`${testInfo.outputDir}/${browserName}/settings-server`));
test.afterEach(() => stopSettingsServer());

// Scenario: 今の合言葉を入れ直すと画面の合言葉が変わり、前の合言葉ではログインできない
test("今の合言葉を入れ直すと画面の合言葉が変わり、前の合言葉ではログインできない", async ({ browser }) => {
  const page = await browser.newPage(); await login(page, password); await openSettings(page);
  const section = page.getByTestId("web-password-change-section");
  await section.getByLabel("今の合言葉").fill(password); await section.getByLabel("新しい合言葉", { exact: true }).fill(newPassword); await section.getByLabel("新しい合言葉（もう一度）").fill(newPassword); await section.getByRole("button", { name: "変える" }).click();
  await expect(section.getByRole("status")).toContainText("合言葉を変えました");
  const old = await browser.newPage(); await login(old, password).catch(() => undefined); await expect(old.getByLabel("合言葉")).toBeVisible();
  const fresh = await browser.newPage(); await login(fresh, newPassword); await expect(fresh.getByRole("link", { name: "設定" })).toBeVisible();
});

// Scenario: 今の合言葉が違えば、画面の合言葉は変わらない
test("今の合言葉が違えば、画面の合言葉は変わらない", async ({ page }) => {
  await login(page, password); await openSettings(page); const section = page.getByTestId("web-password-change-section");
  const response = page.waitForResponse((item) => item.url().endsWith("/api/web-password"));
  await section.getByLabel("今の合言葉").fill("st59-wrong-password-012345"); await section.getByLabel("新しい合言葉", { exact: true }).fill(newPassword); await section.getByLabel("新しい合言葉（もう一度）").fill(newPassword); await section.getByRole("button", { name: "変える" }).click();
  expect((await response).status()).toBe(403); await expect(section.getByRole("alert")).toHaveText("合言葉が違います");
});

// Scenario: 合言葉を変えると、ほかのブラウザのログインは切れ、変えた画面はログインしたまま
test("合言葉を変えると、ほかのブラウザのログインは切れ、変えた画面はログインしたまま", async ({ browser }) => {
  const firstContext = await browser.newContext({ storageState: { cookies: [], origins: [] } }); const secondContext = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const first = await firstContext.newPage(); const second = await secondContext.newPage(); await login(first, password); await login(second, password); await openSettings(first);
  const section = first.getByTestId("web-password-change-section"); await section.getByLabel("今の合言葉").fill(password); await section.getByLabel("新しい合言葉", { exact: true }).fill(newPassword); await section.getByLabel("新しい合言葉（もう一度）").fill(newPassword); await section.getByRole("button", { name: "変える" }).click(); await expect(section.getByRole("status")).toBeVisible();
  await first.goto(`http://127.0.0.1:${settingsPort}/#/coverage`); await expect(first.getByTestId("coverage-view")).toBeVisible(); await second.goto(`http://127.0.0.1:${settingsPort}/#/coverage`); await second.reload(); await expect(second.getByLabel("合言葉")).toBeVisible();
});

// Scenario: 今の合言葉を入れ直すと、新しい値の設定の文字列とコピーのボタンが出る
test("今の合言葉を入れ直すと、新しい値の設定の文字列とコピーのボタンが出る", async ({ page }) => {
  await login(page, password); await openSettings(page); const section = page.getByTestId("collector-token-change-section"); await section.getByLabel("今の合言葉").fill(password); await section.getByRole("button", { name: "変える" }).click();
  const parsed = parseSetupString(await section.getByTestId("collector-token-change-value").inputValue()); expect(parsed?.url).toBe(`http://127.0.0.1:${settingsPort}/api`); expect(parsed?.token).toBeTruthy(); await expect(section.getByRole("button", { name: "値をコピー" })).toBeVisible(); await expect(section.getByLabel("利用者")).toHaveCount(0);
});

// Scenario: 別の画面へ移って戻ると、新しい値の設定の文字列は消える
test("別の画面へ移って戻ると、新しい値の設定の文字列は消える", async ({ page }) => {
  await login(page, password); await openSettings(page); const section = page.getByTestId("collector-token-change-section"); await section.getByLabel("今の合言葉").fill(password); await section.getByRole("button", { name: "変える" }).click(); await expect(section.getByTestId("collector-token-change-value")).toBeVisible(); await page.goto(`http://127.0.0.1:${settingsPort}/#/coverage`); await openSettings(page); await expect(page.getByTestId("collector-token-change-value")).toHaveCount(0);
});

// Scenario: 読み込み直すと、新しい値の設定の文字列は消える
test("読み込み直すと、新しい値の設定の文字列は消える", async ({ page }) => {
  await login(page, password); await openSettings(page); const section = page.getByTestId("collector-token-change-section"); await section.getByLabel("今の合言葉").fill(password); await section.getByRole("button", { name: "変える" }).click(); await expect(section.getByTestId("collector-token-change-value")).toBeVisible(); await page.reload(); await expect(page.getByTestId("collector-token-change-value")).toHaveCount(0);
});

// Scenario: 設定に 2 つの節が並び、落としたときは両方を変えると書いてある
test("設定に 2 つの節が並び、落としたときは両方を変えると書いてある", async ({ page }) => {
  await login(page, password); await openSettings(page); await expect(page.getByText("携帯を落としたときは両方を変える")).toBeVisible(); await expect(page.getByRole("heading", { name: "画面の合言葉を変える" })).toBeVisible(); await expect(page.getByRole("heading", { name: "収集アプリの値を変える" })).toBeVisible();
});

// Scenario: 設定のファイルに書けないと、画面は合言葉が変わっていないと出す
test("設定のファイルに書けないと、画面は合言葉が変わっていないと出す", async ({ page }) => {
  if (server === undefined) throw new Error("設定サーバが無い"); mkdirSync(`${server.settings}.writing`);
  await login(page, password); await openSettings(page); const section = page.getByTestId("web-password-change-section"); await section.getByLabel("今の合言葉").fill(password); await section.getByLabel("新しい合言葉", { exact: true }).fill(newPassword); await section.getByLabel("新しい合言葉（もう一度）").fill(newPassword); await section.getByRole("button", { name: "変える" }).click(); await expect(section.getByRole("alert")).toHaveText("保存できませんでした。合言葉は変わっていません");
});

// Scenario: 開発の形のサーバでは、画面から変えられないと出る
test("開発の形のサーバでは、画面から変えられないと出る", async ({ page }) => {
  stopSettingsServer(); const base = `http://127.0.0.1:${process.env.WEB_PORT ?? "5180"}`; await login(page, process.env.WEB_PASSWORD ?? "", base); await openSettings(page, base);
  for (const id of ["web-password-change-section", "collector-token-change-section"]) { const section = page.getByTestId(id); await section.getByLabel("今の合言葉").fill(process.env.WEB_PASSWORD ?? ""); await section.getByRole("button", { name: "変える" }).click(); await expect(section.getByRole("alert")).toHaveText("この形のサーバでは画面から変えられません。.env を書き換えて起動し直してください"); }
});
