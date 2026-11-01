#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }

check_tree() {
  local tree="$1" cargo gradle package
  cargo="$("$root/tools/release/version.sh" source "$tree/Cargo.toml")"
  if [[ ! "$cargo" =~ ^0\.[0-9]+\.[0-9]+$ ]]; then
    echo "NG: 出所の版番号が 0.MAJOR.PATCH の形でない: ${cargo:-<なし>}" >&2
    return 1
  fi
  if grep -nE '^[[:space:]]*versionName[[:space:]]*=[[:space:]]*"|^[[:space:]]*versionCode[[:space:]]*=[[:space:]]*[0-9]+' "$tree/collector-android/app/build.gradle.kts"; then
    echo "NG: 携帯の組み立てに手書きの版番号がある" >&2
    return 1
  fi
  if grep -nE '"version"[[:space:]]*:' "$tree/web/package.json"; then
    echo "NG: 画面の package.json に手書きの版番号がある" >&2
    return 1
  fi
}

self_test() {
  local work tree
  work="$(mktemp -d "${TMPDIR:-/tmp}/check-version.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  tree="$work/tree"
  mkdir -p "$tree/collector-android/app" "$tree/web"
  printf '[workspace.package]\nversion = "0.4.0"\n' > "$tree/Cargo.toml"
  printf 'android {\n  versionCode = resolvedVersionCode\n  versionName = resolvedBuildVersion\n}\n' > "$tree/collector-android/app/build.gradle.kts"
  printf '{"name":"ashiato-web","private":true}\n' > "$tree/web/package.json"

  echo "== Scenario: 携帯の組み立ての設定に版番号を手で書くと、版の検査が落ちる"
  sed -i 's/versionCode = resolvedVersionCode/versionName = "0.1.0"/' "$tree/collector-android/app/build.gradle.kts"
  if check_tree "$tree" >/dev/null 2>&1; then return 1; fi
  sed -i '/versionName = "0.1.0"/d' "$tree/collector-android/app/build.gradle.kts"

  echo "== Scenario: 画面の package.json に版番号を書くと、版の検査が落ちる"
  sed -i 's/"private":true/"private":true,"version":"0.1.0"/' "$tree/web/package.json"
  if check_tree "$tree" >/dev/null 2>&1; then return 1; fi
  sed -i 's/,"version":"0.1.0"//' "$tree/web/package.json"

  echo "== Scenario: 出所の版の MAJOR が 0 でないと、版の検査が落ちる"
  sed -i 's/version = "0.4.0"/version = "1.0.0"/' "$tree/Cargo.toml"
  if check_tree "$tree" >/dev/null 2>&1; then return 1; fi

  echo "== Scenario: 出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる"
  sed -i 's/version = "1.0.0"/version = "0.4.0-beta.1"/' "$tree/Cargo.toml"
  if check_tree "$tree" >/dev/null 2>&1; then return 1; fi
}

case "${1:-}" in
  --self-test) self_test ;;
  "") check_tree "$root" ;;
  *) echo "使い方: $0 [--self-test]" >&2; exit 2 ;;
esac
