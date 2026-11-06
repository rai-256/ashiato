#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
lock_file="${PGSQL_LOCK_FILE:-$root/tools/release/pgsql.lock}"
die() { echo "NG: $*" >&2; exit 1; }
lock_value() {
  local key="$1"
  awk -F= -v key="$key" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "$lock_file"
}
csv_values() { tr ',' '\n' <<<"$1"; }
usage() { echo "使い方: $0 fetch <置き場> | slim <展開した置き場> <出力> | --self-test" >&2; exit 2; }

fetch() {
  local place="$1" version lock_url url expected archive tmp actual
  version="$(lock_value version)"
  lock_url="$(lock_value url)"
  url="${PGSQL_URL:-$lock_url}"
  expected="$(lock_value sha256)"
  [[ "$version" =~ ^17\.[0-9]+-[0-9]+$ ]] || die 'lock の版または URL が不正'
  [[ "$lock_url" =~ /postgresql-${version}-windows-x64-binaries\.zip$ ]] || die 'lock の版または URL が不正'
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || die 'lock の SHA-256 が不正'
  mkdir -p "$place"
  archive="$place/postgresql-$version-windows-x64-binaries.zip"
  if [[ -f "$archive" ]]; then
    actual="$(sha256sum "$archive" | awk '{print $1}')"
    if [[ "$actual" == "$expected" ]]; then
      echo "$archive"
      return
    fi
    rm -f "$archive"
  fi
  tmp="$(mktemp "${TMPDIR:-/tmp}/pgsql-fetch.XXXXXX")"
  trap 'rm -f "$tmp"' RETURN
  if ! curl --fail --location --retry 2 --silent --show-error -o "$tmp" "$url"; then
    rm -f "$tmp"
    die 'PostgreSQL の取得に失敗'
  fi
  actual="$(sha256sum "$tmp" | awk '{print $1}')"
  if [[ "$actual" != "$expected" ]]; then
    rm -f "$tmp"
    die "指紋が違う: $url"
  fi
  mv "$tmp" "$archive"
  trap - RETURN
  echo "$archive"
}

slim() {
  local source="$1" output="$2" base name license license_destination
  [[ -d "$source" ]] || die "展開した置き場が無い: $source"
  [[ -d "$source/pgsql" ]] && base="$source/pgsql" || base="$source"
  [[ -d "$base/bin" && -d "$base/lib" && -d "$base/share" ]] || die "PostgreSQL の bin/lib/share が無い: $base"
  rm -rf "$output"
  mkdir -p "$output/bin" "$output/lib" "$output/share" "$output/LICENSES"
  while IFS= read -r name; do
    [[ -n "$name" ]] || continue
    [[ -f "$base/bin/$name" ]] || die "必要な実行ファイルが無い: $name"
    cp -p "$base/bin/$name" "$output/bin/"
  done < <(csv_values "$(lock_value executables)")
  while IFS= read -r name; do
    [[ -n "$name" ]] || continue
    [[ -f "$base/bin/$name" ]] || die "必要な DLL が無い: $name"
    cp -p "$base/bin/$name" "$output/bin/"
  done < <(csv_values "$(lock_value dlls)")
  cp -a "$base/lib/." "$output/lib/"
  cp -a "$base/share/." "$output/share/"
  license="$(lock_value license)"
  license_destination="$(lock_value license_destination)"
  [[ -f "$source/$license" ]] && cp -p "$source/$license" "$output/$license_destination" ||
    [[ -f "$base/${license#pgsql/}" ]] && cp -p "$base/${license#pgsql/}" "$output/$license_destination" ||
    die "許諾の文が無い: $license"
}

self_test() {
  local work fake_root zip valid_lock bad_lock cached archive extracted output
  work="$(mktemp -d "${TMPDIR:-/tmp}/pgsql-self-test.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake_root="$work/fake/pgsql"
  mkdir -p "$fake_root/bin" "$fake_root/lib" "$fake_root/share" "$fake_root/doc/src/sgml/html" \
    "$fake_root/pgAdmin 4" "$fake_root/StackBuilder" "$fake_root/include" "$fake_root/doc"
  while IFS= read -r name; do : >"$fake_root/bin/$name"; done < <(csv_values "$(lock_value executables),$(lock_value dlls)")
  : >"$fake_root/doc/src/sgml/html/legalnotice.html"
  : >"$fake_root/lib/needed.dll"
  : >"$fake_root/share/postgres.bki"
  (cd "$work/fake" && /usr/bin/zip -qr "$work/fake.zip" pgsql)
  valid_lock="$work/valid.lock"
  cp "$lock_file" "$valid_lock"
  sed -i "s#^sha256=.*#sha256=$(sha256sum "$work/fake.zip" | cut -d' ' -f1)#" "$valid_lock"
  cached="$work/cached"
  archive="$cached/postgresql-$(awk -F= '$1 == "version" { print $2; exit }' "$valid_lock")-windows-x64-binaries.zip"
  mkdir -p "$cached"
  printf 'stale cache' >"$archive"
  echo '== Scenario: 壊れた PostgreSQL のキャッシュは捨てて取り直す'
  PGSQL_LOCK_FILE="$valid_lock" PGSQL_URL="file://$work/fake.zip" "$0" fetch "$cached" >/dev/null
  [[ "$(sha256sum "$archive" | cut -d' ' -f1)" == "$(awk -F= '$1 == "sha256" { print $2; exit }' "$valid_lock")" ]] || die '壊れた cache を正しい zip で置き換えられない'
  alternate_lock="$work/alternate.lock"
  sed -e 's/^version=.*/version=17.7-1/' \
    -e 's#^url=.*#url=https://get.enterprisedb.com/postgresql/postgresql-17.7-1-windows-x64-binaries.zip#' \
    "$valid_lock" >"$alternate_lock"
  PGSQL_LOCK_FILE="$alternate_lock" PGSQL_URL="file://$work/fake.zip" "$0" fetch "$work/alternate-cache" >/dev/null
  bad_lock="$work/bad.lock"
  sed 's/^sha256=.*/sha256=0000000000000000000000000000000000000000000000000000000000000000/' "$valid_lock" >"$bad_lock"
  echo "== Scenario: 同梱する PostgreSQL の指紋が違えば組まない"
  if PGSQL_LOCK_FILE="$bad_lock" PGSQL_URL="file://$work/fake.zip" "$0" fetch "$work/cache" >"$work/fetch.out" 2>"$work/fetch.err"; then
    die '偽の指紋を受け入れた'
  fi
  grep -q '指紋が違う' "$work/fetch.err" || die '指紋不一致の理由が無い'
  extracted="$work/extracted"
  mkdir -p "$extracted"
  /usr/bin/unzip -q "$work/fake.zip" -d "$extracted"
  output="$work/slim"
  PGSQL_LOCK_FILE="$valid_lock" "$0" slim "$extracted" "$output"
  [[ -f "$output/bin/postgres.exe" && -f "$output/lib/needed.dll" && -f "$output/share/postgres.bki" ]] || die '必要な内容が無い'
  [[ ! -e "$output/pgAdmin 4" && ! -e "$output/StackBuilder" && ! -e "$output/include" && ! -e "$output/doc" ]] || die '不要な内容を切り出した'
  echo 'OK: 指紋検査と slim の自己テストに成功'
}

case "${1:-}" in
  --self-test) self_test ;;
  fetch) [[ $# == 2 ]] || usage; fetch "$2" ;;
  slim) [[ $# == 3 ]] || usage; slim "$2" "$3" ;;
  *) usage ;;
esac
