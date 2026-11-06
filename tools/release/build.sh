#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
. "$root/tools/release/android-tools.sh"
usage() { echo "使い方: $0 <server-linux|server-windows|collector-windows|apk> <出力の置き場>|--self-test" >&2; exit 2; }

version=""
out=""
web_dist=""
renamed_dir=""
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
  [[ -z "$renamed_dir" ]] || rm -rf "$renamed_dir"
  renamed_dir=""
  [[ -z "${TMPDIR:-}" ]] || [[ ! -d "$TMPDIR/node-compile-cache" ]] || rm -rf "$TMPDIR/node-compile-cache"
}

cleanup_license_bundle() {
  [[ -z "$license_bundle" ]] || rm -rf "$license_bundle"
  license_bundle=""
}

copy_canonical_license() {
  local repo="$1" destination="$2"
  # Windows の Git checkout は core.autocrlf で作業木を CRLF にし得る。
  # 配布物の照合元は Linux の checkout なので、HEAD の blob をそのまま使う。
  git -C "$repo" show HEAD:LICENSE >"$destination" || die 'Git の LICENSE blob を読めない'
}

make_license_bundle() {
  local kind="$1" target="${2:-}"
  cleanup_license_bundle
  license_bundle="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-licenses.XXXXXX")"
  mkdir -p "$license_bundle/licenses"
  copy_canonical_license "$root" "$license_bundle/licenses/LICENSE"
  local -a notice_args=()
  [[ -z "$target" || "$target" == native ]] || notice_args+=(--target "$target")
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
  # 試験の zip（package.sh windows --test-build）のサーバだけは、試験の差し込み（ASHIATO_SERVICE_TEST_*）を
  # 読む試験の組み立てにする（ST55 D24）。それ以外は今までどおり配る組み立て
  local server_release=1
  [[ "${ASHIATO_PACKAGE_TEST_BUILD:-}" == 1 ]] && server_release=0
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE="$server_release" ASHIATO_WEB_DIST="$web_dist" \
    cargo build --release -p ashiato-server --bin ashiato-server "${cargo_args[@]}")
  mkdir -p "$out"
  local target_dir="${CARGO_TARGET_DIR:-$root/target}"
  local target_dir_name="${target:+$target/}"
  [[ "$target" == native ]] && target_dir_name=""
  local src="$target_dir/${target_dir_name}release/$binary"
  [[ -f "$src" ]] || die "サーバの実行ファイルが無い: $src"
  make_license_bundle server "$target"
  if [[ "$target" == *windows* || "$binary" == *.exe ]]; then
    # インストーラも、同梱するサーバと同じ組み立て（配る / 試験）にする。呼び手の環境の ASHIATO_RELEASE を
    # 読むと、配るサーバと試験の差し込みを持つインストーラが混ざる（review R55）
    build_installer "$target" "$server_release"
    build_postgres_bundle
    local stage="$out/.ashiato-windows-stage"
    rm -rf "$stage"; mkdir -p "$stage/pgsql"
    cp "$src" "$stage/ashiato-server.exe"
    cp "$installer_src" "$stage/ashiato-setup.exe"
    # 中身を写す（`cp -a <dir> <既にある dir>` は中に <dir> ごと写し、pgsql/.pgsql-slim/bin になる。review R49）
    cp -a "$pgsql_dir/." "$stage/pgsql/"
    [[ -f "$stage/pgsql/bin/postgres.exe" ]] || die "Windows の zip の形が違う: pgsql/bin/postgres.exe が無い"
    cp "$root/tools/db-roles.sql" "$stage/"
    # 許諾の 3 点（ST53）。依存の一覧は notices.sh の全体の一覧に、同梱した PostgreSQL と DLL の節を足す
    cp "$license_bundle/licenses/LICENSE" "$license_bundle/licenses/SOURCE.txt" "$stage/"
    { cat "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt"; printf '\n'; windows_notices "$pgsql_dir"; } >"$stage/THIRD-PARTY-NOTICES.txt"
    archive_directory "$out/ashiato-server-$version-windows-x86_64.zip" "$stage"
    rm -rf "$stage" "$out/.pgsql-slim" "$out/.pgsql-cache" "$out/.pgsql-extracted"
  else
    tar -czf "$out/ashiato-server-$version-linux-x86_64.tar.gz" -C "$(dirname "$src")" "$binary" -C "$license_bundle/licenses" LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  fi
  cleanup_license_bundle
  cleanup_build_tmp
}

