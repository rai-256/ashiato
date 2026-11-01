#!/usr/bin/env bash
# 配った移行の凍結の検査（ST48 / design D9）。
# 配った印（タグ v*）の版にある移行は、名前も前進側の中身も変えない・消さない。
# 配った版を持つ他人の DB は、その移行を当て済みで、後から変えても当て直されない。
# 直したければ直さずに新しい版を足す。戻し手順（.down.sql）は見ない。
# 印が 0 個なら通す。浅い木では印を数えずに落とす。
# 使い方: check-released-migrations.sh [--self-test]
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

# ORDER の版の名前（# 行と空行は読まない）
order_names() { grep -Ev '^[[:space:]]*(#|$)'; }

# 今いる git の木を検査する。0: 通る / 1: 落ちる
check_tree() {
  local tags all_tags tag name bad=0
  if [ "$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)" = true ]; then
    echo "履歴が浅いので配った印を数えられない"
    return 1
  fi
  all_tags="$(git tag --list 'v[0-9]*')"
  for tag in $all_tags; do
    if ! git rev-parse --verify "$tag^{commit}" >/dev/null 2>&1; then
      echo "  NG 印 $tag の commit を読み取れない（タグの取り寄せが足りない可能性）"
      return 1
    fi
  done
  tags="$(git tag --merged HEAD --list 'v[0-9]*')"
  if [ -z "$tags" ]; then
    echo "配った印が 0 個（タグ v* が無い）: 検査する対象が無い"
    return 0
  fi
  local cur_order
  cur_order="$(order_names < migrations/ORDER)"
  for tag in $tags; do
    local old_order
    if ! old_order="$(git show "$tag:migrations/ORDER" 2>/dev/null)"; then
      echo "  NG 印 $tag の migrations/ORDER を取り出せない（タグの取り寄せが足りない可能性）"
      bad=1
      continue
    fi
    while IFS= read -r name; do
      [ -n "$name" ] || continue
      if ! printf '%s\n' "$cur_order" | grep -qxF -- "$name"; then
        echo "  NG 移行 $name は印 $tag の版にあるが、今の ORDER に無い（消した・名前を変えた）"
        bad=1
      elif ! git diff --quiet "$tag" HEAD -- "migrations/$name.sql"; then
        echo "  NG 移行 $name の中身が印 $tag の版から変わっている（直さずに新しい版を足す）"
        bad=1
      fi
    done < <(printf '%s\n' "$old_order" | order_names)
  done
  [ "$bad" -eq 0 ] && echo "配った移行の凍結 OK" || echo "配った移行の凍結 NG"
  return "$bad"
}

if [ "${1:-}" != "--self-test" ]; then
  check_tree
  exit $?
fi

# ---- --self-test: 使い捨ての git の木で 7 つの場合を走らせる ----
work="$(mktemp -d "${TMPDIR:-/tmp}/released-migrations.XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0
n=0

# 木を作る: 版 a と b を持つ最初の commit。印は付けない
new_tree() {
  n=$((n + 1))
  T="$work/t$n"
  mkdir -p "$T/migrations"
  git -C "$T" init -q
  git -C "$T" config user.email t@example.com
  git -C "$T" config user.name t
  git -C "$T" config commit.gpgsign false
  printf '# c\n202001010000_a\n\n202001010001_b\n' >"$T/migrations/ORDER"
  for v in 202001010000_a 202001010001_b; do
    echo "CREATE TABLE $v();" >"$T/migrations/$v.sql"
    echo "DROP TABLE $v;" >"$T/migrations/$v.down.sql"
  done
  git -C "$T" add -A
  git -C "$T" commit -q -m base
}
commit_all() { git -C "$T" add -A && git -C "$T" commit -q -m "$1"; }
tag_v1() { git -C "$T" tag v1.0.0; }
# 期待: $1=0 通る / 1 落ちる（印の echo は呼ぶ側に literal で置く。check_scenarios.py が literal を拾う）
expect() {
  local want="$1" rc=0 out
  out="$(cd "$T" && check_tree 2>&1)" || rc=$?
  if [ "$rc" -ne "$want" ]; then
    echo "  FAIL: rc=$rc（期待 $want）"; printf '%s\n' "$out" | sed 's/^/    /'
    fail=1
  fi
}

new_tree; tag_v1
echo "ALTER TABLE x;" >>"$T/migrations/202001010000_a.sql"; commit_all edit
echo "Scenario: 配った版にある移行の中身を変えると、検査が落ちる"; expect 1

