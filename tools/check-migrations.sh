#!/usr/bin/env bash
# マイグレーションの安全性（製造準備 C）。
# **本体を配布すると、自分が見たことのない他人のデータに当たる。**
# 破壊的変更が 1 つの版に単独で入ると、当てた瞬間に他人の記録が戻らなくなる。
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
for f in migrations/*.sql; do
  case "$f" in *.down.sql) continue;; esac
  if grep -Eiq '(DROP[[:space:]]+(TABLE|COLUMN|SCHEMA)|ALTER[[:space:]]+TABLE[^;]*DROP)' "$f"; then
    echo "  NG $f: 前進側に破壊的変更がある（列や表の削除は別の版に分ける）"
    bad=1
  fi
  d="${f%.sql}.down.sql"
  [ -f "$d" ] || { echo "  NG $f: 戻し手順 $d が無い"; bad=1; }
done
[ "$bad" -eq 0 ] && echo "マイグレーション OK" || { echo "マイグレーション NG"; exit 1; }
