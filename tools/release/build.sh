#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <server-linux|server-windows|collector-windows|apk> <出力の置き場>|--self-test" >&2; exit 2; }

version="${ASHIATO_BUILD_VERSION:-}"
if [[ -n "$version" ]]; then
  "$root/tools/release/version.sh" check "$version" >/dev/null
fi
version="${version:-$($root/tools/release/version.sh source)}"
out=""
web_dist=""

build_web() {
  web_dist="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-web.XXXXXX")"
  (cd "$root/web" && ASHIATO_BUILD_VERSION="$version" npm run build >/dev/null)
  cp -a "$root/web/dist/." "$web_dist/"
}

cleanup_build_tmp() {
  [[ -z "$web_dist" ]] || rm -rf "$web_dist"
  web_dist=""
  [[ -z "${TMPDIR:-}" ]] || [[ ! -d "$TMPDIR/node-compile-cache" ]] || rm -rf "$TMPDIR/node-compile-cache"
}

build_server() {
  local target="${1:-}" binary="ashiato-server"
  build_web
  if [[ -n "$target" ]]; then
    cargo_args=(--target "$target")
    binary+=".exe"
  else
    cargo_args=()
  fi
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE=1 ASHIATO_WEB_DIST="$web_dist" \
    cargo build --release -p ashiato-server --bin ashiato-server "${cargo_args[@]}")
  mkdir -p "$out"
  local target_dir="${CARGO_TARGET_DIR:-$root/target}"
  local src="$target_dir/${target:+$target/}release/$binary"
  [[ -f "$src" ]] || die "サーバの実行ファイルが無い: $src"
  if [[ "$target" == *windows* ]]; then
    (cd "$(dirname "$src")" && zip -q -j "$out/ashiato-server-$version-windows-x86_64.zip" "$binary")
  else
    tar -czf "$out/ashiato-server-$version-linux-x86_64.tar.gz" -C "$(dirname "$src")" "$binary"
  fi
  cleanup_build_tmp
}

build_collector() {
  local target="${1:-}" binary="ashiato-collector-windows"
  if [[ -n "$target" ]]; then
    cargo_args=(--target "$target")
    binary+=".exe"
  else
    cargo_args=()
  fi
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" cargo build --release -p ashiato-collector-windows "${cargo_args[@]}")
  mkdir -p "$out"
  local target_dir="${CARGO_TARGET_DIR:-$root/target}"
  local src="$target_dir/${target:+$target/}release/$binary"
  [[ -f "$src" ]] || die "PC の収集アプリが無い: $src"
  (cd "$(dirname "$src")" && zip -q -j "$out/ashiato-collector-windows-$version-x86_64.zip" "$binary")
}

build_apk() {
  mkdir -p "$out"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE=1 \
    ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q :app:assembleRelease)
  local apk="$root/collector-android/app/build/outputs/apk/release/app-release.apk"
  [[ -f "$apk" ]] || die "APK が無い: $apk"
  cp "$apk" "$out/ashiato-collector-$version.apk"
}

build_one() {
  local kind="$1"
  out="$2"
  mkdir -p "$out"
  case "$kind" in
    server-linux) build_server "" ;;
    server-windows) build_server "x86_64-pc-windows-gnu" ;;
    collector-windows) build_collector "x86_64-pc-windows-gnu" ;;
    apk) build_apk ;;
    *) usage ;;
  esac
}

self_test() {
  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-build.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  echo "== Scenario: release の組み立ては、画面を取り込んだ Linux サーバと APK を作る"
  ASHIATO_BUILD_VERSION="0.1.0" build_one server-linux "$work/out"
  ASHIATO_BUILD_VERSION="0.1.0" build_one apk "$work/out"
  [[ -s "$work/out/ashiato-server-0.1.0-linux-x86_64.tar.gz" ]]
  [[ -s "$work/out/ashiato-collector-0.1.0.apk" ]]
  tar -tzf "$work/out/ashiato-server-0.1.0-linux-x86_64.tar.gz" | grep -Fxq ashiato-server
  echo "OK: Linux サーバと APK の名前と中身を確認した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 2 ]] || usage
build_one "$1" "$2"
