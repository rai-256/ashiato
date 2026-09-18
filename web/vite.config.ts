import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 画面は合言葉を持たない（ブラウザに置かない）。**API の合言葉は proxy が付ける**（PERM-10）。
// 起動側（tools/dev.sh / dist/verify-<tag>/run.sh）が API_TOKEN を環境に持っているので、それを読む。
// 無ければ header を付けず、サーバの 401 がそのまま画面に出る（黙って通さない）。
// 実測 2026-09-14（確認バッチ 20260913-2255）: 付けていなかったので、画面は稼働状況も達成も 401 で読めなかった。
// 画面がどう資格情報を持つかは Story に無い（ST28 は網の話）。ここは確認用の最小で、本決めは deep へ。
const token = process.env.API_TOKEN;
// **行き先も起動側から読む**（2026-09-18）。`BIND` を変えられるのにここが 18787 固定だったので、
// 別の番号で立てた縦串の画面が**隣で動いている別のサーバ**を読んでいた（e2e が偶然緑になっていた）。
// 既定は run.sh / tools/stack.sh と同じ。
const target = `http://${process.env.BIND ?? "127.0.0.1:18787"}`;
const proxy = {
  "/api": {
    target,
    rewrite: (p: string) => p.replace(/^\/api/, ""),
    ...(token ? { headers: { authorization: `Bearer ${token}` } } : {}),
  },
};

// Tailscale 経由（yoshi.tail4360f4.ts.net）で PC / スマホから開くため。先頭 . でサブドメイン全体を許可する。
// proxy と同じく server / preview の双方に要る。無いと Vite が 403 Blocked request を返す
// （実測 2026-09-14: tailscale serve 越しに :5180 / :5199 が 403）。
const allowedHosts = [".tail4360f4.ts.net"];

export default defineConfig({
  plugins: [react()],
  server: { proxy, allowedHosts },
  // preview（build 済みを配る側）は server.proxy を継がない。確認バッチの run.sh が build 済みの画面を
  // vite preview で出すので、同じ proxy を明示する（実測 2026-09-12: 無いと /api が 404）。
  preview: { proxy, allowedHosts },
});