# サーバの Windows の zip に同梱する部品の許諾（ST55 D13。check-artifacts.sh が PostgreSQL の見出しを見る）。
# 全体の一覧（FR-127）は ST53 の notices.sh が持つ。ここは同梱した PostgreSQL と DLL の節だけを書く
windows_notices() {
  local pgsql="$1"
  printf '%s\n\n' 'THIRD-PARTY NOTICES (ashiato-server for Windows)'
  printf '%s\n' '== PostgreSQL' \
    "version: $(sed -n 's/^version=//p' "$root/tools/release/pgsql.lock")" \
    'location: pgsql\ (full text: pgsql\LICENSES\postgresql-legalnotice.html)' ''
  sed -e 's/<[^>]*>//g' "$pgsql/LICENSES/postgresql-legalnotice.html" | sed -e '/^[[:space:]]*$/d'
  printf '\n%s\n' '== PostgreSQL bundled DLLs'
  sed -n 's/^dlls=//p' "$root/tools/release/pgsql.lock" | tr ',' '\n'
  printf '\n%s\n' '== Microsoft Visual C++ Runtime' \
    'files: pgsql\bin\vcruntime140.dll, pgsql\bin\msvcp140.dll' \
    'source: the Visual Studio redistributable folder (VC\Redist\MSVC\<version>\x64\Microsoft.VC14x.CRT)' \
    'terms: Microsoft Visual Studio license terms, Distributable Code (app-local deployment of the Visual C++ runtime)'
}

# 同梱の PostgreSQL（EDB の MSVC の組み立て）の実行ファイルと DLL は VCRUNTIME140.dll（ICU は MSVCP140.dll も）を
# 読む。まっさらな Windows にはこれが無いので、Visual Studio の再頒布の置き場から pgsql\bin に写す
# （D33（仮）・review R28。Rust の 2 つは windows-gnu の組み立てで要らない）。置き場は ASHIATO_VC_CRT_DIR で渡せる
bundle_vc_runtime() {
  local bin="$1" crt="${ASHIATO_VC_CRT_DIR:-}" dll
  if [[ -z "$crt" ]]; then
    local vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" install
    [[ -x "$vswhere" ]] || die "VC の実行時の DLL の置き場が分からない（Visual Studio のある Windows で組むか、ASHIATO_VC_CRT_DIR を渡す）"
    install="$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')"
    [[ -n "$install" ]] || die 'Visual Studio の置き場が分からない'
    crt="$(ls -d "$(cygpath -u "$install")"/VC/Redist/MSVC/*/x64/Microsoft.VC14*.CRT 2>/dev/null | sort -V | tail -n 1 || true)"
  fi
  for dll in vcruntime140.dll msvcp140.dll; do
    [[ -f "$crt/$dll" ]] || die "VC の実行時の DLL が無い: ${crt:-（置き場なし）}/$dll"
    cp "$crt/$dll" "$bin/$dll"
  done
}

installer_src=""
pgsql_dir=""
build_installer() {
  local target="$1" release="$2" binary="ashiato-setup.exe" args=()
  [[ -n "$target" && "$target" != native ]] && args=(--target "$target")
  (cd "$root" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE="$release" cargo build --release -p ashiato-setup --bin ashiato-setup "${args[@]}")
  local target_dir="${CARGO_TARGET_DIR:-$root/target}" name="${target:+$target/}release/$binary"
  [[ "$target" == native ]] && name="release/$binary"
  installer_src="$target_dir/$name"
  [[ -f "$installer_src" ]] || die "インストーラが無い: $installer_src"
}

remove_owned_pgsql_cache() {
  local cache="$1" owned_cache="$2"
  if [[ "$cache" == "$owned_cache" ]]; then
    rm -rf "$cache"
  fi
}

