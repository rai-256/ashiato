#!/usr/bin/env bash
# 未捕捉の異常がログに出ることを、**わざと落として**確かめる（製造準備 C）。
set -euo pipefail
cd "$(dirname "$0")/.."
. tools/ports.sh     # worktree ごとのポート（Story を並行して走らせても取り合わない）
export DATABASE_URL="${DATABASE_URL:-postgres://ashiato:ashiato@127.0.0.1:${ASHIATO_DB_PORT}/ashiato}"
export BIND="${BIND:-127.0.0.1:$((ASHIATO_HTTP_PORT + 1))}"
export API_TOKEN="${API_TOKEN:-panic-token-0123456789abcdef}"
export ASHIATO_SELFTEST_PANIC=1
LOG=$(mktemp)
# SRV が無いときに kill 0 にしない（プロセスグループ全体を止める。tools/smoke.sh と同じ）
cleanup(){ if [ -n "${SRV:-}" ]; then kill "$SRV" 2>/dev/null || true; fi; docker compose down -v >/dev/null 2>&1 || true; }
trap cleanup EXIT
docker compose up -d --wait db >/dev/null
cargo run -q -p ashiato-server --bin ashiato-server > "$LOG" 2>&1 & SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$BIND/healthz" >/dev/null 2>&1 && break; sleep 1; done
curl -s -o /dev/null "http://$BIND/selftest/panic" || true
sleep 2
if grep -q 'kind="panic"' "$LOG" || grep -q '未捕捉の異常' "$LOG"; then
  echo "未捕捉の異常がログに出た（確認済み）"
else
  echo "NG: わざと落としたのにログに出なかった"; sed -n '1,20p' "$LOG"; exit 1
fi
