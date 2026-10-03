import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 画面を配る側は合言葉を付け足さない（ST28 / design D10）。cookie はそのまま通る（http-proxy の既定）。
// 画面の読み出しはログインの印（HttpOnly の cookie）で通る。印が無ければサーバの 401 がそのまま画面に届く。
// **行き先も起動側から読む**（2026-09-18）。`BIND` を変えられるのにここが 18787 固定だったので、
// 別の番号で立てた縦串の画面が**隣で動いている別のサーバ**を読んでいた（e2e が偶然緑になっていた）。
// 既定は run.sh / tools/stack.sh と同じ。
const target = `http://${process.env.BIND ?? "127.0.0.1:18787"}`;
const proxy = {
  "/api": {
    target,
    rewrite: (p: string) => p.replace(/^\/api/, ""),
  },
};

// 手元の網（Tailscale など）越しに PC / スマホから開くため。先頭 . でサブドメイン全体を許可する。
// proxy と同じく server / preview の双方に要る。無いと Vite が 403 Blocked request を返す
// （実測 2026-09-14: tailscale serve 越しに :5180 / :5199 が 403）。
// **網の名前はリポジトリに置かない**（公開するので）。`.env` の ALLOWED_HOSTS から読む
// （`tools/stack.sh` が `.env` を export するので preview にも届く）。例: `.tailXXXXXX.ts.net`
const allowedHosts = (process.env.ALLOWED_HOSTS ?? "").split(",").map((h) => h.trim()).filter(Boolean);

// 画面の本体への指示。`vite dev` は HMR が inline script を使うので付けない（開発用の画面は網へ出さない）。
const previewHeaders = {
  "Content-Security-Policy": "default-src 'self'; frame-ancestors 'none'",
  "Cache-Control": "no-store",
};

export default defineConfig({
  plugins: [react()],
  server: { proxy, allowedHosts },
  // preview（build 済みを配る側）は server.proxy を継がない。確認バッチの run.sh が build 済みの画面を
  // vite preview で出すので、同じ proxy を明示する（実測 2026-09-12: 無いと /api が 404）。
  preview: { proxy, allowedHosts, headers: previewHeaders },
});
