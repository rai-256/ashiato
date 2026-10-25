// SPDX-License-Identifier: AGPL-3.0-only
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  // 画面の文言を Rust の定数と突き合わせるテストが `crates/` の字面を `?raw` で読む（satellites.test.tsx）。
  server: { fs: { allow: ["..", "."] } },
  test: {
    // 画面の検査なので DOM が要る。**実寸のレイアウトは計算されない** ——
    // 24 px や「1 画面に収まる」は指定と勘定で固定し、目視は人間の確認待ちに残す。
    environment: "jsdom",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    setupFiles: ["src/__tests__/setup.ts"],
  },
});
