#!/usr/bin/env bash
# 開発版の印 nightly に載った移行の凍結を検査する（ST54 / design D5）。
# nightly 以後に足した移行と戻し手順の変更は対象外。名前や前進側を直すときは
# migrations/LINEAGE に旧名・直しの移行との対応を残す。
set -euo pipefail
cd "$(dirname "$0")/.."

order_names() { grep -Ev '^[[:space:]]*(#|$)'; }

lineage_renamed_to() {
  local old="$1" line kind from to extra
  [ -f migrations/LINEAGE ] || return 1
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    read -r kind from to extra <<< "$line"
    if [ "$kind" = renamed ] && [ "$from" = "$old" ] && [ -z "${extra:-}" ]; then
      printf '%s\n' "$to"
      return 0
    fi
  done < migrations/LINEAGE
  return 1
}

lineage_has_removed() {
  local old="$1" line kind from to extra
  [ -f migrations/LINEAGE ] || return 1
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    read -r kind from to extra <<< "$line"
    if [ "$kind" = removed ] && [ "$from" = "$old" ] && [ -z "${to:-}" ]; then
      return 0
    fi
  done < migrations/LINEAGE
  return 1
}

# rewritten の 1 列目は今の ORDER の名前（tools/check-migrations.sh と同じ読み方）。名前を変えた移行は、変えた後の名前で引く
lineage_has_rewritten() {
  local current="$1" line kind from to extra
  [ -f migrations/LINEAGE ] || return 1
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    read -r kind from to extra <<< "$line"
    if [ "$kind" = rewritten ] && [ "$from" = "$current" ] && [ -n "${to:-}" ] && [ -z "${extra:-}" ]; then
      return 0
    fi
  done < migrations/LINEAGE
  return 1
}

check_tree() {
  local shallow nightly_order cur_order name current_name renamed bad=0
  if ! shallow="$(git rev-parse --is-shallow-repository)"; then
    echo "履歴が浅いかを確かめられない"
    return 1
  fi
  if [ "$shallow" = true ]; then
    echo "履歴が浅いので nightly を数えられない"
    return 1
  fi
  if ! git rev-parse --verify refs/tags/nightly^{commit} >/dev/null 2>&1; then
    if git show-ref --tags --verify --quiet refs/tags/nightly; then
      echo "NG 印 nightly の commit を読み取れない"
      return 1
    fi
    echo "印 nightly が無い: 検査する対象が無い"
    return 0
  fi
  if ! nightly_order="$(git show nightly:migrations/ORDER 2>/dev/null)"; then
    echo "NG 印 nightly の migrations/ORDER を取り出せない"
    return 1
  fi
  cur_order="$(order_names < migrations/ORDER)"
  local compare_dir
  compare_dir="$(mktemp -d "${TMPDIR:-/tmp}/nightly-migrations.XXXXXX")"
  trap 'rm -rf "$compare_dir"' RETURN
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    current_name="$name"
    if ! printf '%s\n' "$cur_order" | grep -qxF -- "$current_name"; then
      if renamed="$(lineage_renamed_to "$name")" && printf '%s\n' "$cur_order" | grep -qxF -- "$renamed"; then
        current_name="$renamed"
      elif lineage_has_removed "$name"; then
        continue
      else
        echo "NG 移行 $name は nightly の版にあるが、今の ORDER に無い（removed / renamed の対応が無い）"
        bad=1
        continue
      fi
    fi
    if ! git show "nightly:migrations/$name.sql" > "$compare_dir/nightly.sql" 2>/dev/null; then
      echo "NG 印 nightly の移行 $name.sql を取り出せない"
      bad=1
      continue
    fi
    if ! cmp -s "$compare_dir/nightly.sql" "migrations/$current_name.sql"; then
      if lineage_has_rewritten "$current_name"; then
        continue
      fi
      echo "NG 移行 $current_name の中身が nightly の版から変わっている（直しの移行を足して rewritten を書く）"
      bad=1
    fi
  done < <(printf '%s\n' "$nightly_order" | order_names)
  [ "$bad" -eq 0 ] && echo "開発版に載った移行の検査 OK" || echo "開発版に載った移行の検査 NG"
  return "$bad"
}

