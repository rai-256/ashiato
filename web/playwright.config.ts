// SPDX-License-Identifier: AGPL-3.0-only
import { execFileSync } from "node:child_process";
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

// サーバ（tools/stack.sh が立てる）の待ち受け。**tools/ports.sh と同じ規則で組む** —— stack.sh も BIND を
// `127.0.0.1:<ports.sh の ASHIATO_HTTP_PORT>` に上書きするので、`.env` の BIND や CI の env に頼ると食い違う
// （ST52 final review R2: CI の e2e には BIND が無く、サーバが配る画面の e2e が必ず落ちていた）。
// 試験の側（worker）は環境変数 ASHIATO_E2E_SERVER_BIND で受ける。
const SERVER_BIND = execFileSync("bash", ["-c", '. ../tools/ports.sh && printf "127.0.0.1:%s" "$ASHIATO_HTTP_PORT"'], {
  encoding: "utf8",
}).trim();
process.env.ASHIATO_E2E_SERVER_BIND = SERVER_BIND;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // 1 つの DB を共有する。並列にすると偽データが互いを踏む
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0, // 落ちたら落ちたままにする（再試行で緑にしない）
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : [["list"]],
  globalSetup: "./e2e/global-setup.ts", // 1 回ログインして storageState を作る（ST28 / D10）
  use: {
    storageState: "./e2e/.auth/state.json",
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "bash ../tools/stack.sh up",
    url: `http://127.0.0.1:${PORT}/`,
    // 手元で run.sh を立てたままでも走らせられる。CI とハーネスの検証（HARNESS_VERIFY）は毎回 stack.sh から
    // 立てる —— 立っているものを使うと、DB を作り直さずに溜まった状態で測ってしまう
    reuseExistingServer: !process.env.CI && !process.env.HARNESS_VERIFY,
    // 書き出しの進み具合の Scenario が見えるよう、表ごとに 400 ms 待たせる（e2e だけ。確認バッチの run.sh には渡さない。ST33 R19）
    env: { ASHIATO_EXPORT_SELFTEST_DELAY_MS: "400" },
    timeout: 10 * 60 * 1000, // 初回は cargo build --release が入る
    stdout: "pipe",
    stderr: "pipe",
  },
});
