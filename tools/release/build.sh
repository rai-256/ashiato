#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
. "$root/tools/release/android-tools.sh"
usage() { echo "使い方: $0 <server-linux|server-windows|collector-windows|apk> <出力の置き場>|--self-test" >&2; exit 2; }

version=""
out=""
web_dist=""
license_bundle=""

resolve_version() {
  version="${ASHIATO_BUILD_VERSION:-}"
  if [[ -n "$version" ]]; then
    "$root/tools/release/version.sh" check "$version" >/dev/null
  else
    version="$($root/tools/release/version.sh source)"
  fi
}

build_web() {
  web_dist="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-web.XXXXXX")"
  # 新しい runner に web/node_modules は無い。tools/check-build-stamp.sh と同じ形で依存を入れてから組む（final review R1）
  (cd "$root/web" && npm ci --ignore-scripts >/dev/null && ASHIATO_BUILD_VERSION="$version" npm run build >/dev/null)
  cp -a "$root/web/dist/." "$web_dist/"
}

cleanup_build_tmp() {
  [[ -z "$web_dist" ]] || rm -rf "$web_dist"
  web_dist=""
  [[ -z "${TMPDIR:-}" ]] || [[ ! -d "$TMPDIR/node-compile-cache" ]] || rm -rf "$TMPDIR/node-compile-cache"
}

cleanup_license_bundle() {
  [[ -z "$license_bundle" ]] || rm -rf "$license_bundle"
  license_bundle=""
}

make_license_bundle() {
  local kind="$1" target="${2:-}"
  cleanup_license_bundle
  license_bundle="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-licenses.XXXXXX")"
  mkdir -p "$license_bundle/licenses"
  cp "$root/LICENSE" "$license_bundle/licenses/LICENSE"
  local -a notice_args=()
  [[ -z "$target" ]] || notice_args+=(--target "$target")
  case "$kind" in
    server|collector-windows)
      "$root/tools/release/notices.sh" "$kind" "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt" "${notice_args[@]}"
      ;;
    apk)
      "$root/tools/release/notices.sh" apk "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt"
      ;;
    *) die "許諾の種類が不正: $kind" ;;
  esac
  "$root/tools/release/notices.sh" source "$license_bundle/licenses/SOURCE.txt" "$version" ""
}

build_server() {
  local target="${1:-}" binary="ashiato-server"
  build_web
  if [[ -n "$target" && "$target" != native ]]; then
    cargo_args=(--target "$target")
  else
    cargo_args=()
  fi
  if [[ -n "$target" ]]; then
    binary+=".exe"
  fi
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE=1 ASHIATO_WEB_DIST="$web_dist" \
    cargo build --release -p ashiato-server --bin ashiato-server "${cargo_args[@]}")
  mkdir -p "$out"
  local target_dir="${CARGO_TARGET_DIR:-$root/target}"
  local target_dir_name="${target:+$target/}"
  [[ "$target" == native ]] && target_dir_name=""
  local src="$target_dir/${target_dir_name}release/$binary"
  [[ -f "$src" ]] || die "サーバの実行ファイルが無い: $src"
  make_license_bundle server "$target"
  if [[ "$target" == *windows* || "$binary" == *.exe ]]; then
    archive_windows "$out/ashiato-server-$version-windows-x86_64.zip" "$src" "$license_bundle/licenses/LICENSE" "$license_bundle/licenses/SOURCE.txt" "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt"
  else
    tar -czf "$out/ashiato-server-$version-linux-x86_64.tar.gz" -C "$(dirname "$src")" "$binary" -C "$license_bundle/licenses" LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  fi
  cleanup_license_bundle
  cleanup_build_tmp
}

build_collector() {
  local target="${1:-}" binary="ashiato-collector-windows"
  if [[ -n "$target" && "$target" != native ]]; then
    cargo_args=(--target "$target")
  else
    cargo_args=()
  fi
  [[ -n "$target" ]] && binary+=".exe"
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" cargo build --release -p ashiato-collector-windows "${cargo_args[@]}")
  mkdir -p "$out"
  local target_dir="${CARGO_TARGET_DIR:-$root/target}"
  local target_dir_name="${target:+$target/}"
  [[ "$target" == native ]] && target_dir_name=""
  local src="$target_dir/${target_dir_name}release/$binary"
  [[ -f "$src" ]] || die "PC の収集アプリが無い: $src"
  make_license_bundle collector-windows "$target"
  archive_windows "$out/ashiato-collector-windows-$version-x86_64.zip" "$src" "$license_bundle/licenses/LICENSE" "$license_bundle/licenses/SOURCE.txt" "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt"
  cleanup_license_bundle
}

