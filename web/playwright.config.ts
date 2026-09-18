// SPDX-License-Identifier: AGPL-3.0-only
import { defineConfig, devices } from "@playwright/test";

/**
 * 画面の e2e（製造準備 B の工具）。**器だけがここにあり、テストは Story ごとの change が足す。**
 *
 * 起動は `tools/stack.sh` —— 確認バッチの `run.sh` が呼ぶのと**同じもの**。分けると
 * 「e2e は緑なのに人間が見る画面は違う」が起きる（2026-09-18）。
 *
 * **視覚回帰（toHaveScreenshot）は使わない。** 差分の是非を毎回人間が判断することになり、
 * 機械に移したはずの判断が人間へ戻る。アサートするのは**数値と経路** ——
 * 実寸（boundingBox）・スクロール量・フォーカスの位置（activeElement）・可視・URL 遷移。
 * 失敗時の trace とスクショは残すが、比較には使わない。
 */
// `@types/node` は入れない —— DOM 前提の src の型（setTimeout の戻り値など）を動かしたくない。
// ここで使う分だけを宣言する。
declare const process: { env: Record<string, string | undefined> };

const PORT = Number(process.env.WEB_PORT ?? 5180);

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // 1 つの DB を共有する。並列にすると偽データが互いを踏む
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0, // 落ちたら落ちたままにする（再試行で緑にしない）
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "bash ../tools/stack.sh up",
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: !process.env.CI, // 手元で run.sh を立てたままでも走らせられる
    timeout: 10 * 60 * 1000, // 初回は cargo build --release が入る
    stdout: "pipe",
    stderr: "pipe",
  },
});