if [ "${1:-}" != --self-test ]; then
  check_tree
  exit $?
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/nightly-migrations-selftest.XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0 n=0
new_tree() {
  n=$((n + 1)); T="$work/t$n"
  mkdir -p "$T/migrations"
  git -C "$T" init -q
  git -C "$T" config user.email t@example.com
  git -C "$T" config user.name t
  printf '202001010000_a\n' > "$T/migrations/ORDER"
  printf 'CREATE TABLE a ();\n' > "$T/migrations/202001010000_a.sql"
  printf 'DROP TABLE a;\n' > "$T/migrations/202001010000_a.down.sql"
  : > "$T/migrations/LINEAGE"
  git -C "$T" add -A; git -C "$T" commit -q -m base
  git -C "$T" tag nightly
}
commit_all() { git -C "$T" add -A; git -C "$T" commit -q -m "$1"; }
expect() {
  local want="$1" out rc=0 pattern
  shift
  out="$(cd "$T" && check_tree 2>&1)" || rc=$?
  if [ "$rc" -ne "$want" ]; then
    echo "  FAIL: rc=$rc（期待 $want）"; printf '%s\n' "$out" | sed 's/^/    /'; fail=1
  else
    for pattern in "$@"; do
      if ! grep -Fq -- "$pattern" <<< "$out"; then
        echo "  FAIL: 出力に「$pattern」が無い"; printf '%s\n' "$out" | sed 's/^/    /'; fail=1
      fi
    done
  fi
}

new_tree; mv "$T/migrations/202001010000_a.sql" "$T/migrations/202001010001_b.sql"; mv "$T/migrations/202001010000_a.down.sql" "$T/migrations/202001010001_b.down.sql"; printf '202001010001_b\n' > "$T/migrations/ORDER"; commit_all rename
echo "Scenario: 開発版に載った移行の名前を変えて旧名の対応が無ければ、検査が落ちる"; expect 1 202001010000_a

new_tree; mv "$T/migrations/202001010000_a.sql" "$T/migrations/202001010001_b.sql"; mv "$T/migrations/202001010000_a.down.sql" "$T/migrations/202001010001_b.down.sql"; printf '202001010001_b\n' > "$T/migrations/ORDER"; printf 'renamed 202001010000_a 202001010001_b\n' > "$T/migrations/LINEAGE"; commit_all rename-with-lineage
echo "Scenario: 開発版に載った移行の名前を変えても、旧名の対応があれば通る"; expect 0

new_tree; mv "$T/migrations/202001010000_a.sql" "$T/migrations/202001010001_b.sql"; printf 'CREATE TABLE changed ();\n' > "$T/migrations/202001010001_b.sql"; mv "$T/migrations/202001010000_a.down.sql" "$T/migrations/202001010001_b.down.sql"; printf '202001010001_b\n' > "$T/migrations/ORDER"; printf 'renamed 202001010000_a 202001010001_b\n' > "$T/migrations/LINEAGE"; commit_all rename-and-rewrite
echo "Scenario: 名前を変えて中身も変えた移行は、旧名の対応だけでは落ちる"; expect 1 202001010001_b '直しの移行を足して rewritten を書く'

new_tree; printf 'CREATE TABLE changed ();\n' > "$T/migrations/202001010000_a.sql"; commit_all rewrite
echo "Scenario: 開発版に載った移行の中身を変えて直しの対応が無ければ、検査が落ちる"; expect 1 202001010000_a '直しの移行を足して rewritten を書く'

new_tree; printf 'CREATE TABLE changed ();\n' > "$T/migrations/202001010000_a.sql"; printf '202001010000_a\n202001010001_c\n' > "$T/migrations/ORDER"; printf 'CREATE TABLE c ();\n' > "$T/migrations/202001010001_c.sql"; printf 'DROP TABLE c;\n' > "$T/migrations/202001010001_c.down.sql"; printf 'rewritten 202001010000_a 202001010001_c\n' > "$T/migrations/LINEAGE"; commit_all rewrite-with-fix
echo "Scenario: 開発版に載った移行の中身を変えても、直しの移行の対応があれば通る"; expect 0

n=$((n + 1)); T="$work/t$n"; mkdir -p "$T/migrations"; git -C "$T" init -q; git -C "$T" config user.email t@example.com; git -C "$T" config user.name t; printf '202001010000_a\n' > "$T/migrations/ORDER"; : > "$T/migrations/202001010000_a.sql"; git -C "$T" add -A; git -C "$T" commit -q -m no-nightly
echo "Scenario: 印 nightly が無ければ、開発版に載った移行の検査は通る"; expect 0

