#!/usr/bin/env bash
# 縦串を 1 コマンドで立てる。**人間が見るものと、e2e が見るものを同じ起動にする。**
#
#   ./tools/stack.sh up        DB → サーバ → 偽データ → 画面。前景で待つ（Ctrl-C で全部止まる）
#   ./tools/stack.sh up --check-only   DB → 役割 → 移行 → サーバの /healthz まで通して止める（偽データ・画面は立てない）
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
check_only=0; [ "${2:-}" = "--check-only" ] && check_only=1

if [ -f .env ]; then set -a; . ./.env; set +a; fi
export DATABASE_URL="${DATABASE_URL:?.env を読み込む（set -a; . ./.env; set +a）か DATABASE_URL を渡す}"
export DATABASE_OWNER_URL="${DATABASE_OWNER_URL:?.env を読み込むか DATABASE_OWNER_URL を渡す（移行は所有者の接続で当てる）}"
export BIND="${BIND:-127.0.0.1:18787}"
export API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
export WEB_PASSWORD="${WEB_PASSWORD:?.env を読み込む（set -a; . ./.env; set +a）か WEB_PASSWORD を渡す}"
export WEB_PORT="${WEB_PORT:-5180}"     # 開発用 vite（5173）と衝突しない番号。--strictPort で黙って逃げない

if [ "$cmd" = "down" ]; then
  docker compose down >/dev/null 2>&1 || true
  echo "== DB を止めた"
  exit 0
fi
[ "$cmd" = "up" ] || { sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2; }

# port が使用中なら 30 秒待たずにここで止める（実測: 5173 を別プロジェクトの vite が使っていて、
# preview が隣の番号に逃げ、確認者は別の画面を見ていた）。
# 同じ host:port か、全インタフェース（0.0.0.0 / [::] / *）で塞がれているときだけ「使用中」。
# 番号だけで見ない —— 本番のサーバが Tailscale の IP:18787 で動いている横で 127.0.0.1:18787 は使える（実測）
busy() { local l port="${1##*:}"; l="$(ss -ltn 2>/dev/null | awk '{print $4}')"
  printf '%s\n' "$l" | grep -qxF "$1" || printf '%s\n' "$l" | grep -qE "^(0\.0\.0\.0|\[::\]|\*):$port$"; }
busy "$BIND" && { echo "error: $BIND は使用中（$(ss -ltnp 2>/dev/null | grep -F "$BIND " | grep -oE 'users:\(.*' | head -1)）。BIND を変えるか、そのサーバを止める"; exit 1; }
if [ "$check_only" = 0 ] && busy "127.0.0.1:$WEB_PORT"; then
  echo "error: port $WEB_PORT は使用中。WEB_PORT=<別の番号> で叩き直す"; exit 1
fi

# 依存の導入を前段に含める（製造準備 B）。渡されていて実在するならビルドしない
server="${SERVER_BIN:-target/release/ashiato-server}"
if [ ! -x "$server" ]; then
  echo "== サーバを build（$server が無い）"
  cargo build -q --release -p ashiato-server --bin ashiato-server
  server="target/release/ashiato-server"
fi
web="${WEB_DIST:-web/dist}"
if [ "$check_only" = 0 ] && [ ! -f "$web/index.html" ]; then
  echo "== 画面を build（$web が無い）"
  (cd web && npm ci --silent >/dev/null && npm run build --silent >/dev/null)
  web="web/dist"
fi
[ "$check_only" = 1 ] || web_abs="$(cd "$web" && pwd)"

if [ "${STACK_RESET:-}" = "1" ]; then
  echo "== DB を作り直す（STACK_RESET=1）"; docker compose down -v >/dev/null 2>&1 || true
fi
echo "== DB"; docker compose up -d --wait db >/dev/null
# 役割 → 移行 → サーバの順（volume を消すと役割も消える。design D5 / review R17）
./tools/db-roles.sh
echo "== 移行（所有者の接続。サーバは移行を当てない）"; "$server" migrate
trap 'kill 0' EXIT
# サーバの環境から管理者と所有者の秘密を外す（.env の全部を export しているので。design D5）
echo "== サーバ $BIND"
env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL "$server" &
srv=$!
for _ in $(seq 1 30); do curl -sf "http://$BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$BIND/healthz" >/dev/null || { echo "サーバが起動しない（BIND=$BIND）"; exit 1; }
# 起動した口が網の外に開いていないこと（design D11 (a)）。落ちたら止める（trap が全部止める）
if [ "$check_only" = 1 ]; then
  ./tools/check-exposure.sh --listen-only || exit 1
  # 呼び出し元まで巻き込まないよう、サーバだけを止める
  trap - EXIT; kill "$srv"; wait "$srv" 2>/dev/null || true
  echo "== 作り直し → 役割 → 移行 → サーバの /healthz まで通った"
  exit 0
fi
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
sleep 2   # vite preview が待ち受けるのを待つ
./tools/check-exposure.sh --listen-only || exit 1
echo
echo "画面: http://127.0.0.1:$WEB_PORT    API: http://$BIND    （端末から届く手順は docs/network.md を見る。BIND は loopback のまま）"
wait
