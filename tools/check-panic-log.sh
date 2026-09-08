#!/usr/bin/env bash
# 未捕捉の異常がログに出ることを、**わざと落として**確かめる（製造準備 C）。
set -euo pipefail
cd "$(dirname "$0")/.."
export DATABASE_URL="${DATABASE_URL:-postgres://ashiato:ashiato@127.0.0.1:55432/ashiato}"
export BIND="${BIND:-127.0.0.1:18788}"
export API_TOKEN="${API_TOKEN:-panic-token-0123456789abcdef}"
export ASHIATO_SELFTEST_PANIC=1
LOG=$(mktemp)
cleanup(){ kill "${SRV:-0}" 2>/dev/null || true; docker compose down -v >/dev/null 2>&1 || true; }
trap cleanup EXIT
docker compose up -d --wait db >/dev/null
cargo run -q -p ashiato-server > "$LOG" 2>&1 & SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$BIND/healthz" >/dev/null 2>&1 && break; sleep 1; done
curl -s -o /dev/null "http://$BIND/selftest/panic" || true
sleep 2
if grep -q 'kind="panic"' "$LOG" || grep -q '未捕捉の異常' "$LOG"; then
  echo "未捕捉の異常がログに出た（確認済み）"
else
  echo "NG: わざと落としたのにログに出なかった"; sed -n '1,20p' "$LOG"; exit 1
fi
