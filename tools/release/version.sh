#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

die() { echo "NG: $*" >&2; exit 1; }
usage() {
  echo "使い方: $0 source [<Cargo.toml>]|major|check <版番号>|code <版番号>|order|plan <印>|nightly-plan|number <commit>|--self-test" >&2
  exit 2
}

# 出所の版を読む唯一の bash の入口（既定は根の Cargo.toml。検査の使い捨ての木は道を渡す。/code-review R40）
source_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/{s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p;}' "${1:-$ROOT/Cargo.toml}" | head -n1
}

parse_version() {
  local value="$1"
  if [[ "$value" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-beta\.([1-9]|[1-8][0-9]|9[0-8])|-dev\.[0-9]{8})?$ ]]; then
    # 先頭の 0 を 8 進数と読まない（Kotlin・Rust・TS と同じ 10 進数。/code-review R36）
    MAJOR=$((10#${BASH_REMATCH[1]})); MINOR=$((10#${BASH_REMATCH[2]})); PATCH=$((10#${BASH_REMATCH[3]}))
    SUFFIX=${BASH_REMATCH[4]}
    BETA=${BASH_REMATCH[5]:+$((10#${BASH_REMATCH[5]}))}
    return 0
  fi
  return 1
}

# 前後の空白（CRLF の取り出しの \r を含む）を落とす。Rust・gradle・画面の trim と同じ（/code-review R6）
trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  printf '%s' "${value%"${value##*[![:space:]]}"}"
}

# 左の数字（tools/release/MAJOR）。数字 1 つでなければ落ちる（D9）
major_file() {
  [[ -z "${1:-}" ]] && set -- "$ROOT/tools/release/MAJOR"
  local value
  value=$(cat "${1:-$ROOT/tools/release/MAJOR}" 2>/dev/null) || die "左の数字のファイルが読めない: ${1:-$ROOT/tools/release/MAJOR}"
  value=$(trim "$value")
  [[ "$value" =~ ^[0-9]+$ ]] || die "左の数字のファイルが数字 1 つでない: ${value:-<空>}"
  echo $((10#$value))
}

# D3: 左×10,000,000 + 真ん中×10,000 + 右。後ろの -beta・-dev は値に入れない
version_code() {
  local value="$1"
  parse_version "$value" || die "版番号の形が違う: $value"
  (( MAJOR <= 209 )) || die "左の数字が上限 209 を超えた: $MAJOR（超えた部分 $((MAJOR - 209))）"
  (( MINOR <= 999 )) || die "真ん中の数字が上限 999 を超えた: $MINOR（超えた部分 $((MINOR - 999))）"
  (( PATCH <= 9999 )) || die "右の数字が上限 9999 を超えた: $PATCH（超えた部分 $((PATCH - 9999))）"
  echo $((MAJOR * 10000000 + MINOR * 10000 + PATCH))
}

# D9: 番号を持たない版（3 つの数字が出所の版と同じ）か、番号（左が MAJOR と同じで <左>.<真ん中> が 0.0 でない）
check_rule() {
  local value="$1" expected="$2" major="$3" value_major value_minor value_patch
  parse_version "$value" || die "版番号の形が違う: $value"
  value_major=$MAJOR; value_minor=$MINOR; value_patch=$PATCH
  # 出所の版は 0.0.<N>（N は 1 以上）だけ（spec。Rust・gradle・画面と同じ。/code-review R8）
  parse_version "$expected" && [[ -z "$SUFFIX" ]] && (( MAJOR == 0 && MINOR == 0 && PATCH >= 1 )) \
    || die "出所の版が 0.0.<N>（N は 1 以上）の形でない: $expected"
  if [[ "$value_major.$value_minor.$value_patch" != "$MAJOR.$MINOR.$PATCH" ]]; then
    (( value_major == major )) || die "左の数字が MAJOR（$major）と違う: $value"
    (( value_major != 0 || value_minor != 0 )) || die "番号の <左>.<真ん中> が 0.0: $value"
  fi
  version_code "$value" >/dev/null
  echo "$value"
}

check_version() {
  local major
  major=$(major_file) || exit 1
  check_rule "$1" "$(source_version)" "$major"
}

version_sort_key() {
  local value=${1#v}
  parse_version "$value" || return 1
  local kind=2 number=99
  [[ "$SUFFIX" == -beta.* ]] && { kind=1; number=$BETA; }
  [[ "$SUFFIX" == -dev.* ]] && { kind=0; number=0; }
  printf '%09d.%09d.%09d.%03d.%03d\n' "$MAJOR" "$MINOR" "$PATCH" "$kind" "$number"
}

order_versions() {
  local line value key
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    value=${line#refs/tags/}; value=${value#v}
    key=$(version_sort_key "$value") || die "並べられない印: $line"
    printf '%s\t%s\n' "$key" "$line"
  done | sort -k1,1 | cut -f2-
}

require_full_history() {
  local shallow
  # 問い合わせが失敗したら浅くないとはみなさない（final review R17）
  shallow=$(git rev-parse --is-shallow-repository) || die "履歴が浅いかを確かめられない"
  [[ "$shallow" != true ]] || die "履歴が浅い"
}

main_ref() {
  local ref
  if ref=$(git rev-parse --verify refs/remotes/origin/main 2>/dev/null); then
    echo "$ref"
  elif ref=$(git rev-parse --verify refs/heads/main 2>/dev/null); then
    echo "$ref"
  else
    die "main の参照が無い"
  fi
}

plan() {
  local tag=${1:-} commit main version numbers name
  [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.([1-9]|[1-8][0-9]|9[0-8]))?$ ]] || die "印の形が違う: $tag"
  require_full_history
  commit=$(git rev-parse --verify "refs/tags/$tag^{commit}" 2>/dev/null) || die "印が無い: $tag"
  version=${tag#v}
  check_version "$version" >/dev/null
  main=$(main_ref)
  git merge-base --is-ancestor "$commit" "$main" || die "印の commit が main の祖先でない"
  # 移る間（ST62・ST63 まで）の D11: 印の commit に番号の印 build-<番号> があり、印の 3 つの数字が同じときだけ通す
  numbers=
  while IFS= read -r name; do
    [[ "$name" =~ ^build-[0-9]+\.[0-9]+\.[0-9]+$ ]] && numbers+=" ${name#build-}"
  done < <(git tag --points-at "$commit" --list 'build-*')
  [[ -n "$numbers" ]] || die "印の commit に番号が無い: $tag"
  [[ " $numbers " == *" ${version%%-*} "* ]] || die "印の版とその commit の番号が違う: $version（番号${numbers}）"
  local channel=release prerelease=false
  [[ "$version" == *-beta.* ]] && { channel=beta; prerelease=true; }
  echo "channel=$channel"
  echo "version=$version"
  echo "version_code=$(version_code "$version")"
  echo "prerelease=$prerelease"
}

# D11（移る間。ST62・ST63 まで）: main の HEAD でなく、main の first-parent の上で最も大きい番号の commit を組む
nightly_plan() {
  local main nightly tag event now date first_parent number commit latest="" latest_commit="" build
  require_full_history
  main=$(main_ref)
  event=${GITHUB_EVENT_NAME:-schedule}
  now=${ASHIATO_NOW:-}
  if [[ -n "$now" ]]; then date=$(TZ=Asia/Tokyo date -d "$now" +%Y%m%d); else date=$(TZ=Asia/Tokyo date +%Y%m%d); fi
  if tag=$(git rev-parse --verify 'refs/tags/nightly^{commit}' 2>/dev/null); then
    nightly=$tag
  else
    nightly=
  fi
  first_parent=$(git rev-list --first-parent "$main")
  while IFS= read -r number; do
    commit=$(git rev-parse --verify "refs/tags/build-$number^{commit}" 2>/dev/null) || continue
    grep -qx "$commit" <<<"$first_parent" || continue
    latest=$number; latest_commit=$commit
  done < <(build_numbers)
  if [[ "${ASHIATO_NIGHTLY_PUBLISH:-}" == on ]]; then echo "publish=yes"; else echo "publish=no"; fi
  if [[ -z "$latest" ]]; then
    echo "build=no"
    echo "番号がまだ無い"
    return 0
  fi
  if [[ -n "$nightly" && "$nightly" == "$latest_commit" ]]; then
    build=no
  elif [[ "$event" == schedule && "${ASHIATO_NIGHTLY_PUBLISH:-}" != on ]]; then
    build=no
  else
    build=yes
  fi
  echo "build=$build"
  echo "version=$latest-dev.$date"
  echo "commit=$latest_commit"
}

# 番号の印 build-<番号> の一覧を、番号の小さい順に並べる（D7）
build_numbers() {
  local name key
  while IFS= read -r name; do
    [[ "$name" =~ ^build-[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
    key=$(version_sort_key "${name#build-}") || continue
    printf '%s\t%s\n' "$key" "${name#build-}"
  done < <(git tag --list 'build-*') | sort -k1,1 | cut -f2
}

# D4・D5・D7・D13・D14・D15: commit に付ける番号を決める。印は作らない（作るのは number.sh）
number() {
  local arg=${1:-} changed first_parent commit main prev prev_commit known="" tagged="" name
  local left prev_left prev_minor prev_patch minor patch subject parents
  require_full_history
  commit=$(git rev-parse --verify "$arg^{commit}" 2>/dev/null) || die "commit が無い: $arg"
  # 4. 印が既にある commit は、最新の番号でなくても、その番号を返す
  while IFS= read -r name; do
    [[ "$name" =~ ^build-[0-9]+\.[0-9]+\.[0-9]+$ ]] && tagged=${name#build-}
  done < <(git tag --points-at "$commit" --list 'build-*' | sort -V)
  if [[ -n "$tagged" ]]; then
    echo "number=$tagged"; echo "reuse=yes"
    return 0
  fi
  main=$(main_ref)
  # grep -q で早く閉じると rev-list が SIGPIPE で落ち、pipefail が拾うので、一覧を先に取る
  first_parent=$(git rev-list --first-parent "$main")
  grep -qx "$commit" <<<"$first_parent" || { echo "number="; echo "main の first-parent の上に無い commit には番号を付けない"; return 0; }
  prev=$(build_numbers | tail -n1)
  if [[ -n "$prev" ]]; then
    prev_commit=$(git rev-parse --verify "refs/tags/build-$prev^{commit}") || die "前の番号の印が読めない: build-$prev"
    if [[ "$prev_commit" == "$commit" ]] || ! git merge-base --is-ancestor "$prev_commit" "$commit"; then
      echo "number="; echo "前の番号（$prev）の commit の子孫でない commit には番号を付けない"; return 0
    fi
    known=$prev_commit
  else
    known=$(git rev-parse --verify "$commit^1" 2>/dev/null) || known=$(git hash-object -t tree /dev/null)
  fi
  # 3. 差分が製品に触れるか（判定は PR の CI と同じ changes.sh）
  # 差分を先に取る（changes.sh が早く閉じると git diff が SIGPIPE で落ち、pipefail が「文書だけ」と誤読する）
  changed=$(git diff --name-only "$known" "$commit")
  if ! "$ROOT/tools/release/changes.sh" code <<<"$changed"; then
    echo "number="; echo "前の番号から製品が変わっていない（docs/・openspec/・*.md だけ）"; return 0
  fi
  # 左の数字は、その commit の中のファイルから読む
  left=$(git show "$commit:tools/release/MAJOR" 2>/dev/null) || die "左の数字のファイルが読めない: $commit"
  left=$(trim "$left")
  [[ "$left" =~ ^[0-9]+$ ]] || die "左の数字のファイルが数字 1 つでない: ${left:-<空>}"
  left=$((10#$left))
  if [[ -z "$prev" ]]; then
    (( left == 0 )) && { minor=2; patch=0; } || { minor=0; patch=0; }
  else
    parse_version "$prev"; prev_left=$MAJOR; prev_minor=$MINOR; prev_patch=$PATCH
    if (( left < prev_left )); then
      die "左の数字（$left）が前の番号（$prev）より小さい"
    elif (( left > prev_left )); then
      minor=0; patch=0
    else
      # D5・D15: 前の番号の commit を除く first-parent に確認バッチの merge があれば真ん中を上げる
      minor=$prev_minor; patch=$((prev_patch + 1))
      while IFS= read -r subject; do
        if [[ "$subject" =~ ^Merge\ pull\ request\ \#[0-9]+\ from\ [^/]+/verify/ ]]; then minor=$((prev_minor + 1)); patch=0; break; fi
      done < <(git log --first-parent --merges --format=%s "$prev_commit..$commit")
    fi
  fi
  # D14: 上限を超える番号は付けない（version_code が超えた数字を示して落ちる）
  version_code "$left.$minor.$patch" >/dev/null
  echo "number=$left.$minor.$patch"
  echo "reuse=no"
}

number_self_test() {
  local work d out rc n=0
  work=$(mktemp -d "${TMPDIR:-/tmp}/version-number-test.XXXXXX")
  trap 'rm -rf "${work:-}"' RETURN
  # d <名前>: 使い捨ての木（main・番号の印なし・左は 0）
  nt_new() {
    d="$work/$1"; mkdir -p "$d/tools/release" "$d/crates/server/src" "$d/docs"
    cp "$ROOT/tools/release/version.sh" "$ROOT/tools/release/changes.sh" "$d/tools/release/"
    echo 0 > "$d/tools/release/MAJOR"; echo 'fn main() {}' > "$d/crates/server/src/lib.rs"; echo doc > "$d/docs/a.txt"
    git -C "$d" init -q -b main; git -C "$d" config user.email t@example.com; git -C "$d" config user.name t
    git -C "$d" add .; git -C "$d" commit -q -m base
  }
  # nt_merge <枝> <ファイル> [中身]: --no-ff の merge（題は配布元の形）を main に足す
  nt_merge() {
    n=$((n + 1))
    git -C "$d" checkout -q -b "$1"
    mkdir -p "$(dirname "$d/$2")"
    if [[ $# -ge 3 ]]; then printf '%s\n' "$3" > "$d/$2"; else echo "$n" >> "$d/$2"; fi
    git -C "$d" add -A; git -C "$d" commit -q -m "change $n"
    git -C "$d" checkout -q main
    git -C "$d" merge -q --no-ff -m "Merge pull request #$n from o/$1" "$1"
  }
  nt_tag() { git -C "$d" tag "build-$1" "${2:-HEAD}"; }
  nt_number() { (cd "$d" && tools/release/version.sh number "${1:-HEAD}" 2>&1); }
  nt_expect() { # 期待する番号の行
    out=$(nt_number "${2:-HEAD}") || { echo "$out"; return 1; }
    grep -qx "number=$1" <<<"$out" || { echo "$out"; return 1; }
  }
  nt_refuse() { # 0 でない終了コード・理由・番号なし
    if out=$(nt_number); then echo "$out"; return 1; fi
    grep -q "$1" <<<"$out" && ! grep -q '^number=[0-9]' <<<"$out" || { echo "$out"; return 1; }
  }

  echo "== Scenario: 製品に触れた merge の commit の run が成功すると、次の番号が付く"
  nt_new product; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs
  nt_expect 0.2.1 || return 1; grep -qx 'reuse=no' <<<"$out" || return 1
  echo "== Scenario: 文書だけの merge の commit には番号を付けない"
  nt_new docs; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge docs/x docs/b.md; nt_merge docs/y openspec/c.md
  nt_expect '' || return 1
  echo "== Scenario: 道具だけの merge の commit にも番号が付く"
  nt_new tool; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge chore/t tools/lanes.json
  nt_expect 0.2.1 || return 1
  echo "== Scenario: 大きな差分（数千のファイル名）でも、製品に触れていれば番号が付く"
  # 2 本の grep をつなぐと SIGPIPE で rc=1（文書だけ）に誤読した。数千の名前で確かめる
  nt_new bigdiff; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0
  git -C "$d" checkout -q -b feat/big
  mkdir -p "$d/docs/big"; (cd "$d/docs/big" && seq 1 4000 | sed 's/^/n/;s/$/.txt/' | xargs touch)
  mkdir -p "$d/crates/server/src"; echo big >> "$d/crates/server/src/lib.rs"
  git -C "$d" add -A; git -C "$d" commit -q -m big; git -C "$d" checkout -q main
  git -C "$d" merge -q --no-ff -m "Merge pull request #99 from o/feat/big" feat/big
  nt_expect 0.2.1 || return 1
  out=$(seq 1 20000 | sed 's/^/crates\/f/'); "$d/tools/release/changes.sh" code <<<"$out" || { echo "changes.sh code が大きな一覧で rc!=0"; return 1; }
  echo "== Scenario: 取り消された merge の中身は、後の文書だけの merge の番号に入る"
  nt_new cancelled; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs
  [[ -z "$(git -C "$d" tag --list 'build-*' --points-at HEAD)" ]] || return 1
  nt_merge docs/x docs/b.md; nt_expect 0.2.1 || return 1
  echo "== Scenario: 番号を付けた commit の祖先の run が後から成功しても、番号を付けない"
  nt_new ancestor; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs
  local a; a=$(git -C "$d" rev-parse HEAD); nt_merge feat/c crates/server/src/lib.rs; nt_tag 0.2.1
  nt_expect '' "$a" || return 1
  echo "== main の first-parent の上に無い commit（merge した枝の中の commit）には番号を付けない"
  # 前の番号の子孫で、製品に触れていても、main の first-parent の上に無ければ付けない（design D4 の 2.。code-verify R5）
  nt_new side; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs
  out=$(nt_number feat/b) || { echo "$out"; return 1; }
  grep -qx 'number=' <<<"$out" && grep -q 'first-parent の上に無い' <<<"$out" || { echo "$out"; return 1; }
  nt_expect 0.2.1 || return 1
  nt_new ancestor2; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs
  a=$(git -C "$d" rev-parse HEAD); nt_merge feat/c crates/server/src/lib.rs; nt_tag 0.2.1
  echo "== Scenario: 同じ commit の run を走らせ直しても、番号は変わらない"
  nt_expect 0.2.1 || return 1; grep -qx 'reuse=yes' <<<"$out" || return 1
  echo "== Scenario: 欠番の commit を走らせ直すと、後の番号が付いていても、その番号を返す"
  nt_new gap; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs; a=$(git -C "$d" rev-parse HEAD); nt_tag 0.2.1
  nt_merge feat/c crates/server/src/lib.rs; nt_tag 0.2.2
  nt_expect 0.2.1 "$a" || return 1; grep -qx 'reuse=yes' <<<"$out" || return 1
  echo "== Scenario: 番号の印が無い木では、文書だけの merge に番号を付けない"
  nt_new notag; nt_merge docs/x docs/b.md; nt_expect '' || return 1
  echo "== Scenario: 最初の番号は 0.2.0"
  nt_new first; nt_merge feat/a crates/server/src/lib.rs; nt_expect 0.2.0 || return 1
  echo "== Scenario: 間に確認バッチの merge が無ければ、右の数字が 1 上がる"
  nt_new patch; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.3; nt_merge fix/x crates/server/src/lib.rs; nt_expect 0.2.4 || return 1
  echo "== Scenario: 間に確認バッチの merge があれば、真ん中が上がり右が 0 に戻る"
  nt_new verify-then-docs; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.3
  nt_merge verify/2026-11-23 crates/server/src/lib.rs; nt_merge docs/x docs/b.md; nt_expect 0.3.0 || return 1
  echo "== Scenario: 確認バッチの merge 自身の番号は、真ん中が上がる"
  nt_new verify-self; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.3
  nt_merge verify/2026-11-23 crates/server/src/lib.rs; nt_expect 0.3.0 || return 1
  echo "== Scenario: 前の番号の commit が確認バッチの merge でも、次は右が上がる"
  nt_tag 0.3.0; nt_merge chore/t tools/lanes.json; nt_expect 0.3.1 || return 1
  echo "== Scenario: 左の数字のファイルを上げると、真ん中と右が 0 になる"
  nt_new major-up; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.7.12; nt_merge chore/major tools/release/MAJOR 1
  nt_expect 1.0.0 || return 1
  echo "== Scenario: 左の数字のファイルを下げると、番号を付けずに止まる"
  nt_new major-down; nt_merge chore/major tools/release/MAJOR 1; nt_tag 1.0.3; nt_merge chore/major2 tools/release/MAJOR 0
  nt_refuse '左の数字.*小さい' || return 1
  echo "== Scenario: 左の数字のファイルが数字 1 つでなければ、番号を付けずに止まる"
  nt_new major-bad; nt_merge chore/major tools/release/MAJOR 0.1; nt_refuse '数字 1 つでない' || return 1
  echo "== Scenario: 真ん中か左が上限を超える番号は付けない"
  nt_new minor-cap; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.999.4; nt_merge verify/2026-11-23 crates/server/src/lib.rs
  nt_refuse '真ん中の数字が上限 999 を超えた' || return 1
  nt_new major-cap; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge chore/major tools/release/MAJOR 210
  nt_refuse '左の数字が上限 209 を超えた' || return 1
  echo "== Scenario: 携帯の versionCode に収まらない番号は付けない"
  nt_new patch-cap; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.9999; nt_merge feat/b crates/server/src/lib.rs
  nt_refuse '右の数字が上限 9999 を超えた' || return 1
  echo "== Scenario: 組み立てが落ちた番号は、次の commit に回さない"
  nt_new failed-build; nt_merge feat/a crates/server/src/lib.rs; nt_tag 0.2.0; nt_merge feat/b crates/server/src/lib.rs; nt_tag 0.2.1
  nt_merge feat/c crates/server/src/lib.rs; nt_expect 0.2.2 || return 1
  echo "== Scenario: 浅い木では、番号付けの判定は通らない"
  git clone -q --depth=1 "file://$d" "$work/shallow-number"
  if out=$(cd "$work/shallow-number" && tools/release/version.sh number HEAD 2>&1); then return 1; fi
  grep -q '履歴が浅い' <<<"$out" && ! grep -q '^number=' <<<"$out" || return 1
}

self_test() {
  local table="$ROOT/tools/release/version-table.tsv" line version expected source left verdict got
  while IFS=$'\t' read -r version expected; do
    [[ -z "$version" || "$version" == \#* ]] && continue
    if [[ "$expected" == - ]]; then
      if (version_code "$version" >/dev/null 2>&1); then die "version-table.tsv: 拒むはずの $version を受けた"; fi
      continue
    fi
    [[ "$(version_code "$version")" == "$expected" ]] || die "version-table.tsv: $version"
  done < "$table"
  # D9: 受ける・断る版番号の表を check の規則に全行当てる（gradle・Rust・TypeScript の試験も同じ表を読む）
  while IFS=$'\t' read -r version source left verdict; do
    [[ -z "$version" || "$version" == \#* ]] && continue
    if (check_rule "$version" "$source" "$left" >/dev/null 2>&1); then got=ok; else got=ng; fi
    [[ "$got" == "$verdict" ]] || die "version-check-table.tsv: $version（出所 $source・左 $left）は $verdict のはずが $got"
  done < "$ROOT/tools/release/version-check-table.tsv"
  [[ "$(major_file)" =~ ^[0-9]+$ ]] || return 1
  # MAJOR が壊れていれば check は落ちる（NG の文だけ出して rc=0 で受けない）
  local broken; broken=$(mktemp -d "${TMPDIR:-/tmp}/version-major.XXXXXX")
  mkdir -p "$broken/tools/release"; cp "$ROOT/tools/release/version.sh" "$ROOT/tools/release/version-check-table.tsv" "$broken/tools/release/"
  printf '[workspace.package]\nversion = "0.0.1"\n' > "$broken/Cargo.toml"
  for content in x '' '+0' '0 0' '-0'; do
    printf '%s' "$content" > "$broken/tools/release/MAJOR"
    if "$broken/tools/release/version.sh" check 0.2.0 >/dev/null 2>&1; then rm -rf "$broken"; return 1; fi
  done
  # Windows の取り出し（CRLF）と前後の空白は落とさない（Rust・gradle・画面と同じ。/code-review R6）
  for content in $'0\r\n' $' 0 \n' $'0\n'; do
    printf '%s' "$content" > "$broken/tools/release/MAJOR"
    "$broken/tools/release/version.sh" check 0.2.0 >/dev/null 2>&1 || { echo "MAJOR の前後の空白で check が落ちた: $(od -c "$broken/tools/release/MAJOR" | head -n1)"; rm -rf "$broken"; return 1; }
  done
  rm -rf "$broken"
  echo "== Scenario: versionCode は版番号の 3 つの数字から決まる"
  [[ "$(version_code 0.0.1)" == 1 && "$(version_code 0.2.0)" == 20000 && "$(version_code 0.2.7)" == 20007 && "$(version_code 0.3.0)" == 30000 && "$(version_code 0.250.9999)" == 2509999 && "$(version_code 1.0.0)" == 10000000 ]] || return 1
  echo "== Scenario: 後ろに -beta や -dev が付いても、同じ番号は同じ versionCode になる"
  for version in 0.4.0 0.4.0-beta.2 0.4.0-dev.20261101 0.4.0-dev.20261102; do [[ "$(version_code "$version")" == 40000 ]] || return 1; done
  echo "== Scenario: 最初の番号は、前の式の 0.1.x の APK より大きい"
  (( $(version_code 0.2.0) == 20000 && $(version_code 0.2.0) > 10199 )) || return 1
  echo "== Scenario: 桁を超える版番号では、versionCode の計算が失敗する"
  for version in 0.1000.0 0.1.10000 210.0.0; do
    if got=$(version_code "$version" 2>&1); then return 1; fi
    grep -q '超えた' <<<"$got" || return 1
  done
  if (version_code 0.4.0-beta.99 >/dev/null 2>&1); then return 1; fi
  # 先頭の 0 は 10 進数で読む（gradle・Rust と同じ値。/code-review R36）
  [[ "$(version_code 0.010.0)" == 100000 && "$(version_code 0.08.0)" == 80000 ]] || return 1
  [[ "$(version_sort_key 0.08.0)" == "$(version_sort_key 0.8.0)" ]] || return 1

  number_self_test || return 1

  local work t base out rc release_base formal_base numbered
  work=$(mktemp -d "${TMPDIR:-/tmp}/version-self-test.XXXXXX")
  trap 'rm -rf "$work"' RETURN
  t="$work/tree"; mkdir -p "$t/tools/release" "$t/migrations" "$t/docs/contract"
  cp "$ROOT/tools/release/version.sh" "$t/tools/release/version.sh"
  cp "$ROOT/tools/release/version-table.tsv" "$t/tools/release/version-table.tsv"
  cp "$ROOT/tools/release/MAJOR" "$t/tools/release/MAJOR"
  printf '[workspace.package]\nversion = "0.0.1"\n' > "$t/Cargo.toml"
  : > "$t/migrations/ORDER"; echo '{}' > "$t/docs/contract/envelope.json"
  echo 'CREATE TABLE init();' > "$t/migrations/202601010000_init.sql"
  git -C "$t" init -q; git -C "$t" config user.email t@example.com; git -C "$t" config user.name t
  git -C "$t" add .; git -C "$t" commit -q -m base; base=$(git -C "$t" rev-parse HEAD); git -C "$t" branch main; git -C "$t" tag build-0.4.0
  git -C "$t" tag v0.4.0
  echo "== Scenario: 正式版の印は、通常のリリースと判定される"; out=$(cd "$t" && tools/release/version.sh plan v0.4.0); grep -q 'channel=release' <<<"$out"; grep -q 'version=0.4.0' <<<"$out"
  echo "== Scenario: ベータ版の印で、先行公開のリリースになる"; git -C "$t" tag v0.4.0-beta.2; out=$(cd "$t" && tools/release/version.sh plan v0.4.0-beta.2); grep -q 'channel=beta' <<<"$out"
  git -C "$t" tag v0.4.0-beta.98; out=$(cd "$t" && tools/release/version.sh plan v0.4.0-beta.98); grep -q 'version_code=40000' <<<"$out"
  echo "== Scenario: 印の版とその commit の番号が違えば公開しない"; git -C "$t" tag v0.5.0-beta.1
  if out=$(cd "$t" && tools/release/version.sh plan v0.5.0-beta.1 2>&1); then return 1; fi
  grep -q '番号が違う' <<<"$out" || return 1
  echo "== Scenario: main の祖先でない commit の印は公開しない"; git -C "$t" checkout -qb other; echo x > "$t/x"; git -C "$t" add x; git -C "$t" commit -q -m other; git -C "$t" tag v0.4.0-beta.3
  if out=$(cd "$t" && tools/release/version.sh plan v0.4.0-beta.3 2>&1); then return 1; fi
  grep -q '祖先でない' <<<"$out" || return 1
  echo "== Scenario: 決めた形でない印は、どのチャンネルにも配らない"; for version in v0.4.0-rc.1 v0.4 release-0.4.0 v0.4.0-beta.99; do
    if (cd "$t" && tools/release/version.sh plan "$version" >/dev/null 2>&1); then return 1; fi
  done
  git -C "$t" checkout -q main; git clone -q --depth=1 "file://$t" "$work/shallow"; echo "== Scenario: 浅い木では、印の判定は通らない"
  if out=$(cd "$work/shallow" && tools/release/version.sh plan v0.4.0 2>&1); then return 1; fi
  grep -q '履歴が浅い' <<<"$out" || return 1

  ordered=$(printf '%s\n' v0.4.0 v0.4.0-beta.10 v0.4.0-beta.9 v0.4.0-beta.2 | (cd "$t" && tools/release/version.sh order))
  [[ "$ordered" == $'v0.4.0-beta.2\nv0.4.0-beta.9\nv0.4.0-beta.10\nv0.4.0' ]] || return 1
  git clone -q "file://$t" "$work/no-main"; git -C "$work/no-main" checkout -qb test-no-main; git -C "$work/no-main" branch -D main >/dev/null; git -C "$work/no-main" update-ref -d refs/remotes/origin/main
  if out=$(cd "$work/no-main" && tools/release/version.sh plan v0.4.0 2>&1); then return 1; fi
  grep -q 'main の参照が無い' <<<"$out" || return 1

  echo "== Scenario: 修正の枝の印も、main の祖先でなければ公開しない"
  git -C "$t" tag v0.3.0 "$base"; git -C "$t" checkout -q -b release/0.3 v0.3.0
  echo fix > "$t/fix"; git -C "$t" add fix; git -C "$t" commit -q -m hotfix; git -C "$t" tag v0.3.1
  if out=$(cd "$t" && tools/release/version.sh plan v0.3.1 2>&1); then return 1; fi
  grep -q '祖先でない' <<<"$out" || return 1
  # CI と同じ形: clone した側で refs/remotes/origin/release/0.3 の先を印で取る
  git clone -q "file://$t" "$work/hotfix-clone"; git -C "$work/hotfix-clone" checkout -q --detach v0.3.1
  if out=$(cd "$work/hotfix-clone" && tools/release/version.sh plan v0.3.1 2>&1); then return 1; fi
  grep -q '祖先でない' <<<"$out" || return 1
  git -C "$t" checkout -q main
  echo "== Scenario: 番号の無い commit の印は公開しない"
  echo z > "$t/z"; git -C "$t" add z; git -C "$t" commit -q -m no-number
  git -C "$t" tag -d v0.4.0 >/dev/null; git -C "$t" tag v0.4.0
  if out=$(cd "$t" && tools/release/version.sh plan v0.4.0 2>&1); then return 1; fi
  grep -q '印の commit に番号が無い' <<<"$out" || return 1

  git -C "$t" tag -d v0.4.0 >/dev/null
  # 開発版の判定は、番号の印を持つ commit だけを見る。基の番号は 0.3.9 に付け替える
  git -C "$t" tag -d build-0.4.0 >/dev/null; git -C "$t" tag build-0.3.9 "$base"
  git -C "$t" tag nightly "$base"; echo "== Scenario: nightly が最新の番号の commit を指していれば、開発版を作らない"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on GITHUB_EVENT_NAME=workflow_dispatch tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out"
  # 注釈つきの印でも commit で比べる（final review R18）
  git -C "$t" tag -d nightly >/dev/null; git -C "$t" tag -a -m nightly nightly "$base"
  out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on GITHUB_EVENT_NAME=workflow_dispatch tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out" || return 1
  git -C "$t" tag -d nightly >/dev/null; git -C "$t" tag nightly "$base"
  git -C "$t" checkout -q main; echo y > "$t/y"; git -C "$t" add y; git -C "$t" commit -q -m numbered; git -C "$t" tag build-0.4.0
  numbered=$(git -C "$t" rev-parse HEAD)
  echo w > "$t/w"; git -C "$t" add w; git -C "$t" commit -q -m no-number-after
  echo "== Scenario: nightly より新しい番号があれば、その番号の commit で開発版を作り直す"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on ASHIATO_NOW=2026-11-01T15:00:00Z tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"; grep -q 'version=0.4.0-dev.20261102' <<<"$out"; grep -qx "commit=$numbered" <<<"$out"
  echo "== Scenario: 予定の実行が落ちた次の回に、同じ変更を拾う"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on GITHUB_EVENT_NAME=schedule tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"
  git -C "$t" tag -d nightly >/dev/null; echo "== Scenario: 印 nightly がまだ無ければ、作り直す"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"
  echo "== Scenario: 公開の設定が入っていなければ、予定の実行は組まずに終える"; out=$(cd "$t" && unset ASHIATO_NIGHTLY_PUBLISH && GITHUB_EVENT_NAME=schedule tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out"; grep -q 'publish=no' <<<"$out"
  echo "== Scenario: 公開の設定が入っていなければ、手で起こした作り直しも公開しない"; out=$(cd "$t" && unset ASHIATO_NIGHTLY_PUBLISH && GITHUB_EVENT_NAME=workflow_dispatch tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"; grep -q 'publish=no' <<<"$out"
  git clone -q "file://$t" "$work/no-number"; git -C "$work/no-number" tag -l 'build-*' | xargs git -C "$work/no-number" tag -d >/dev/null
  echo "== Scenario: 番号がまだ無ければ、開発版を組まない"; out=$(cd "$work/no-number" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out"; grep -q '番号がまだ無い' <<<"$out"
  git clone -q --depth=1 "file://$t" "$work/nightly-shallow"; echo "== Scenario: 浅い木では、開発版の作り直しの判定は通らない"
  if out=$(cd "$work/nightly-shallow" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan 2>&1); then return 1; fi
  grep -q '履歴が浅い' <<<"$out" || return 1
  ! grep -q '^build=' <<<"$out" || return 1
}

case "${1:-}" in
  source) [[ $# -le 2 ]] || usage; source_version "${2:-}" ;;
  major) [[ $# -le 2 ]] || usage; major_file "${2:-}" ;;
  check) [[ $# == 2 ]] || usage; check_version "$2" ;;
  code) [[ $# == 2 ]] || usage; version_code "$2" ;;
  order) order_versions ;;
  plan) [[ $# == 2 ]] || usage; plan "$2" ;;
  nightly-plan) nightly_plan ;;
  number) [[ $# == 2 ]] || usage; number "$2" ;;
  --self-test) self_test ;;
  *) usage ;;
esac
