#!/usr/bin/env bash
# 番号の印 build-<番号> を配布元へ作り（tag）、番号で組んだ配布物を配布元の下書きのリリースへ置く（place）。
# 番号の判定は version.sh number。ここは判定された番号を記録し、置くだけ（D1（仮）・D2（仮）・D7）。
# 上書き（--clobber）・公開・印 v* の操作はしない。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REMOTE="${ASHIATO_NUMBER_REMOTE:-origin}"

die() { echo "NG: $*" >&2; exit 1; }
usage() {
  echo "使い方: $0 tag <番号> <commit>|place <番号> <commit> <置き場>|--self-test" >&2
  exit 2
}

check_number() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "番号の形が違う: $1"
}

full_commit() {
  local commit
  commit=$(git -C "$ROOT" rev-parse --verify -q "$1^{commit}") || die "commit が無い: $1"
  [[ "$commit" =~ ^[0-9a-f]{40}$ ]] || die "commit が 40 桁でない: $commit"
  echo "$commit"
}

# 配布元の印 build-<番号> が指す commit（無ければ空）。軽い印でも注釈つきでも commit を返す
remote_tag_commit() {
  local number="$1" lines sha ref commit=""
  lines=$(git -C "$ROOT" ls-remote "$REMOTE" "refs/tags/build-$number" "refs/tags/build-$number^{}") || die "配布元の印を読めない"
  while read -r sha ref; do
    [[ -n "$sha" ]] || continue
    # 注釈つきの印は ^{} の行（commit）を優先する
    if [[ "$ref" == "refs/tags/build-$number^{}" || -z "$commit" ]]; then commit=$sha; fi
  done <<<"$lines"
  echo "$commit"
}

tag() {
  local number="$1" commit existing
  check_number "$number"
  commit=$(full_commit "$2")
  existing=$(remote_tag_commit "$number")
  if [[ -n "$existing" ]]; then
    [[ "$existing" == "$commit" ]] || die "印 build-$number は別の commit を指している: $existing（渡した commit $commit）"
    return 0
  fi
  if existing=$(git -C "$ROOT" rev-parse --verify -q "refs/tags/build-$number^{commit}"); then
    [[ "$existing" == "$commit" ]] || die "手元の印 build-$number が別の commit を指している: $existing（渡した commit $commit）"
  else
    git -C "$ROOT" tag "build-$number" "$commit"
  fi
  # 同じ名前の印が先にあれば、配布元がこの push を断る（D7）。強制はしない
  git -C "$ROOT" push "$REMOTE" "refs/tags/build-$number"
}

expected_names() {
  local number="$1"
  printf '%s\n' \
    "ashiato-server-${number}-linux-x86_64.tar.gz" \
    "ashiato-server-${number}-windows-x86_64.zip" \
    "ashiato-collector-windows-${number}-x86_64.zip" \
    "ashiato-collector-${number}.apk" \
    "ashiato-collector-${number}-unsigned.apk"
}