archive_windows() {
  local archive="$1"; shift; local -a files=("$@")
  if command -v zip >/dev/null 2>&1; then
    (cd "$(dirname "${files[0]}")" && zip -q -j "$archive" "${files[@]##*/}")
  elif command -v powershell.exe >/dev/null 2>&1; then
    local ps_archive="$archive"; local ps_files=""
    for file in "${files[@]}"; do
      local ps_file="$file"
      command -v cygpath >/dev/null 2>&1 && ps_file="$(cygpath -w "$file")"
      ps_files+="'$ps_file',"
    done
    command -v cygpath >/dev/null 2>&1 && ps_archive="$(cygpath -w "$archive")"
    powershell.exe -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath $ps_files -DestinationPath '$ps_archive' -Force"
  elif command -v pwsh >/dev/null 2>&1; then
    local ps_files=""
    for file in "${files[@]}"; do ps_files+="'$file',"; done
    pwsh -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath $ps_files -DestinationPath '$archive' -Force"
  else
    die "Windows の zip を作る道具が無い（zip / PowerShell）"
  fi
}

windows_target() {
  # Windows runner は native cargo が .exe を出す。WSL 等の Linux runner は
  # 呼び出し側が指定した cross target を使う（既定は GNU toolchain）。
  if [[ "${OSTYPE:-}" == msys* || "${OSTYPE:-}" == cygwin* || "${OS:-}" == Windows_NT ]]; then
    printf '%s' native
  else
    printf '%s' "${ASHIATO_WINDOWS_TARGET:-x86_64-pc-windows-gnu}"
  fi
}

build_apk() {
  mkdir -p "$out"
  make_license_bundle apk
  local kotlin_daemon_dir="${TMPDIR:-/tmp}/ashiato-kotlin-daemon"
  mkdir -p "$kotlin_daemon_dir"
  # Kotlin の増分の組み立ては BuildConfig の定数を埋め込んだ古い class を組み直さないことがあり、
  # 前の commit の印が APK に残る（実測 2026-11-02: LocationService.class に前の commit の印）。配る APK は増分にしない
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE=1 \
    GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.daemon.storageDir=$kotlin_daemon_dir" \
    ./gradlew --no-daemon --no-build-cache --no-configuration-cache -Pkotlin.incremental=false -q \
      -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= \
      "-Pashiato.licenseAssets=$license_bundle" :app:assembleRelease)
  local apk="$root/collector-android/app/build/outputs/apk/release/app-release.apk"
  if [[ "${ASHIATO_APK_UNSIGNED:-}" == 1 ]]; then
    apk="$root/collector-android/app/build/outputs/apk/release/app-release-unsigned.apk"
  fi
  [[ -f "$apk" ]] || die "APK が無い: $apk"
  cp "$apk" "$out/ashiato-collector-$version.apk"
  cleanup_license_bundle
}

build_one() {
  local kind="$1"
  out="$2"
  mkdir -p "$out"
  case "$kind" in
    server-linux) build_server "" ;;
    server-windows) build_server "$(windows_target)" ;;
    collector-windows) build_collector "$(windows_target)" ;;
    apk) build_apk ;;
    *) usage ;;
  esac
}

self_test() {
  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-build.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  # 版番号は生きた出所の版から組む（出所の版を上げても壊れない。final review R10）
  local v; v="$("$root/tools/release/version.sh" source)"
  echo "== Scenario: release の組み立ては、画面を取り込んだ Linux サーバと APK を作る"
  ASHIATO_BUILD_VERSION="$v" "$0" server-linux "$work/out"
  ASHIATO_BUILD_VERSION="$v" "$0" apk "$work/out"
  [[ -s "$work/out/ashiato-server-$v-linux-x86_64.tar.gz" ]]
  [[ -s "$work/out/ashiato-collector-$v.apk" ]]
  tar -tzf "$work/out/ashiato-server-$v-linux-x86_64.tar.gz" | grep -Fxq ashiato-server
  local server_text apk_build_config
  server_text="$(tar -xOzf "$work/out/ashiato-server-$v-linux-x86_64.tar.gz" ashiato-server | strings)"
  grep -Fq "$v+" <<<"$server_text"
  apk_build_config="$("$(android_tool apkanalyzer)" dex code --class dev.ashiato.collector.BuildConfig "$work/out/ashiato-collector-$v.apk")"
  grep -Eq '\.field public static final (BASE_URL|API_TOKEN|USER_ID):.* = ""' <<<"$apk_build_config"
  echo "OK: Linux サーバと APK の名前と中身を確認した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 2 ]] || usage
# 組み立てが途中で落ちても、画面の写しの一時の置き場を残さない（/code-review R37）
trap 'cleanup_license_bundle; cleanup_build_tmp' EXIT
resolve_version
build_one "$1" "$2"
