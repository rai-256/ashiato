// SPDX-License-Identifier: AGPL-3.0-only
import { defineConfig, devices } from "@playwright/test";

/**
 * 録画用（人間が後から動画で見る）。通常の e2e（`playwright.config.ts`）は変えない。
 * 置き場はここ（`tools/recording/st22/`）で、`tools/recording/record-st22.sh` が録画のたびに
 * 対象コミットの `web/` へ写してから走らせる（対象コミットに録画用のファイルが無くても撮れるように）。
 *
 * 起動はしない —— 画面とサーバは別の場所（WSL の `tools/stack.sh up`）で立っている前提で、
 * `REC_BASE_URL` に向けて走らせる。Windows 側の playwright から WSL の 127.0.0.1 を叩くための形。
 * 合否はアサーションが決める。動画と trace は見るための材料で、比較には使わない。
 */
declare const process: { env: Record<string, string | undefined> };

export default defineConfig({
  testDir: ".",
  testMatch: ["e2e/day-erase.spec.ts", "e2e-recording/st22-erase-reload.rec.ts"],
  outputDir: process.env.REC_OUT ?? "./recording-results",
  fullyParallel: false,
  workers: 1,
  retries: 0, // 落ちたら落ちたままにする（再試行で緑にしない）
  reporter: [["list"], ["json", { outputFile: `${process.env.REC_OUT ?? "./recording-results"}/results.json` }],
    ["html", { open: "never", outputFolder: process.env.REC_REPORT ?? "./recording-report" }]],
  globalSetup: "./e2e/global-setup.ts",
  use: {
    storageState: "./e2e/.auth/state.json",
    baseURL: process.env.REC_BASE_URL ?? "http://127.0.0.1:5190",
    video: {
      mode: "on",
      size: { width: 1280, height: 720 },
      // 操作の注釈（何をクリックしたか）とテスト名を動画に焼き込む
      show: {
        actions: { position: "top-right", cursor: "pointer", fontSize: 20, duration: 1500 },
        test: { level: "step", position: "bottom-left", fontSize: 24 },
      },
    },
    trace: "on",
    screenshot: "on",
    launchOptions: { slowMo: Number(process.env.REC_SLOWMO_MS ?? 250) },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 720 } } }],
});