place() {
  local number="$1" commit dir="$3" name tag_commit view err rc body line want
  local -a names=() missing=() have=()
  check_number "$number"
  commit=$(full_commit "$2")
  [[ -d "$dir" ]] || die "配布物の置き場が無い: $dir"
  mapfile -t names < <(expected_names "$number")
  for name in "${names[@]}"; do
    [[ -f "$dir/$name" ]] || die "欠けた配布物: $name"
  done
  # 置く前に検査する。落ちれば配布元へ何も書かない（R10）
  "$ROOT/tools/release/check-artifacts.sh" "$dir" --allow-debug-key --expect-version "$number" >/dev/null \
    || die "配布物の検査に落ちた。配布元へ何も書かない"

  tag_commit=$(remote_tag_commit "$number")
  [[ -n "$tag_commit" ]] || die "印 build-$number が配布元に無い（先に tag を走らせる）"
  [[ "$tag_commit" == "$commit" ]] || die "印 build-$number は別の commit を指している: $tag_commit（渡した commit $commit）"

  want="ashiato-build: $number $commit"
  err=$(mktemp "${TMPDIR:-/tmp}/number-gh-err.XXXXXX")
  trap "rm -f '$err'" RETURN
  rc=0
  view=$(gh release view "build-$number" --json body,isDraft,assets 2>"$err") || rc=$?
  if (( rc != 0 )); then
    grep -qi 'release not found' "$err" || die "下書きを読めない: $(cat "$err")"
    gh release create "build-$number" --draft --title "$number" --target "$commit" --notes "$want" >/dev/null
    have=()
  else
    [[ "$(jq -r '.isDraft' <<<"$view")" == true ]] || die "build-$number は下書きでない。何も置かない"
    body=$(jq -r '.body // ""' <<<"$view")
    # 説明文の commit が渡した commit と違えば、前の commit の配布物を揃っていると読まずに止める（R3）
    line=$(grep -E '^ashiato-build: ' <<<"$body" | head -n1 || true)
    [[ "$line" == "$want" ]] || die "下書きの説明文が渡した commit と違う: ${line:-<行が無い>}（期待 $want）"
    mapfile -t have < <(jq -r '.assets[].name' <<<"$view")
  fi
  for name in "${names[@]}"; do
    local found=0 existing
    for existing in "${have[@]+"${have[@]}"}"; do [[ "$existing" == "$name" ]] && found=1; done
    (( found )) || missing+=("$dir/$name")
  done
  (( ${#missing[@]} == 0 )) && return 0
  gh release upload "build-$number" "${missing[@]}" >/dev/null
}

self_test() {
  local work out rc
  work=$(mktemp -d "${TMPDIR:-/tmp}/number-self-test.XXXXXX")
  trap 'rm -rf "${work:-}"' RETURN
  local bare="$work/origin.git" d="$work/tree" fake="$work/bin" calls="$work/gh-calls" state="$work/gh-state" pushes="$work/pushes"
  mkdir -p "$fake" "$state"
  : >"$calls"; : >"$pushes"

  # 偽の配布元（git）。push ごとに更新した参照を控える
  git init -q --bare -b main "$bare"
  cat >"$bare/hooks/pre-receive" <<EOF
#!/usr/bin/env bash
while read -r old new ref; do echo "\$ref" >>"$pushes"; done
EOF
  chmod +x "$bare/hooks/pre-receive"

  # 偽の gh。下書きは \$state/release.json に持つ。要求は全部 \$calls に残す
  cat >"$fake/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${NUMBER_FAKE_CALLS:?}"
state="${NUMBER_FAKE_STATE:?}"
case "$1 $2" in
  "release view")
    [[ -f "$state/release.json" ]] || { echo "release not found" >&2; exit 1; }
    cat "$state/release.json" ;;
  "release create")
    [[ ! -f "$state/release.json" ]] || exit 30
    notes=""; shift 2
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == --notes ]]; then notes="$2"; shift 2; else shift; fi
    done
    jq -n --arg body "$notes" '{isDraft: true, body: $body, assets: []}' >"$state/release.json" ;;
  "release upload")
    shift 2; shift   # tag
    for f in "$@"; do
      [[ "$f" != --* ]] || exit 31
      jq --arg n "$(basename "$f")" '.assets += [{name: $n}]' "$state/release.json" >"$state/release.json.new"
      mv "$state/release.json.new" "$state/release.json"
    done ;;
  *) exit 99 ;;
esac
EOF
  chmod +x "$fake/gh"

  # 使い捨ての木。number.sh は自分の置き場の根の check-artifacts.sh を呼ぶので、偽の検査（.dirty の印を持つ置き場を落とす）を置く
  mkdir -p "$d/tools/release"
  cp "$ROOT/tools/release/number.sh" "$ROOT/tools/release/version.sh" "$ROOT/tools/release/changes.sh" "$d/tools/release/"
  echo 0 >"$d/tools/release/MAJOR"
  cat >"$d/tools/release/check-artifacts.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
