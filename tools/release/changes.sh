#!/usr/bin/env bash
set -euo pipefail

usage() { echo "使い方: $0 code|deps|base|--self-test" >&2; exit 2; }

code() {
  # grep 1 本で済ます（2 本つなぐと、先の grep が SIGPIPE で落ち、pipefail が「文書だけ」と誤読する。大きな一覧で起きる）
  if grep -qvE '^(docs/|openspec/)|\.md$'; then
    exit 0
  fi
  exit 1
}

deps() {
  # 判定そのもの（この台本）を変えた PR でも vulns の job を走らせ、判定の自己試験（check-vulns.sh --self-test）を通す
  if grep -qE '^(Cargo\.lock|web/package-lock\.json|osv-scanner\.toml|tools/check-vulns(-issues)?\.(sh|py)|tools/check-pg-minor\.sh|tools/release/changes\.sh)$|^collector-android/.*\.lockfile$'; then
    exit 0
  fi
  exit 1
}

# main への push の差分の基準（design D8）。前に成功した main の push の run の commit（workflow が
# 環境変数 PREV_SUCCESS_SHA に渡す）が、この木にあり HEAD の祖先（HEAD 自身を除く）ならその commit を、
# 空・木に無い・祖先でなければ all を返す。祖先でない commit（後の commit の run が先に成功した・走らせ直した・
# force-push で消えた）を基準にすると、差分が HEAD の変更を写さず、コードの job を飛ばす（/code-review R7）。
base() {
  local sha="${PREV_SUCCESS_SHA:-}" commit head
  if [[ -n "$sha" ]] && commit=$(git rev-parse --verify -q "${sha}^{commit}") \
    && head=$(git rev-parse --verify -q 'HEAD^{commit}') && [[ "$commit" != "$head" ]] \
    && git merge-base --is-ancestor "$commit" "$head"; then
    echo "$sha"
  else
    echo all
  fi
}

self_test() {
  local self tmp
  self="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  tmp="$(mktemp -d)"
  trap "rm -rf '$tmp'" EXIT
  (
    cd "$tmp"
    git init -q .
    git config user.email t@example.invalid
    git config user.name t
    mkdir -p crates/x docs
    echo 1 >crates/x/a.rs
    git add -A && git commit -qm base
    local good; good="$(git rev-parse HEAD)"
    echo 2 >crates/x/a.rs
    git commit -qam "A: 製品に触れる（run は取り消された）"
    echo 3 >docs/b.md
    git add -A && git commit -qm "B: 文書だけ"

    echo "== Scenario: 取り消された run の後の文書だけの merge でも、コードの job が走る"
    local got; got="$(PREV_SUCCESS_SHA="$good" "$self" base)"
    [[ "$got" == "$good" ]] || { echo "NG: 基準が前に成功した commit でない: $got" >&2; exit 1; }
    git diff --name-only "$got" HEAD | "$self" code || { echo "NG: code=false になった" >&2; exit 1; }
    # 直前（A）だけを基準にした今の挙動なら文書だけに見える
    if git diff --name-only HEAD~1 HEAD | "$self" code; then echo "NG: 文書だけの差分が code になった" >&2; exit 1; fi

    echo "== Scenario: 前に成功した main の run が見つからなければ、全部走らせる"
    got="$(PREV_SUCCESS_SHA="" "$self" base)"
    [[ "$got" == all ]] || { echo "NG: 空で all でない: $got" >&2; exit 1; }
    got="$(env -u PREV_SUCCESS_SHA "$self" base)"
    [[ "$got" == all ]] || { echo "NG: 未設定で all でない: $got" >&2; exit 1; }
    got="$(PREV_SUCCESS_SHA=0123456789012345678901234567890123456789 "$self" base)"
    [[ "$got" == all ]] || { echo "NG: 木に無い commit で all でない: $got" >&2; exit 1; }

    echo "== 前に成功した run の commit が HEAD の祖先でなければ、全部走らせる"
    # 製品に触れた C1 の run を、後の C2（文書だけ）の run が成功した後に走らせ直す（/code-review R7）
    local c1 c2; c1="$(git rev-parse HEAD~1)"; c2="$(git rev-parse HEAD)"
    git checkout -q "$c1"
    got="$(PREV_SUCCESS_SHA="$c2" "$self" base)"
    [[ "$got" == all ]] || { echo "NG: 子孫の commit を基準にした: $got" >&2; exit 1; }
    # 同じ commit（走らせ直し）も差分が空になるので基準にしない
    got="$(PREV_SUCCESS_SHA="$c1" "$self" base)"
    [[ "$got" == all ]] || { echo "NG: HEAD 自身を基準にした: $got" >&2; exit 1; }
    # 枝分かれした commit（force-push で消えた側）も基準にしない
    git checkout -q -b gone "$good"; echo 4 >crates/x/a.rs; git commit -qam gone; local gone; gone="$(git rev-parse HEAD)"
    git checkout -q "$c2"
    got="$(PREV_SUCCESS_SHA="$gone" "$self" base)"
    [[ "$got" == all ]] || { echo "NG: 祖先でない commit を基準にした: $got" >&2; exit 1; }
  )
  echo "changes.sh self-test: OK"
}

case "${1:-}" in
  code) [[ $# == 1 ]] || usage; code ;;
  deps) [[ $# == 1 ]] || usage; deps ;;
  base) [[ $# == 1 ]] || usage; base ;;
  --self-test) [[ $# == 1 ]] || usage; self_test ;;
  *) usage ;;
esac
