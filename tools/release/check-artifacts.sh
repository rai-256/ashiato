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
  grep -Eq "package: .*versionCode='[0-9]+' .*versionName='[^']+'" <<<"$dump" || die "APK の版番号が読めない: $apk"
  echo "OK: $(grep -Eo "versionCode='[0-9]+' versionName='[^']+'" <<<"$dump" | head -n1)"
}

range_self_test() {
  local work clone
  work="$(mktemp -d "${TMPDIR:-/tmp}/gradle-range.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  clone="$work/clone"
  git clone -q --local --no-hardlinks "$root" "$clone"
  if (cd "$clone/collector-android" && ASHIATO_BUILD_VERSION=0.100.0 ./gradlew --no-build-cache --no-configuration-cache -q :app:assembleDebug); then
    "$root/tools/release/version.sh" code 0.100.0 >/dev/null 2>&1 && die "桁を超えた版番号の検査が成功した"
  fi
  echo "OK: 桁を超えた版番号を Gradle の版検査が拒否した"
}

case "${1:-}" in
  apk-version) shift; [ "$#" -ge 1 ] || die "APK が無い"; for apk in "$@"; do apk_version "$apk"; done ;;
  --self-test-gradle-range) range_self_test ;;
  *) echo "使い方: $0 apk-version <apk...>|--self-test-gradle-range" >&2; exit 2 ;;
esac
