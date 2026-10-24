#!/usr/bin/env bash
# 試験の印と名前が base から減っていないか（CH04 の C1。Task 10.2）。
#   tools/check-tests-kept.sh [<base>] [<path>]   既定は origin/main と crates/server（rc=0 通過 / rc=1 減った / rc=2 base が無い）
#   tools/check-tests-kept.sh --self-test         使い捨ての git の木で、通すべきものが通り、落ちるべきものが落ちることを見る
# 消えた `Scenario:` の行は、足した行にも一字一句同じものがあること。
# 消えた `fn <名前>` は、足した行にも同じ名前があること —— 行で比べると、補助関数の引数を変えただけで
# 「消えた」と見てしまい、行頭から始まる `fn` も拾えなかった（2026-10-25 CH04 Q5）。
# 比べるのは重複を残した多重集合（同じ名前の試験が 2 本あって 1 本消えれば落ちる）。
set -euo pipefail

# check <base> <path> : いまの作業ディレクトリの git の木で比べる。rc を返す（exit しない）
check() {
  local base=$1 path=$2 d lost_scn lost_fn rc=0
  if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
    echo "[FAIL] base の ref が見つからない: $base（git fetch で取ってくるか、別の base を渡す）" >&2
    return 2
  fi
  d=$(git diff "$base"...HEAD -- "$path")

  removed() { printf '%s\n' "$d" | grep -E "^-([^-].*)?$1" | sed 's/^-//' || true; }
  added() { printf '%s\n' "$d" | grep -E "^\+([^+].*)?$1" | sed 's/^+//' || true; }
  names() { grep -oE 'fn [A-Za-z0-9_]+' || true; }

  # sort -u にしない: comm は並べた入力の重複を 1 つずつ数える
  lost_scn=$(comm -23 <(removed 'Scenario:' | sort) <(added 'Scenario:' | sort) | grep -v '^$' || true)
  lost_fn=$(comm -23 <(removed 'fn ' | names | sort) <(added 'fn ' | names | sort) | grep -v '^$' || true)

  if [ -n "$lost_scn" ]; then
    echo "[FAIL] 消えた Scenario の印:"; printf '  %s\n' "$lost_scn"; rc=1
  fi
  if [ -n "$lost_fn" ]; then
    echo "[FAIL] 消えた fn の名前:"; printf '  %s\n' "$lost_fn"; rc=1
  fi
  [ "$rc" -eq 0 ] && echo "[OK] Scenario の印と fn の名前は $base から減っていない（$path）"
  return "$rc"
}

self_test() {
  local fail=0 base out rc
  tmp=$(mktemp -d)  # trap が関数の外で走るので大域
  trap 'rm -rf "$tmp"' EXIT
  g() { git -C "$tmp/r" -c user.name=t -c user.email=t@example.invalid -c core.hooksPath=/dev/null "$@"; }
  # fresh <base の中身を作る命令> : 木を作り直して base を commit し、branch を切る
  fresh() {
    rm -rf "$tmp/r"; mkdir -p "$tmp/r/src"
    git init -q "$tmp/r"
    (cd "$tmp/r" && eval "$1")
    g add -A; g commit -qm base --no-verify
    base=$(g rev-parse HEAD)
    g checkout -qb work
  }
  commit() { g add -A; g commit -qm work --no-verify --allow-empty; }
  run() { set +e; out=$(cd "$tmp/r" && check "$1" src 2>&1); rc=$?; set -e; }
  want() {  # want <説明> <条件の rc>
    if [[ $2 == 0 ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1" >&2; fail=1; fi
  }
  # 印の字は分けて組む（この台本が Scenario の印の検査に拾われないように）
  local m="Scen""ario:"
  local seed="printf '// $m 甲\\nfn a_test() {}\\nfn helper(x: i32) {}\\n' > src/a.rs"

  fresh "$seed"
  printf '// %s 甲\nfn a_test() {}\nfn helper(x: i32, y: i32) {}\nfn b_test() {}\n' "$m" >"$tmp/r/src/a.rs"; commit
  run "$base"
  want "残っていれば（引数を変え、足しただけ）rc=0" "$([[ $rc == 0 ]]; echo $?)"

  fresh "$seed"
  printf '// %s 甲\nfn helper(x: i32) {}\n' "$m" >"$tmp/r/src/a.rs"; commit
  run "$base"
  want "fn を消せば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "消えた名前を挙げる" "$(grep -q 'fn a_test' <<<"$out"; echo $?)"

  fresh "$seed"
  printf 'fn a_test() {}\nfn helper(x: i32) {}\n' >"$tmp/r/src/a.rs"; commit
  run "$base"
  want "印の行を消せば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "消えた印を挙げる" "$(grep -q "$m 甲" <<<"$out"; echo $?)"

  # 同じ名前の試験と印が 2 つのファイルにあり、両方消して 1 つだけ足し直す（重複を潰すと見逃す）
  fresh "printf '// $m 乙\\nfn same() {}\\n' > src/a.rs; cp src/a.rs src/b.rs"
  rm "$tmp/r/src/a.rs" "$tmp/r/src/b.rs"
  printf '// %s 乙\nfn same() {}\n' "$m" >"$tmp/r/src/c.rs"; commit
  run "$base"
  want "同じ名前の 2 本のうち 1 本が消えれば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "重複の fn も挙げる" "$(grep -q 'fn same' <<<"$out"; echo $?)"
  want "重複の印も挙げる" "$(grep -q "$m 乙" <<<"$out"; echo $?)"

  run no-such-ref
  want "base が無ければ rc=0 でない" "$([[ $rc != 0 ]]; echo $?)"
  want "無い ref の名前を出す" "$(grep -q 'no-such-ref' <<<"$out"; echo $?)"

  [[ $fail == 0 ]] && echo "self-test OK" || { echo "self-test FAILED" >&2; exit 1; }
}

case "${1:-}" in
  --self-test) self_test ;;
  *)
    cd "$(dirname "$0")/.."
    rc=0; check "${1:-origin/main}" "${2:-crates/server}" || rc=$?
    exit "$rc"
    ;;
esac
