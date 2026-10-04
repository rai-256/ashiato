// SPDX-License-Identifier: AGPL-3.0-only
import { defineConfig, devices } from "@playwright/test";

/**
 * 録画用（人間が後から動画で見る）。通常の e2e（`web/playwright.config.ts`）は変えない。
 *
 * ハーネスの `scripts/record-run` が、録画のたびに対象コミットの `web/playwright.recording.config.ts` へ写し、
 * 次の環境変数を渡して走らせる（宣言は `tools/recording/recording.json`）:
 *   REC_TEST_MATCH  走らせる spec（JSON の配列。web/ から）
 *   REC_BASE_URL    tools/record-env.sh が立てた画面
 *   REC_OUT / REC_REPORT  記録フォルダの中の出力先
 *   WEB_PASSWORD    ログインの合言葉（tools/record-env.sh が乱数で作る。global-setup が読む）
 * 起動はしない（画面とサーバは tools/record-env.sh が立てている）。
 * 合否はアサーションが決める。動画と trace は見るための材料で、比較には使わない。
 */
declare const process: { env: Record<string, string | undefined> };

const OUT = process.env.REC_OUT ?? "./recording-results";

export default defineConfig({
  testDir: ".",
  testMatch: JSON.parse(process.env.REC_TEST_MATCH ?? "[]") as string[],
  outputDir: OUT,
  fullyParallel: false,
  workers: 1,
  retries: 0, // 落ちたら落ちたままにする（再試行で緑にしない）
  forbidOnly: true, // 対象コミットに test.only が残っていたら走らせない（一部だけ走って緑になるのを防ぐ）
  reporter: [["list"], ["json", { outputFile: `${OUT}/results.json` }],
    ["html", { open: "never", outputFolder: process.env.REC_REPORT ?? "./recording-report" }]],
  globalSetup: "./e2e/global-setup.ts",
  use: {
    storageState: "./e2e/.auth/state.json",
    baseURL: process.env.REC_BASE_URL,
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
