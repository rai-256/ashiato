// SPDX-License-Identifier: AGPL-3.0-only
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    // 画面の検査なので DOM が要る。**実寸のレイアウトは計算されない** ——
    // 24 px や「1 画面に収まる」は指定と勘定で固定し、目視は人間の確認待ちに残す。
    environment: "jsdom",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    setupFiles: ["src/__tests__/setup.ts"],
  },
});
