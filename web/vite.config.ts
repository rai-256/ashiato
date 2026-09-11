import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const proxy = { "/api": { target: "http://127.0.0.1:18787", rewrite: (p: string) => p.replace(/^\/api/, "") } };

export default defineConfig({
  plugins: [react()],
  server: { proxy },
  // preview（build 済みを配る側）は server.proxy を継がない。確認バッチの run.sh が build 済みの画面を
  // vite preview で出すので、同じ proxy を明示する（実測 2026-09-12: 無いと /api が 404）。
  preview: { proxy },
});
