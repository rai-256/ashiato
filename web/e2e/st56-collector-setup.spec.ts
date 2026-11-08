// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test } from "@playwright/test";
import { parseSetupString } from "../src/collectorSetup";

declare const process: { env: Record<string, string | undefined> };

const password = (): string => {
  const value = process.env.WEB_PASSWORD;
  if (!value) throw new Error("WEB_PASSWORD が無い");
  return value;
};

async function openSettings(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/#/settings");
  await expect(page.getByTestId("collector-setup-section")).toBeVisible();
}

// Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る
test("合言葉を入れ直すと、設定の文字列とコピーのボタンが出る", async ({ page }) => {
  await openSettings(page);
  const section = page.getByTestId("collector-setup-section");
  await section.getByLabel("合言葉").fill(password());
  await section.getByRole("button", { name: "表示" }).click();

  const value = section.getByTestId("collector-setup-value");
  await expect(value).toBeVisible();
  const parsed = parseSetupString(await value.inputValue());
  expect(parsed).toEqual({
    url: `${new URL(page.url()).origin}/api`,
    token: process.env.API_TOKEN ?? "dev-token-0123456789abcdef",
  });
  await expect(section.getByRole("button", { name: "値をコピー" })).toBeVisible();
  await expect(section.getByLabel("利用者")).toHaveCount(0);
});

// Scenario: 合言葉が違えば出さない
test("合言葉が違えば出さない", async ({ page }) => {
  await openSettings(page);
  const section = page.getByTestId("collector-setup-section");
  await section.getByLabel("合言葉").fill("wrong-password-0123456789");
  const response = page.waitForResponse((item) => item.url().endsWith("/api/collector-setup") && item.request().method() === "POST");
  await section.getByRole("button", { name: "表示" }).click();
  expect((await response).status()).toBe(403);
  await expect(section.getByRole("alert")).toHaveText("合言葉が違います。");
  await expect(page).toHaveURL(/#\/settings$/);
  await expect(page.getByTestId("login")).toHaveCount(0);
  await expect(section.getByTestId("collector-setup-value")).toHaveCount(0);
  await expect(section.getByLabel("合言葉")).toBeVisible();
});

// Scenario: 別の画面へ移って戻ると設定の文字列は消える
test("別の画面へ移って戻ると設定の文字列は消える", async ({ page }) => {
  await openSettings(page);
  const section = page.getByTestId("collector-setup-section");
  await section.getByLabel("合言葉").fill(password());
  await section.getByRole("button", { name: "表示" }).click();
  await expect(section.getByTestId("collector-setup-value")).toBeVisible();

  await page.goto("/#/coverage");
  await page.goto("/#/settings");
  const returnedSection = page.getByTestId("collector-setup-section");
  await expect(returnedSection.getByLabel("合言葉")).toBeVisible();
  await expect(returnedSection.getByTestId("collector-setup-value")).toHaveCount(0);
});

// Scenario: 開き直すと設定の文字列は消える
test("開き直すと設定の文字列は消える", async ({ page }) => {
  await openSettings(page);
  const section = page.getByTestId("collector-setup-section");
  await section.getByLabel("合言葉").fill(password());
  await section.getByRole("button", { name: "表示" }).click();
  await expect(section.getByTestId("collector-setup-value")).toBeVisible();

  await page.reload();
  const reloadedSection = page.getByTestId("collector-setup-section");
  await expect(reloadedSection.getByLabel("合言葉")).toBeVisible();
  await expect(reloadedSection.getByTestId("collector-setup-value")).toHaveCount(0);
});
