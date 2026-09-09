#!/usr/bin/env bash
# マイグレーションの安全性（製造準備 C）。
# **本体を配布すると、自分が見たことのない他人のデータに当たる。**
# 破壊的変更が 1 つの版に単独で入ると、当てた瞬間に他人の記録が戻らなくなる。
set -euo pipefail
cd "$(dirname "$0")/.."
# **列の削除だけが破壊的なのではない。** 型の変換とビューの作り直しも、
# 当てた瞬間に元へ戻せなくなる（0003 は `jsonb` → `text` で、戻すと原文が失われる）。
# 実測: この検査は 0003 を素通りさせていた（review R5 / R14）。
DESTRUCTIVE='(DROP[[:space:]]+(TABLE|COLUMN|SCHEMA|VIEW)|ALTER[[:space:]]+TABLE[^;]*DROP|ALTER[[:space:]]+COLUMN[^;]*TYPE)'
# 逃げ道: down.sql に「不可逆」と明記された版は、人間が承知のうえと見なして通す。
# **前進側の .sql ではなく戻し側に書かせる** —— 戻せないことは戻し手順の性質だから。
ACKNOWLEDGED='不可逆'

bad=0
for f in migrations/*.sql; do
  case "$f" in *.down.sql) continue;; esac
  d="${f%.sql}.down.sql"
  if grep -Eiq "$DESTRUCTIVE" "$f"; then
    if [ -f "$d" ] && grep -q "$ACKNOWLEDGED" "$d"; then
      echo "  ok $f: 破壊的だが $d に「$ACKNOWLEDGED」と明記されている"
    else
      echo "  NG $f: 前進側に破壊的変更がある（列・表・ビューの削除、列の型変換）"
      echo "     戻せないなら $d に「$ACKNOWLEDGED」と、何が失われるかを書く"
      bad=1
    fi
  fi
  [ -f "$d" ] || { echo "  NG $f: 戻し手順 $d が無い"; bad=1; }
done
[ "$bad" -eq 0 ] && echo "マイグレーション OK" || { echo "マイグレーション NG"; exit 1; }
