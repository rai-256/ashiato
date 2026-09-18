#!/usr/bin/env bash
# 縦串を 1 コマンドで立てる。**人間が見るものと、e2e が見るものを同じ起動にする。**
#
#   ./tools/stack.sh up        DB → サーバ → 偽データ → 画面。前景で待つ（Ctrl-C で全部止まる）
#   ./tools/stack.sh down      DB を止める（-v は付けない。消すのは STACK_RESET=1 の up）
#
# 環境変数:
#   SERVER_BIN    サーバの実体（既定 target/release/ashiato-server。無ければ build する）
#   WEB_DIST      画面の build 済みディレクトリ（既定 web/dist。無ければ build する）
#   SEED          normal（既定）/ max / empty
#   STACK_RESET=1 起動の前に DB を作り直す（e2e のように毎回同じ状態から始めたいとき）
#   BIND / WEB_PORT / DATABASE_URL / API_TOKEN
#
# 呼び出し元は 2 つある。**分けない** —— 分けると「e2e は緑なのに人間が見る画面は違う」が起きる。
#   - 確認バッチ  dist/verify-<tag>/run.sh（ビルド済みを SERVER_BIN / WEB_DIST で渡す）
#   - e2e         web/playwright.config.ts の webServer
set -euo pipefail
cd "$(dirname "$0")/.."
cmd="${1:-up}"

if [ -f .env ]; then set -a; . ./.env; set +a; fi
export DATABASE_URL="${DATABASE_URL:-postgres://ashiato:ashiato@127.0.0.1:55432/ashiato}"
export BIND="${BIND:-127.0.0.1:18787}"
export API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
export WEB_PORT="${WEB_PORT:-5180}"     # 開発用 vite（5173）と衝突しない番号。--strictPort で黙って逃げない

if [ "$cmd" = "down" ]; then
  docker compose down >/dev/null 2>&1 || true
  echo "== DB を止めた"
  exit 0
fi
[ "$cmd" = "up" ] || { sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2; }

# port が使用中なら 30 秒待たずにここで止める（実測: 5173 を別プロジェクトの vite が使っていて、
# preview が隣の番号に逃げ、確認者は別の画面を見ていた）。
# 同じ host:port か、全インタフェース（0.0.0.0 / [::] / *）で塞がれているときだけ「使用中」。
# 番号だけで見ない —— 本番のサーバが Tailscale の IP:18787 で動いている横で 127.0.0.1:18787 は使える（実測）
busy() { local l port="${1##*:}"; l="$(ss -ltn 2>/dev/null | awk '{print $4}')"
  printf '%s\n' "$l" | grep -qxF "$1" || printf '%s\n' "$l" | grep -qE "^(0\.0\.0\.0|\[::\]|\*):$port$"; }
busy "$BIND" && { echo "error: $BIND は使用中（$(ss -ltnp 2>/dev/null | grep -F "$BIND " | grep -oE 'users:\(.*' | head -1)）。BIND を変えるか、そのサーバを止める"; exit 1; }
busy "127.0.0.1:$WEB_PORT" && { echo "error: port $WEB_PORT は使用中。WEB_PORT=<別の番号> で叩き直す"; exit 1; }

# 依存の導入を前段に含める（製造準備 B）。渡されていて実在するならビルドしない
server="${SERVER_BIN:-target/release/ashiato-server}"
if [ ! -x "$server" ]; then
  echo "== サーバを build（$server が無い）"
  cargo build -q --release -p ashiato-server --bin ashiato-server
  server="target/release/ashiato-server"
fi
web="${WEB_DIST:-web/dist}"
if [ ! -f "$web/index.html" ]; then
  echo "== 画面を build（$web が無い）"
  (cd web && npm ci --silent >/dev/null && npm run build --silent >/dev/null)
  web="web/dist"
fi
web_abs="$(cd "$web" && pwd)"

if [ "${STACK_RESET:-}" = "1" ]; then
  echo "== DB を作り直す（STACK_RESET=1）"; docker compose down -v >/dev/null 2>&1 || true
fi
echo "== DB"; docker compose up -d --wait db >/dev/null
trap 'kill 0' EXIT
echo "== サーバ $BIND"; "$server" &
for _ in $(seq 1 30); do curl -sf "http://$BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$BIND/healthz" >/dev/null || { echo "サーバが起動しない（BIND=$BIND）"; exit 1; }
# 偽データ。**STACK_RESET=1（毎回同じ状態から始める＝e2e）のときは落ちたら止める。**
# 溜まった DB では seed の読み直しが合わないことがあり（実測 2026-09-18: ST19 の主張が
# 「入れた 21 件と読めた件数が合わない」で rc=1）、そこを警告で流すと
# **何が入っているか分からない画面**を人間と e2e の両方が見る。
echo "== 偽データ（${SEED:-normal}）"
if ! ./tools/seed.sh "${SEED:-normal}" > /tmp/ashiato-seed.log 2>&1; then
  if [ "${STACK_RESET:-}" = "1" ]; then
    echo "error: 作り直した DB で seed が落ちた（rc≠0）。偽データが不定のまま先へ進めない:" >&2
    tail -5 /tmp/ashiato-seed.log >&2
    exit 1
  fi
  echo "warn: seed が rc≠0（DB に前の実行が残っている可能性。続ける）:" >&2
  tail -3 /tmp/ashiato-seed.log >&2
fi
echo "== 画面 http://127.0.0.1:$WEB_PORT"
(cd web && npx vite preview --host 127.0.0.1 --port "$WEB_PORT" --strictPort --outDir "$web_abs" >/dev/null 2>&1) &
echo
echo "画面: http://127.0.0.1:$WEB_PORT    API: http://$BIND    （端末から届くには BIND を LAN / Tailscale の IP にする）"
wait
