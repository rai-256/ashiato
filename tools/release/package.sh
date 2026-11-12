#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <linux|windows> <置き場>|--self-test" >&2; exit 2; }

build="${ASHIATO_PACKAGE_BUILD:-$root/tools/release/build.sh}"

package() {
  local platform="$1" out="$2"
  mkdir -p "$out"
  # 版番号は build.sh が出所から解決する。ここでは PR と同じ release 経路だけを選ぶ。
  case "$platform" in
    linux)
      ASHIATO_RELEASE=1 env -u ASHIATO_BUILD_VERSION "$build" server-linux "$out"
      ASHIATO_RELEASE=1 env -u ASHIATO_BUILD_VERSION "$build" apk "$out"
      ;;
    windows)
      if [[ "${ASHIATO_PACKAGE_TEST_BUILD:-}" == 1 ]]; then
        env -u ASHIATO_BUILD_VERSION "$build" server-windows "$out"
      else
        ASHIATO_RELEASE=1 env -u ASHIATO_BUILD_VERSION "$build" server-windows "$out"
        ASHIATO_RELEASE=1 env -u ASHIATO_BUILD_VERSION "$build" collector-windows "$out"
      fi
      ;;
    *) usage ;;
  esac
}

self_test() {
  local work fake calls
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-package.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake="$work/build.sh"; calls="$work/calls"
  cat >"$fake" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${ASHIATO_RELEASE:-}" == 1 && -z "${ASHIATO_BUILD_VERSION:-}" ]] || { echo "release/version 環境が違う" >&2; exit 24; }
printf '%s\n' "$*" >> "${PACKAGE_CALLS:?}"
[[ "${PACKAGE_FAIL_KIND:-}" != "$1" ]] || { echo "fake build failure: $1" >&2; exit 23; }
mkdir -p "$2"
touch "$2/$1.ok"
[[ "$1" != server-windows ]] || touch "$2/ashiato-updater.exe"
EOF
  chmod +x "$fake"
  echo "== Scenario: 配る形を壊した PR は、その PR の CI で落ちる"
  PACKAGE_CALLS="$calls" ASHIATO_BUILD_VERSION=wrong ASHIATO_PACKAGE_BUILD="$fake" "$0" linux "$work/success" || die '成功する package を落とした'
  grep -q '^server-linux ' "$calls" || die '成功時の Linux server build が走っていない'
  grep -q '^apk ' "$calls" || die '成功時の APK build が走っていない'
  : >"$calls"
  if PACKAGE_CALLS="$calls" PACKAGE_FAIL_KIND=apk ASHIATO_BUILD_VERSION=wrong \
      ASHIATO_PACKAGE_BUILD="$fake" "$0" linux "$work/fail" >/dev/null 2>&1; then
    die '失敗した build を package が通した'
  fi
  grep -q '^server-linux ' "$calls" || die 'Linux の server build が走っていない'
  grep -q '^apk ' "$calls" || die '失敗した APK build が走っていない'
  [[ "$(wc -l <"$calls")" == 2 ]] || die 'APK の前に別の build が余計に走った'
  # Windows の配る経路も、残った ASHIATO_BUILD_VERSION を持ち込まない（Linux と同じ。review R56）
  : >"$calls"
  PACKAGE_CALLS="$calls" ASHIATO_BUILD_VERSION=wrong ASHIATO_PACKAGE_BUILD="$fake" "$0" windows "$work/windows" ||
    die 'Windows の配る経路が ASHIATO_BUILD_VERSION を持ち込んだ'
  grep -q '^server-windows ' "$calls" && grep -q '^collector-windows ' "$calls" || die 'Windows の build が走っていない'
  [[ -f "$work/windows/ashiato-updater.exe" ]] || die 'Windows の配布物に更新係が無い'
  echo "OK: package の build 失敗を伝播した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
if [[ $# == 3 && "$1" == windows && "$3" == --test-build ]]; then
  ASHIATO_PACKAGE_TEST_BUILD=1 package windows "$2"
elif [[ $# == 2 ]]; then
  package "$1" "$2"
else
  usage
fi
