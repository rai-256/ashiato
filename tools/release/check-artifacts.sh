#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }

apk_version() {
  local apk="$1" dump aapt_cmd
  [ -f "$apk" ] || die "APK が無い: $apk"
  if command -v aapt2 >/dev/null 2>&1; then
    aapt_cmd="$(command -v aapt2)"
  elif command -v aapt >/dev/null 2>&1; then
    aapt_cmd="$(command -v aapt)"
  else
    aapt_cmd="$(find "${ANDROID_HOME:-/nonexistent}" -type f \( -name aapt2 -o -name aapt \) -perm -111 2>/dev/null | sort -V | tail -n1)"
  fi
  [ -n "$aapt_cmd" ] || die "aapt が無い"
  dump="$($aapt_cmd dump badging "$apk")"
  local line version_code version_name expected_code
  line="$(grep -Eo "versionCode='[0-9]+' versionName='[^']+'" <<<"$dump" | head -n1)"
  [ -n "$line" ] || die "APK の版番号が読めない: $apk"
  version_code="${line#versionCode=\'}"; version_code="${version_code%%\'*}"
  version_name="${line#*versionName=\'}"; version_name="${version_name%\'}"
  expected_code="$("$root/tools/release/version.sh" code "$version_name")" || die "APK の versionName が不正: $version_name"
  echo "== Scenario: 組み立てた APK の versionCode は、渡した版番号から計算した値"
  [ "$version_code" = "$expected_code" ] || die "APK の versionCode が版番号から計算した値と違う: $line（期待 $expected_code）"
  echo "OK: $line"
}

range_self_test() {
  local work clone
  work="$(mktemp -d "${TMPDIR:-/tmp}/gradle-range.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  clone="$work/clone"
  git clone -q --local --no-hardlinks "$root" "$clone"
  echo "== Scenario: 桁を超える版番号では、携帯の組み立てが失敗する"
  sed -i 's/^version = "[^"]*"/version = "0.100.0"/' "$clone/Cargo.toml"
  local output
  if output=$(cd "$clone/collector-android" && ./gradlew --no-build-cache --no-configuration-cache -q :app:assembleDebug 2>&1); then
    die "桁を超えた版番号の検査が成功した"
  fi
  grep -Fq 'MINOR が上限を超えた' <<<"$output" || die "Gradle が MINOR 超過以外の理由で失敗した: $output"
  echo "OK: 桁を超えた版番号を Gradle の版検査が拒否した"
}

case "${1:-}" in
  apk-version) shift; [ "$#" -ge 1 ] || die "APK が無い"; for apk in "$@"; do apk_version "$apk"; done ;;
  --self-test-gradle-range) range_self_test ;;
  *) echo "使い方: $0 apk-version <apk...>|--self-test-gradle-range" >&2; exit 2 ;;
esac