new_tree; printf '202001010000_a\n202001010001_n\n' > "$T/migrations/ORDER"; printf 'CREATE TABLE n ();\n' > "$T/migrations/202001010001_n.sql"; printf 'DROP TABLE n;\n' > "$T/migrations/202001010001_n.down.sql"; commit_all add; printf 'changed\n' > "$T/migrations/202001010001_n.sql"; commit_all change-new
echo "Scenario: 印 nightly の後に足した移行は、変えても通る"; expect 0

new_tree; printf 'changed down\n' > "$T/migrations/202001010000_a.down.sql"; commit_all down
echo "Scenario: 開発版に載った移行の戻し手順は、変えても通る"; expect 0

new_tree; git -C "$T" rm -q migrations/ORDER; git -C "$T" commit -q -m nightly-without-order; git -C "$T" tag -f nightly >/dev/null; printf '202001010000_a\n' > "$T/migrations/ORDER"; commit_all current-order
echo "Scenario: 印 nightly の版の移行を取り出せなければ、検査は通らない"; expect 1 '印 nightly の migrations/ORDER を取り出せない'

# 名前を変えて中身も変えた移行は、renamed と、今の名前の rewritten で通る（tools/check-migrations.sh の LINEAGE の検査も同じ木で通る。final review R2）
new_tree; mv "$T/migrations/202001010000_a.sql" "$T/migrations/202001010001_b.sql"; printf 'CREATE TABLE changed ();\n' > "$T/migrations/202001010001_b.sql"; mv "$T/migrations/202001010000_a.down.sql" "$T/migrations/202001010001_b.down.sql"; printf 'CREATE TABLE c ();\n' > "$T/migrations/202001010002_c.sql"; printf 'DROP TABLE c;\n' > "$T/migrations/202001010002_c.down.sql"; printf '202001010001_b\n202001010002_c\n' > "$T/migrations/ORDER"; printf 'renamed 202001010000_a 202001010001_b\nrewritten 202001010001_b 202001010002_c\n' > "$T/migrations/LINEAGE"; commit_all rename-and-rewrite-with-fix
echo "== 名前を変えて中身も変えた移行は、renamed と今の名前の rewritten で通る"; expect 0
if ! out="$("$PWD/tools/check-migrations.sh" --dir "$T/migrations" 2>&1)"; then echo "  FAIL: 同じ LINEAGE を check-migrations.sh が通さない"; printf '%s\n' "$out" | sed 's/^/    /'; fail=1; fi

# removed の行は、行末のコメントや空白があっても読む（final review R19）
new_tree; printf '202001010001_n\n' > "$T/migrations/ORDER"; printf 'CREATE TABLE n ();\n' > "$T/migrations/202001010001_n.sql"; printf 'DROP TABLE n;\n' > "$T/migrations/202001010001_n.down.sql"; git -C "$T" rm -q migrations/202001010000_a.sql migrations/202001010000_a.down.sql; printf 'removed 202001010000_a\n' > "$T/migrations/LINEAGE"; commit_all removed
echo "== 開発版に載った移行を外しても、removed の対応があれば通る"; expect 0
printf '  removed   202001010000_a   # 使わなくなった\n' > "$T/migrations/LINEAGE"; commit_all removed-with-comment
echo "== removed の行に行末のコメントと空白があっても通る"; expect 0

new_tree; git -C "$T" commit --allow-empty -q -m next; git clone -q --depth=1 "file://$T" "$work/shallow"; mkdir -p "$work/shallow/tools"; cp "$PWD/tools/check-nightly-migrations.sh" "$work/shallow/tools/"; echo "Scenario: 浅い木では、開発版に載った移行の検査は通らない"; if out=$(cd "$work/shallow" && tools/check-nightly-migrations.sh 2>&1); then echo "  FAIL: 浅い木で検査が通った"; fail=1; elif ! grep -q '履歴が浅い' <<< "$out"; then echo "  FAIL: 浅い木の出力に「履歴が浅い」が無い"; fail=1; fi

[ "$fail" -eq 0 ] && echo "check-nightly-migrations self-test OK" || { echo "check-nightly-migrations self-test NG"; exit 1; }
