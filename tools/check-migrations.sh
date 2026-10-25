#!/usr/bin/env bash
# マイグレーションの安全性（製造準備 C）。
# **本体を配布すると、自分が見たことのない他人のデータに当たる。**
# 破壊的変更が 1 つの版に単独で入ると、当てた瞬間に他人の記録が戻らなくなる。
set -euo pipefail
self="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
cd "$(dirname "$0")/.."
# 自己検査（--self-test）は使い捨ての migrations/ にこの検査を向けるため、--dir を内部で受ける
dir=migrations
case "${1:-}" in
  --dir) dir="$2";;
  --self-test) mode=self-test;;
esac

# これから足す版の規則（ST48 / design D10）。名前の時刻（頭の 12 桁）がこの境以上の版にだけ課す。
# 走っている ST23・ST41・ST29 の版（どれも境より前）に、後から規則を課さないため。
NEW_RULES_FROM=202610250000

# 前進側が文として持つ取引の文を、1 行に 1 つ出す。コメント・文字列・`$$` / `$tag$` の本体の中は数えない
# （`DO $$ BEGIN … END $$;` の BEGIN / END は関数の本体で、取引ではない）。
txn_statements() {
  awk '
    { src = src $0 "\n" }
    END {
      n = length(src); out = ""; i = 1
      while (i <= n) {
        c = substr(src, i, 1); two = substr(src, i, 2)
        if (two == "--") { while (i <= n && substr(src, i, 1) != "\n") i++; continue }
        if (two == "/*") { j = index(substr(src, i + 2), "*/"); i = (j == 0) ? n + 1 : i + 2 + j + 1; continue }
        if (c == "\047") { i++; while (i <= n) { if (substr(src, i, 1) == "\047") { if (substr(src, i + 1, 1) == "\047") { i += 2; continue }; break }; i++ }; i++; out = out " "; continue }
        if (c == "$") {
          rest = substr(src, i)
          if (match(rest, /^\$[A-Za-z_]*\$/)) {
            tag = substr(rest, 1, RLENGTH); j = index(substr(rest, RLENGTH + 1), tag)
            i = (j == 0) ? n + 1 : i + RLENGTH + j - 1 + length(tag); out = out " "; continue
          }
        }
        out = out c; i++
      }
      m = split(out, stmts, ";")
      for (k = 1; k <= m; k++) {
        t = toupper(stmts[k]); gsub(/^[ \t\r\n]+/, "", t)
        if (t ~ /^(BEGIN|COMMIT|ROLLBACK|END|START[ \t\r\n]+TRANSACTION)([ \t\r\n]|$)/) { gsub(/[ \t\r\n]+/, " ", t); print t }
      }
    }' "$1"
}

# 1 つの版に、これから足す版の規則を当てる。落ちれば 1 を返す
check_new_rules() {
  local f="$1" d="$2" name stamp txn
  name="$(basename "$f" .sql)"
  stamp="${name%%_*}"
  case "$stamp" in *[!0-9]*|"") return 0;; esac
  [ "$stamp" -ge "$NEW_RULES_FROM" ] || return 0
  local rc=0
  txn="$(txn_statements "$f" | head -n1)"
  if [ -n "$txn" ]; then
    echo "  NG $f: 前進側が自分で取引の文（${txn}）を持つ。取引は適用する側（migrate）が持つ"
    rc=1
  fi
  if [ -f "$d" ] && ! grep -Eiq "DELETE[[:space:]]+FROM[[:space:]]+migration_ledger\.applied[[:space:]]+WHERE[[:space:]]+name[[:space:]]*=[[:space:]]*'${name}'" "$d"; then
    echo "  NG $d: 戻し手順が版の記録の行を消さない（DELETE FROM migration_ledger.applied WHERE name = '${name}'）"
    rc=1
  fi
  return "$rc"
}

