#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

die() { echo "NG: $*" >&2; exit 1; }
usage() {
  echo "使い方: $0 source|check <版番号>|code <版番号>|order|plan <印>|nightly-plan|--self-test" >&2
  exit 2
}

source_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/{s/^version *= *"\([^"]*\)".*/\1/p;}' "$ROOT/Cargo.toml" | head -n1
}

parse_version() {
  local value="$1"
  if [[ "$value" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-beta\.([1-9][0-9]?)|-dev\.[0-9]{8})?$ ]]; then
    MAJOR=${BASH_REMATCH[1]}; MINOR=${BASH_REMATCH[2]}; PATCH=${BASH_REMATCH[3]}
    SUFFIX=${BASH_REMATCH[4]}
    BETA=${BASH_REMATCH[5]}
    return 0
  fi
  return 1
}

version_code() {
  local value="$1"
  parse_version "$value" || die "版番号の形が違う: $value"
  (( MINOR <= 99 )) || die "MINOR が上限を超えた: $MINOR"
  (( PATCH <= 99 )) || die "PATCH が上限を超えた: $PATCH"
  local tail=99
  [[ "$SUFFIX" == -dev.* ]] && tail=0
  [[ "$SUFFIX" == -beta.* ]] && tail=$BETA
  (( tail <= 99 )) || die "beta 番号が上限を超えた: $tail"
  echo $((MAJOR * 1000000 + MINOR * 10000 + PATCH * 100 + tail))
}

check_version() {
  local value="$1" expected value_major value_minor value_patch
  parse_version "$value" || die "版番号の形が違う: $value"
  value_major=$MAJOR; value_minor=$MINOR; value_patch=$PATCH
  expected="$(source_version)"
  parse_version "$expected" || die "Cargo.toml の出所の版番号が不正: $expected"
  [[ "$value_major.$value_minor.$value_patch" == "$MAJOR.$MINOR.$PATCH" ]] ||
    die "版番号の出所が一致しない: $value（出所 $expected）"
  version_code "$value" >/dev/null
  echo "$value"
}

