#!/usr/bin/env bash
# 定期取得の枠が壁時計を読まないことを見る（ST13 / design D5）。
# 試験の中で壁時計を動かしても、枠が読まない限り回数は変わらない（spec-review R11）。
# だから「壁時計が戻っても / 進んでも回数は変わらない」は、試験ではなくこの台本が担保する。
# 枠は `tokio::time`（戻らない時計）だけで測る。DB は要らない。
#
# **呼び出しの名前でなく入口で止める**（code_verify R18）。`Utc::now` だけを数えていたときは、
# `UNIX_EPOCH.elapsed()`・別名にした `Utc`・`time` の crate・SQL の `now()` を通した。
# 壁時計を持つ crate（chrono / time）と std の壁時計（SystemTime / UNIX_EPOCH）はファイルに名前が出ること自体を断り、
# SQL の壁時計の関数も数える。
set -euo pipefail
cd "$(dirname "$0")/.."

FILE=${SCHEDULED_FILE:-crates/server/src/scheduled.rs}
[ -f "$FILE" ] || { echo "  NG $FILE が無い"; exit 1; }

# 壁時計の入口。コメントの行は数えない
pattern='\bchrono\b|\btime::(OffsetDateTime|UtcDateTime|PrimitiveDateTime)|\bOffsetDateTime\b|now_utc|now_local|SystemTime|UNIX_EPOCH|\bLocal::now|\bUtc\b'
sql='\b(now|clock_timestamp|statement_timestamp|transaction_timestamp|timeofday)[[:space:]]*\(|current_(timestamp|date|time)\b|\blocaltime(stamp)?\b'
hits=$(grep -nE "$pattern" "$FILE" | grep -vE '^[0-9]+:[[:space:]]*//' || true)
sql_hits=$(grep -niE "$sql" "$FILE" | grep -vE '^[0-9]+:[[:space:]]*//' || true)

all=$(printf '%s\n%s\n' "$hits" "$sql_hits" | sed '/^$/d' | sort -u)
if [ -n "$all" ]; then
  echo "$all" | sed 's/^/  NG /'
  echo "定期取得の枠が壁時計を読んでいる（か、読む入口を持っている）: $FILE"
  exit 1
fi

# 判定を通った後にだけ Scenario の行を出す（落ちるときに担保したと読めないように）
# Scenario: 壁時計が戻っても取りに行く回数は変わらない
echo "壁時計が戻っても取りに行く回数は変わらない: 枠は壁時計を読まない（読む入口が 0 件）"
# Scenario: 壁時計が進んでも取りに行く回数は変わらない
echo "壁時計が進んでも取りに行く回数は変わらない: 枠は壁時計を読まない（読む入口が 0 件）"
echo "定期取得の枠は壁時計を読まない OK"