dir="$1"; [[ -d "$dir" ]] || exit 2
[[ "$2 $3 $4" == "--allow-debug-key --expect-version "* ]] || exit 3
! grep -rqF '.dirty' "$dir" || { echo "NG: .dirty の印がある" >&2; exit 1; }
EOF
  chmod +x "$d/tools/release/check-artifacts.sh"
  mkdir -p "$d/crates/server/src"; echo 'fn main() {}' >"$d/crates/server/src/lib.rs"
  git -C "$d" init -q -b main; git -C "$d" config user.email t@example.com; git -C "$d" config user.name t
  git -C "$d" add .; git -C "$d" commit -q -m base
  git -C "$d" remote add origin "$bare"
  echo 1 >>"$d/crates/server/src/lib.rs"; git -C "$d" add -A; git -C "$d" commit -q -m "change"
  git -C "$d" push -q origin main
  : >"$pushes"
  local c_a c_b number=0.2.1
  c_b=$(git -C "$d" rev-parse HEAD); c_a=$(git -C "$d" rev-parse HEAD^)
  local ns="$d/tools/release/number.sh"

  # 置き場（5 つの配布物。中身は検査の偽が見る印だけ）
  local good="$work/good" dirty="$work/dirty" name
  mkdir -p "$good" "$dirty"
  while IFS= read -r name; do printf '%s+%s\n' "$number" "$c_b" >"$good/$name"; done < <(expected_names "$number")
  cp "$good"/* "$dirty/"; printf '%s+%s.dirty\n' "$number" "$c_b" >"$dirty/ashiato-collector-$number.apk"

  # 偽の gh の下で走らせる。要求は $calls、配布元の状態は $state
  run_gh() { PATH="$fake:$PATH" NUMBER_FAKE_CALLS="$calls" NUMBER_FAKE_STATE="$state" "$@"; }
  reset_gh() { rm -f "$state/release.json"; : >"$calls"; }
  expect_fail() { # <理由の語> -- コマンド…
    local word="$1"; shift 2
    if out=$("$@" 2>&1); then echo "$out"; return 1; fi
    grep -q "$word" <<<"$out" || { echo "$out"; return 1; }
  }
  no_writes() { # gh の書く要求が 1 つも無い
    ! grep -Eq '^release (create|upload|edit|delete)' "$calls" || { cat "$calls"; return 1; }
  }

  echo "== Scenario: 番号の印を作る手順は、印を作る要求だけを出す"
  "$ns" tag "$number" "$c_b" >/dev/null 2>&1 || return 1
  [[ "$(git -C "$bare" rev-parse "refs/tags/build-$number")" == "$c_b" ]] || { echo "印が C を指さない"; return 1; }
  [[ "$(cat "$pushes")" == "refs/tags/build-$number" ]] || { echo "印の要求だけでない"; cat "$pushes"; return 1; }
  : >"$pushes"
  "$ns" tag "$number" "$c_b" >/dev/null 2>&1 || return 1
  [[ ! -s "$pushes" ]] || { echo "2 回目が要求を出した"; return 1; }

  echo "== Scenario: 番号の印が別の commit を指していれば、何も置かずに止まる"
  reset_gh
  expect_fail '別の commit' -- run_gh "$ns" place "$number" "$c_a" "$good" || return 1
  no_writes || return 1
  [[ ! -f "$state/release.json" ]] || return 1
  # tag も別の commit を指す印を作り直さない
  expect_fail '別の commit' -- "$ns" tag "$number" "$c_a" || return 1
  [[ ! -s "$pushes" ]] || return 1

  echo "== Scenario: 配布物の検査に落ちた置き場では、配布元へ何も書かない"
  reset_gh
  if out=$(run_gh "$ns" place "$number" "$c_b" "$dirty" 2>&1); then echo "$out"; return 1; fi
  [[ ! -s "$calls" ]] || { echo "配布元への要求が出た"; cat "$calls"; return 1; }
  [[ ! -s "$pushes" ]] || return 1

  echo "== Scenario: 番号を付けると、下書き 1 つに配布物を置く要求だけを出す"
  reset_gh
  run_gh "$ns" place "$number" "$c_b" "$good" >/dev/null 2>&1 || return 1
  [[ "$(grep -c '^release create' "$calls")" == 1 ]] || { cat "$calls"; return 1; }
  grep -q "^release create build-$number --draft --title $number --target $c_b --notes ashiato-build: $number $c_b\$" "$calls" || { cat "$calls"; return 1; }
  [[ "$(grep -c '^release upload' "$calls")" == 1 ]] || { cat "$calls"; return 1; }
  [[ "$(jq '.assets | length' "$state/release.json")" == 5 ]] || return 1
  jq -e --arg w "ashiato-build: $number $c_b" '.body == $w' "$state/release.json" >/dev/null || return 1
  ! grep -Eq 'clobber|release edit|release delete|v[0-9]|tag|push' "$calls" || { cat "$calls"; return 1; }
  [[ ! -s "$pushes" ]] || return 1

  echo "== Scenario: 同じ番号の下書きに配布物が揃っていれば、置き直さない"
  : >"$calls"
  run_gh "$ns" place "$number" "$c_b" "$good" >/dev/null 2>&1 || return 1
  no_writes || return 1

  echo "== Scenario: 下書きに欠けた配布物があれば、それだけを足す"
  : >"$calls"
  jq --arg n "ashiato-collector-windows-$number-x86_64.zip" '.assets |= map(select(.name != $n))' "$state/release.json" >"$state/r.new"
  mv "$state/r.new" "$state/release.json"
  run_gh "$ns" place "$number" "$c_b" "$good" >/dev/null 2>&1 || return 1
  [[ "$(grep -c '^release upload' "$calls")" == 1 ]] || { cat "$calls"; return 1; }
  grep -q "^release upload build-$number $good/ashiato-collector-windows-$number-x86_64.zip\$" "$calls" || { cat "$calls"; return 1; }
  ! grep -Eq '^release (create|edit|delete)|clobber' "$calls" || return 1

  echo "== Scenario: 下書きの説明文の commit が違えば、何も置かずに止まる"
  : >"$calls"
  jq --arg b "ashiato-build: $number $c_a" '.body = $b' "$state/release.json" >"$state/r.new"; mv "$state/r.new" "$state/release.json"
  jq 'del(.assets[0])' "$state/release.json" >"$state/r.new"; mv "$state/r.new" "$state/release.json"
  expect_fail '説明文' -- run_gh "$ns" place "$number" "$c_b" "$good" || return 1
  no_writes || return 1

  echo "== Scenario: 番号を付けても main の commit は増えない"
  # 製品に触れる merge を足した木で、判定（version.sh number）→ 印 → 置くを走らせ、main は前と同じ
  git -C "$d" checkout -q -b feat/p
  echo 2 >>"$d/crates/server/src/lib.rs"; git -C "$d" add -A; git -C "$d" commit -q -m "product"
  git -C "$d" checkout -q main
  git -C "$d" merge -q --no-ff -m "Merge pull request #7 from o/feat/p" feat/p
  git -C "$d" push -q origin main
  local before after next
  before=$(git -C "$d" rev-parse main); : >"$pushes"; reset_gh
  next=$(cd "$d" && tools/release/version.sh number HEAD | sed -n 's/^number=//p')
  [[ -n "$next" ]] || return 1
  "$ns" tag "$next" "$before" >/dev/null 2>&1 || return 1
  after=$(git -C "$d" rev-parse main)
  [[ "$before" == "$after" && "$(git -C "$bare" rev-parse main)" == "$before" ]] || { echo "main の commit が変わった"; return 1; }
  [[ "$(cat "$pushes")" == "refs/tags/build-$next" ]] || { echo "記録が印だけでない"; cat "$pushes"; return 1; }

  echo "OK: 番号の印と下書きへの置き場を確認した"
}

case "${1:-}" in
  --self-test) self_test ;;
  tag) [[ $# == 3 ]] || usage; tag "$2" "$3" ;;
  place) [[ $# == 4 ]] || usage; place "$2" "$3" "$4" ;;
  *) usage ;;
esac