new_tree; tag_v1
git -C "$T" rm -q migrations/202001010001_b.sql migrations/202001010001_b.down.sql
printf '202001010000_a\n' >"$T/migrations/ORDER"; commit_all rm
echo "Scenario: 配った版にある移行を消すと、検査が落ちる"; expect 1

new_tree; tag_v1
git -C "$T" mv migrations/202001010001_b.sql migrations/202001010002_b.sql
git -C "$T" mv migrations/202001010001_b.down.sql migrations/202001010002_b.down.sql
printf '202001010000_a\n202001010002_b\n' >"$T/migrations/ORDER"; commit_all rename
echo "Scenario: 配った版にある移行の名前を変えると、検査が落ちる"; expect 1

new_tree
echo "ALTER TABLE x;" >>"$T/migrations/202001010000_a.sql"; commit_all edit
echo "Scenario: 配った印が無ければ、検査は通る"; expect 0

new_tree; tag_v1
echo "ALTER TABLE x;" >"$T/migrations/202001010002_c.sql"
echo "DROP x;" >"$T/migrations/202001010002_c.down.sql"
echo 202001010002_c >>"$T/migrations/ORDER"; commit_all add
echo "ALTER TABLE y;" >>"$T/migrations/202001010002_c.sql"; commit_all edit-new
echo "Scenario: 配った印の後に足した移行は、変えても通る"; expect 0

new_tree; tag_v1
echo "-- changed" >>"$T/migrations/202001010000_a.down.sql"; commit_all edit-down
echo "Scenario: 配った版にある移行の戻し手順は、変えても通る"; expect 0

# 印の指す commit が無い木: 元の台本どおり、印の ref だけ置いて commit の実体は持たない
new_tree
echo 0123456789abcdef0123456789abcdef01234567 >"$T/.git/refs/tags/v1.0.0"
echo "Scenario: 配った印の版を読めなければ、検査は通らない"; expect 1

# 祖先でない印は数えず、枝の祖先の印は数え、開発版の印も配った印にしない
new_tree; git -C "$T" tag v0.3.0
git -C "$T" checkout -qb side
echo 'CREATE TABLE side_only();' >"$T/migrations/202001010002_side.sql"
echo 'DROP TABLE side_only;' >"$T/migrations/202001010002_side.down.sql"
echo 202001010002_side >>"$T/migrations/ORDER"
git -C "$T" add -A; git -C "$T" commit -q -m side-migration; git -C "$T" tag v0.3.1
git -C "$T" checkout -q -
echo 'CREATE TABLE main_only();' >"$T/migrations/202001010002_side.sql"
echo 'DROP TABLE main_only;' >"$T/migrations/202001010002_side.down.sql"
echo 202001010002_side >>"$T/migrations/ORDER"
git -C "$T" add -A; git -C "$T" commit -q -m main-migration
echo "Scenario: 今の木の祖先でない印は、配った移行の検査で数えない"; expect 0

new_tree; git -C "$T" tag v0.3.0
git -C "$T" checkout -qb main-with-beta
echo 'CREATE TABLE beta_only();' >"$T/migrations/202001010002_beta.sql"
echo 'DROP TABLE beta_only;' >"$T/migrations/202001010002_beta.down.sql"
echo 202001010002_beta >>"$T/migrations/ORDER"
git -C "$T" add -A; git -C "$T" commit -q -m beta-migration; git -C "$T" tag v0.4.0-beta.1
git -C "$T" checkout -qb release/0.3 v0.3.0
echo "Scenario: 修正の枝の上では、枝の祖先の印を数える"; expect 0
new_tree; git -C "$T" tag nightly
echo "Scenario: 開発版の印は、配った印に数えない"; expect 0

# タグだけを取り寄せた浅い木は、印が見えても検査を続けない
new_tree; tag_v1; echo next >"$T/next"; commit_all next
git clone -q --depth=1 "file://$T" "$work/shallow"
git -C "$work/shallow" fetch -q --depth=1 origin '+refs/tags/v*:refs/tags/v*'
mkdir -p "$work/shallow/tools"
cp "$ROOT/tools/check-released-migrations.sh" "$work/shallow/tools/check-released-migrations.sh"
echo "Scenario: 浅い木では、配った移行の検査は通らない"
if out=$(cd "$work/shallow" && tools/check-released-migrations.sh 2>&1); then
  echo "  FAIL: 浅い木で検査が通った"; fail=1
elif ! grep -q '履歴が浅い' <<<"$out"; then
  echo "  FAIL: 浅い木の出力に「履歴が浅い」が無い"; printf '%s\n' "$out"; fail=1
fi

[ "$fail" -eq 0 ] && echo "check-released-migrations self-test OK" || { echo "check-released-migrations self-test NG"; exit 1; }