build_postgres_bundle() {
  local cache="${PGSQL_CACHE_DIR:-$out/.pgsql-cache}" extracted="$out/.pgsql-extracted"
  mkdir -p "$cache"
  local archive; archive="$($root/tools/release/pgsql.sh fetch "$cache")"
  rm -rf "$extracted"; mkdir -p "$extracted"
  unzip -q "$archive" -d "$extracted"
  pgsql_dir="$out/.pgsql-slim"
  "$root/tools/release/pgsql.sh" slim "$extracted" "$pgsql_dir"
  rm -rf "$extracted"
  bundle_vc_runtime "$pgsql_dir/bin"
  remove_owned_pgsql_cache "$cache" "$out/.pgsql-cache"
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
  archive_collector "$out/ashiato-collector-windows-$version-x86_64.zip" "$src" "$license_bundle/licenses/LICENSE" "$license_bundle/licenses/SOURCE.txt" "$license_bundle/licenses/THIRD-PARTY-NOTICES.txt"
  cleanup_license_bundle
}

archive_collector() {
  local archive="$1" file="$2" archive_name="ashiato-collector-windows"; shift 2
  [[ "$version" == *-dev.* ]] && archive_name+="-dev"
  local archive_src="$file"
  if [[ "$archive_name" != "ashiato-collector-windows" ]]; then
    # 名前を変えた写しは一時の置き場に作る（repo の根に作らない）。落ちても EXIT の cleanup_build_tmp が消す
    renamed_dir="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-collector-name.XXXXXX")"
    archive_src="$renamed_dir/$archive_name.exe"
    cp "$file" "$archive_src"
  fi
  archive_windows "$archive" "$archive_src" "$@"
  [[ -z "$renamed_dir" ]] || rm -rf "$renamed_dir"
  renamed_dir=""
}

archive_windows() {
  local archive="$1"; shift; local -a files=("$@")
  if command -v zip >/dev/null 2>&1; then
    zip -q -j "$archive" "${files[@]}"
  elif command -v powershell.exe >/dev/null 2>&1; then
    local ps_archive="$archive"; local ps_files=""
    for file in "${files[@]}"; do
      local ps_file="$file"
      command -v cygpath >/dev/null 2>&1 && ps_file="$(cygpath -w "$file")"
      ps_files+="${ps_files:+,}'$ps_file'"
    done
    command -v cygpath >/dev/null 2>&1 && ps_archive="$(cygpath -w "$archive")"
    powershell.exe -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath @($ps_files) -DestinationPath '$ps_archive' -Force"
  elif command -v pwsh >/dev/null 2>&1; then
    local ps_files=""
    for file in "${files[@]}"; do ps_files+="${ps_files:+,}'$file'"; done
    pwsh -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath @($ps_files) -DestinationPath '$archive' -Force"
  else
    die "Windows の zip を作る道具が無い（zip / PowerShell）"
  fi
}

# Windows PowerShell 5.1 の Compress-Archive は zip の中の場所を `\` 区切りで書く。
# unzip が警告で rc=1 を返し、`pgsql/bin/postgres.exe` も見つからなくなるので、
# .NET の ZipFile で entry 名を `/` 区切りにして自分で書く。
zip_directory_command() {
  local directory="$1" archive="$2"
  printf '%s' "\$ErrorActionPreference = 'Stop'; Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem; \$root = (Resolve-Path -LiteralPath '$directory').ProviderPath.TrimEnd('\\', '/'); if (Test-Path -LiteralPath '$archive') { Remove-Item -LiteralPath '$archive' -Force }; \$zip = [System.IO.Compression.ZipFile]::Open('$archive', 'Create'); try { Get-ChildItem -LiteralPath \$root -Recurse -Force -File | ForEach-Object { [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(\$zip, \$_.FullName, \$_.FullName.Substring(\$root.Length + 1).Replace('\\', '/'), 'Optimal') } } finally { \$zip.Dispose() }"
}