version_sort_key() {
  local value=${1#v}
  parse_version "$value" || return 1
  local kind=2 number=99
  [[ "$SUFFIX" == -beta.* ]] && { kind=1; number=$BETA; }
  [[ "$SUFFIX" == -dev.* ]] && { kind=0; number=0; }
  printf '%09d.%09d.%09d.%03d.%s\n' "$MAJOR" "$MINOR" "$PATCH" "$kind" "$number"
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
  [[ "$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)" != true ]] || die "履歴が浅い"
}

main_ref() {
  git rev-parse --verify refs/remotes/origin/main 2>/dev/null || git rev-parse --verify refs/heads/main 2>/dev/null || git rev-parse HEAD
}

plan() {
  local tag=${1:-} commit main version expected base branch
  [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.[1-9][0-9]?)?$ ]] || die "印の形が違う: $tag"
  require_full_history
  commit=$(git rev-parse --verify "refs/tags/$tag^{commit}" 2>/dev/null) || die "印が無い: $tag"
  version=${tag#v}
  expected=$(source_version)
  check_version "$version" >/dev/null
  main=$(main_ref)
  if ! git merge-base --is-ancestor "$commit" "$main"; then
    branch=$(git for-each-ref --format='%(refname:short)' --contains "$commit" 'refs/heads/release/*' | head -n1 || true)
    if [[ "$branch" != "release/${version%%.*}.$(printf '%s' "${version#*.}" | cut -d. -f1)" ]]; then
      die "印の commit が main の祖先でない"
    fi
    [[ "$version" != *-beta.* ]] || die "ベータ版の修正の枝は許さない"
    base=$(git describe --tags --abbrev=0 --match "v${version%%.*}.$(printf '%s' "${version#*.}" | cut -d. -f1).*" "${commit}^" 2>/dev/null || true)
    [[ -n "$base" ]] || die "修正の枝の元の印が無い"
    git diff --quiet "$base" "$commit" -- migrations/ORDER docs/contract/envelope.json || die "修正の枝で移行または封筒が変わった"
  fi
  local channel=release prerelease=false
  [[ "$version" == *-beta.* ]] && { channel=beta; prerelease=true; }
  echo "channel=$channel"
  echo "version=$version"
  echo "version_code=$(version_code "$version")"
  echo "prerelease=$prerelease"
}

nightly_plan() {
  local main nightly source tag event now date
  require_full_history
  main=$(main_ref)
  source=$(source_version)
  event=${GITHUB_EVENT_NAME:-schedule}
  now=${ASHIATO_NOW:-}
  if [[ -n "$now" ]]; then date=$(TZ=Asia/Tokyo date -d "$now" +%Y%m%d); else date=$(TZ=Asia/Tokyo date +%Y%m%d); fi
  if tag=$(git rev-parse --verify refs/tags/nightly 2>/dev/null); then
    nightly=$tag
  else
    nightly=
  fi
  if tag=$(git rev-parse --verify "refs/tags/v$source^{commit}" 2>/dev/null) && git merge-base --is-ancestor "$tag" "$main"; then
    echo "build=no"
    echo "publish=no"
    echo "version=$source-dev.$date"
    echo "出所の版を上げる必要がある"
    return 0
  fi
  if [[ -n "$nightly" && "$nightly" == "$main" ]]; then
    echo "build=no"
  elif [[ "$event" == schedule && "${ASHIATO_NIGHTLY_PUBLISH:-}" != on ]]; then
    echo "build=no"
  else
    echo "build=yes"
  fi
  if [[ "${ASHIATO_NIGHTLY_PUBLISH:-}" == on ]]; then echo "publish=yes"; else echo "publish=no"; fi
  echo "version=$source-dev.$date"
}

self_test() {
  local table="$ROOT/tools/release/version-table.tsv" line version expected
  while IFS=$'\t' read -r version expected; do
    [[ -z "$version" || "$version" == \#* ]] && continue
    [[ "$(version_code "$version")" == "$expected" ]] || die "version-table.tsv: $version"
  done < "$table"
  echo "== Scenario: versionCode は版番号から決まり、ベータ < 正式 < 次の版の順になる"
  [[ "$(version_code 0.3.0-beta.2)" == 30002 && "$(version_code 0.3.0)" == 30099 && "$(version_code 0.3.1)" == 30199 && "$(version_code 1.0.0)" == 1000099 && "$(version_code 0.4.0-dev.20261101)" == 40000 ]] || return 1
  echo "== Scenario: 同じ版の開発版は、毎日同じ versionCode になる"
  [[ "$(version_code 0.4.0-dev.20261101)" == "$(version_code 0.4.0-dev.20261102)" ]] || return 1
  echo "== Scenario: 桁を超える版番号では、versionCode の計算が失敗する"
  ! (version_code 0.100.0 >/dev/null 2>&1) && ! (version_code 0.1.100 >/dev/null 2>&1) || return 1

  local work t base out rc
  work=$(mktemp -d "${TMPDIR:-/tmp}/version-self-test.XXXXXX")
  trap 'rm -rf "$work"' RETURN
  t="$work/tree"; mkdir -p "$t/tools/release" "$t/migrations" "$t/docs/contract"
  cp "$ROOT/tools/release/version.sh" "$t/tools/release/version.sh"
  cp "$ROOT/tools/release/version-table.tsv" "$t/tools/release/version-table.tsv"
  printf '[workspace.package]\nversion = "0.4.0"\n' > "$t/Cargo.toml"
  : > "$t/migrations/ORDER"; echo '{}' > "$t/docs/contract/envelope.json"
  git -C "$t" init -q; git -C "$t" config user.email t@example.com; git -C "$t" config user.name t
  git -C "$t" add .; git -C "$t" commit -q -m base; base=$(git -C "$t" rev-parse HEAD); git -C "$t" branch main
  git -C "$t" tag v0.4.0
  echo "== Scenario: 正式版の印は、通常のリリースと判定される"; out=$(cd "$t" && tools/release/version.sh plan v0.4.0); grep -q 'channel=release' <<<"$out"; grep -q 'version=0.4.0' <<<"$out"
  echo "== Scenario: ベータ版の印で、先行公開のリリースになる"; git -C "$t" tag v0.4.0-beta.2; out=$(cd "$t" && tools/release/version.sh plan v0.4.0-beta.2); grep -q 'channel=beta' <<<"$out"
  echo "== Scenario: 印の版と出所の版が違えば公開しない"; git -C "$t" tag v0.5.0-beta.1; ! (cd "$t" && tools/release/version.sh plan v0.5.0-beta.1 >/dev/null 2>&1)
  echo "== Scenario: main の祖先でない commit の印は公開しない"; git -C "$t" checkout -qb other; echo x > "$t/x"; git -C "$t" add x; git -C "$t" commit -q -m other; git -C "$t" tag v0.4.0-beta.3; ! (cd "$t" && tools/release/version.sh plan v0.4.0-beta.3 2>&1 | grep -q '祖先でない')
  echo "== Scenario: 決めた形でない印は、どのチャンネルにも配らない"; for version in v0.4.0-rc.1 v0.4 release-0.4.0; do ! (cd "$t" && tools/release/version.sh plan "$version" >/dev/null 2>&1); done
  git -C "$t" checkout -q main; git clone -q --depth=1 "file://$t" "$work/shallow"; echo "== Scenario: 浅い木では、印の判定は通らない"; ! (cd "$work/shallow" && tools/release/version.sh plan v0.4.0 2>&1 | grep -q '履歴が浅い')

  git -C "$t" tag -d v0.4.0 >/dev/null
  git -C "$t" tag nightly; echo "== Scenario: nightly と main の HEAD が同じ日は、開発版を作らない"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on GITHUB_EVENT_NAME=workflow_dispatch tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out"
  git -C "$t" checkout -q main; echo y > "$t/y"; git -C "$t" add y; git -C "$t" commit -q -m advance; echo "== Scenario: main が進んだ日は、開発版を作り直す"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on ASHIATO_NOW=2026-11-01T15:00:00Z tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"; grep -q 'version=0.4.0-dev.20261102' <<<"$out"
  echo "== Scenario: 予定の実行が落ちた次の回に、同じ変更を拾う"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on GITHUB_EVENT_NAME=schedule tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"
  git -C "$t" tag -d nightly >/dev/null; echo "== Scenario: 印 nightly がまだ無ければ、作り直す"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"
  echo "== Scenario: 公開の設定が入っていなければ、予定の実行は組まずに終える"; out=$(cd "$t" && unset ASHIATO_NIGHTLY_PUBLISH && GITHUB_EVENT_NAME=schedule tools/release/version.sh nightly-plan); grep -q 'build=no' <<<"$out"; grep -q 'publish=no' <<<"$out"
  echo "== Scenario: 公開の設定が入っていなければ、手で起こした作り直しも公開しない"; out=$(cd "$t" && unset ASHIATO_NIGHTLY_PUBLISH && GITHUB_EVENT_NAME=workflow_dispatch tools/release/version.sh nightly-plan); grep -q 'build=yes' <<<"$out"; grep -q 'publish=no' <<<"$out"
  git -C "$t" tag v0.4.0
  echo "== Scenario: 出所の版の正式版を出した後は、出所の版を上げるまで開発版を組まない"; out=$(cd "$t" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan); grep -q '出所の版を上げる必要' <<<"$out"
  git clone -q --depth=1 "file://$t" "$work/nightly-shallow"; echo "== Scenario: 浅い木では、開発版の作り直しの判定は通らない"; ! (cd "$work/nightly-shallow" && ASHIATO_NIGHTLY_PUBLISH=on tools/release/version.sh nightly-plan 2>&1 | grep -q '履歴が浅い')
}

case "${1:-}" in
  source) source_version ;;
  check) [[ $# == 2 ]] || usage; check_version "$2" ;;
  code) [[ $# == 2 ]] || usage; version_code "$2" ;;
  order) order_versions ;;
  plan) [[ $# == 2 ]] || usage; plan "$2" ;;
  nightly-plan) nightly_plan ;;
  --self-test) self_test ;;
  *) usage ;;
esac
