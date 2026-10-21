#!/usr/bin/env bash
# 定期取得の枠が壁時計を読まないことを見る（ST13 / design D5）。
# 試験の中で壁時計を動かしても、枠が読まない限り回数は変わらない（spec-review R11）。
# だから「壁時計が戻っても / 進んでも回数は変わらない」は、試験ではなくこの台本が担保する。
# 枠は `tokio::time`（戻らない時計）だけで測る。DB は要らない。
set -euo pipefail
cd "$(dirname "$0")/.."

FILE=${SCHEDULED_FILE:-crates/server/src/scheduled.rs}
[ -f "$FILE" ] || { echo "  NG $FILE が無い"; exit 1; }

# 壁時計を読む呼び出し。コメントの行は数えない
hits=$(grep -nE 'Utc::now|SystemTime|chrono::Local|Local::now' "$FILE" | grep -vE '^[0-9]+:[[:space:]]*//' || true)

# Scenario: 壁時計が戻っても取りに行く回数は変わらない
echo "壁時計が戻っても取りに行く回数は変わらない: 枠は壁時計を読まない（読む呼び出しが 0 件）"
# Scenario: 壁時計が進んでも取りに行く回数は変わらない
echo "壁時計が進んでも取りに行く回数は変わらない: 枠は壁時計を読まない（読む呼び出しが 0 件）"

if [ -n "$hits" ]; then
  echo "$hits" | sed 's/^/  NG /'
  echo "定期取得の枠が壁時計を読んでいる: $FILE"
  exit 1
fi
echo "定期取得の枠は壁時計を読まない OK"