if [ "${mode:-}" = self-test ]; then
  work="$(mktemp -d "${TMPDIR:-/tmp}/check-migrations-selftest.XXXXXX")"
  trap 'rm -rf "$work"' EXIT
  new=202610250001_selftest
  mk() { # mk <ケース> <前進側> <戻し側>
    mkdir -p "$work/$1/migrations"
    printf '%s\n' "$2" > "$work/$1/migrations/${new}.sql"
    printf '%s\n' "$3" > "$work/$1/migrations/${new}.down.sql"
  }
  ledger_del="DELETE FROM migration_ledger.applied WHERE name = '${new}';"
  mk txn $'CREATE TABLE t (id int);\nCOMMIT;' "$ledger_del"
  mk noledger 'CREATE TABLE t (id int);' 'DROP TABLE t;'
  mk dofn $'DO $$\nBEGIN\n  PERFORM 1;\nEND\n$$;' "$ledger_del"
  mk good 'CREATE TABLE t (id int);' "$ledger_del"
  fail=0
  expect() { # expect <ケース> <fail|pass> <出力に含める名前>
    local out rc=0
    out="$("$self" --dir "$work/$1/migrations" 2>&1)" || rc=$?
    if [ "$2" = fail ]; then
      if [ "$rc" -ne 0 ] && grep -q "$new" <<<"$out"; then return 0; fi
      echo "  self-test NG: $1 が落ちない、または版の名前が出ない (rc=$rc)"; echo "$out"; return 1
    fi
    [ "$rc" -eq 0 ] || { echo "  self-test NG: $1 が通らない (rc=$rc)"; echo "$out"; return 1; }
  }
  expect txn fail && echo "Scenario: 新しい版が自分で取引を持つと、検査が落ちる" || fail=1
  expect noledger fail && echo "Scenario: 新しい版の戻し手順が版の記録の行を消さないと、検査が落ちる" || fail=1
  expect dofn pass || fail=1
  expect good pass || fail=1
  [ "$fail" -eq 0 ] && echo "self-test OK" || { echo "self-test NG"; exit 1; }
  exit 0
fi

echo "Scenario: 前からある版には、この規則を求めない"
# **列の削除だけが破壊的なのではない。** 型の変換とビューの作り直しも、
# 当てた瞬間に元へ戻せなくなる（0003 は `jsonb` → `text` で、戻すと原文が失われる）。
# 実測: この検査は 0003 を素通りさせていた（review R5 / R14）。
DESTRUCTIVE='(DROP[[:space:]]+(TABLE|COLUMN|SCHEMA|VIEW)|ALTER[[:space:]]+TABLE[^;]*DROP|ALTER[[:space:]]+COLUMN[^;]*TYPE)'
# 逃げ道: down.sql に「不可逆」と明記された版は、人間が承知のうえと見なして通す。
# **前進側の .sql ではなく戻し側に書かせる** —— 戻せないことは戻し手順の性質だから。
ACKNOWLEDGED='不可逆'

bad=0
for f in "$dir"/*.sql; do
  case "$f" in *.down.sql) continue;; esac
  d="${f%.sql}.down.sql"
  # 名前は作成時刻（YYYYMMDDHHMM_<slug>.sql）。連番にしない ——
  # 並走する Story が番号を取り合う（実測 2026-09-11: ST02 が 0007 を取り、ST03 が 0008 へずらす PR を出した）。
  # 適用の順は migrations/ORDER が持つ（末尾に 1 行足す。merge=union で並走する Story の追記は衝突しない）ので、
  # 名前は衝突しないことだけが要る。ORDER に無い版・ファイルの無い版は crates/server/build.rs がビルドで落とす。
  case "$(basename "$f")" in
    [0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]_*.sql) ;;
    *) echo "  NG $f: 名前が YYYYMMDDHHMM_<slug>.sql でない（連番は並走する Story が取り合う）"; bad=1;;
  esac
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
  check_new_rules "$f" "$d" || bad=1
done
[ "$bad" -eq 0 ] && echo "マイグレーション OK" || { echo "マイグレーション NG"; exit 1; }