archive_directory() {
  local archive="$1" directory="$2"
  if command -v zip >/dev/null 2>&1; then
    (cd "$directory" && zip -q -r "$archive" .)
  elif command -v powershell.exe >/dev/null 2>&1; then
    local ps_directory="$directory" ps_archive="$archive"
    command -v cygpath >/dev/null 2>&1 && ps_directory="$(cygpath -w "$directory")" && ps_archive="$(cygpath -w "$archive")"
    powershell.exe -NoProfile -NonInteractive -Command "$(zip_directory_command "$ps_directory" "$ps_archive")"
  elif command -v pwsh >/dev/null 2>&1; then
    pwsh -NoProfile -NonInteractive -Command "$(zip_directory_command "$directory" "$archive")"
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
  # Kotlin の daemon の置き場は組み立てごとに作り、終わったら消す（固定名で $TMPDIR に残さない）
  local kotlin_daemon_dir; kotlin_daemon_dir="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-kotlin-daemon.XXXXXX")"
  # Kotlin の増分の組み立ては BuildConfig の定数を埋め込んだ古い class を組み直さないことがあり、
  # 前の commit の印が APK に残る（実測 2026-11-02: LocationService.class に前の commit の印）。配る APK は増分にしない
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$version" ASHIATO_RELEASE=1 \
    GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.daemon.storageDir=$kotlin_daemon_dir -Dkotlin.compiler.execution.strategy=in-process" \
    ./gradlew --no-daemon --no-build-cache --no-configuration-cache -Pkotlin.incremental=false -q \
      -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= \
      "-Pashiato.licenseAssets=$license_bundle" :app:assembleRelease) || { rm -rf "$kotlin_daemon_dir"; die "APK の組み立てに失敗した"; }
  rm -rf "$kotlin_daemon_dir"
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
  local work internal_cache external_cache
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-build.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  internal_cache="$work/out/.pgsql-cache"
  external_cache="$work/external-pgsql-cache"
  mkdir -p "$internal_cache" "$external_cache"
  remove_owned_pgsql_cache "$internal_cache" "$internal_cache"
  [[ ! -e "$internal_cache" ]] || die '自分で作った PostgreSQL の cache を消せない'
  remove_owned_pgsql_cache "$external_cache" "$internal_cache"
  [[ -d "$external_cache" ]] || die '外から渡した PostgreSQL の cache を消した'
  # Windows の checkout は LICENSE を CRLF にすることがあるが、配布物には Git の
  # blob と同じバイト列を入れる（package-artifacts は Linux で blob と比べる）。
  local checkout="$work/checkout" expected="$work/expected-LICENSE" actual="$work/actual-LICENSE"
  git clone -q --local --no-hardlinks "$root" "$checkout"
  git -C "$checkout" show HEAD:LICENSE >"$expected"
  sed 's/$/\r/' "$checkout/LICENSE" >"$checkout/LICENSE.crlf"
  mv "$checkout/LICENSE.crlf" "$checkout/LICENSE"
  echo "== Scenario: Windows の checkout でも、配布物の LICENSE は Git の blob と同じ"
  copy_canonical_license "$checkout" "$actual"
  cmp -s "$expected" "$actual" || die 'Windows の checkout で LICENSE のバイト列が変わった'
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

  echo "== Scenario: 開発版の PC の収集アプリの配布物の実行ファイルは、-dev の名前を持つ"
  local fake="$work/ashiato-collector-windows.exe"
  printf 'fake' >"$fake"
  ASHIATO_BUILD_VERSION="0.4.0-dev.20261102" version="0.4.0-dev.20261102" archive_collector "$work/dev.zip" "$fake"
  zipinfo -1 "$work/dev.zip" | grep -Fxq ashiato-collector-windows-dev.exe
  ASHIATO_BUILD_VERSION="0.4.0" version="0.4.0" archive_collector "$work/official.zip" "$work/ashiato-collector-windows.exe"
  zipinfo -1 "$work/official.zip" | grep -Fxq ashiato-collector-windows.exe
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 2 ]] || usage
# 組み立てが途中で落ちても、画面の写しの一時の置き場を残さない（/code-review R37）
trap 'cleanup_license_bundle; cleanup_build_tmp' EXIT
resolve_version
build_one "$1" "$2"
