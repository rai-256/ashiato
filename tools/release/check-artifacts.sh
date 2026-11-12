#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
quiet_build() {
  local log="$1"; shift
  "$@" >"$log" 2>&1 || { tail -n 40 "$log" >&2; return 1; }
}
. "$root/tools/release/android-tools.sh"
usage() { echo "使い方: $0 <置き場> [--channel <release|beta|nightly>] [--allow-debug-key] [--expect-version <版番号>] [--expect-license <ファイル>] | check <置き場> [...] | apk-version [--expect-version <版番号>] <apk...> | apk-label <apk> | apk-manifest <apk> | --self-test | --self-test-android" >&2; exit 2; }

apk_dump() {
  local apk="$1"; [ -f "$apk" ] || die "APK が無い: $apk"
  local aapt_cmd; aapt_cmd="$(android_tool aapt2 || android_tool aapt || true)"; [ -n "$aapt_cmd" ] || die "aapt が無い"
  "$aapt_cmd" dump badging "$apk"
}

apk_label() {
  local line; line="$(apk_dump "$1" | grep -E "^application-label:")" || die "APK のアプリ名が読めない"
  echo "$line"
}

apk_manifest() {
  local badging tree aapt_cmd
  badging="$(apk_dump "$1")"
  aapt_cmd="$(android_tool aapt2 || android_tool aapt || true)"; [ -n "$aapt_cmd" ] || die "aapt が無い"
  tree="$($aapt_cmd dump xmltree "$1" --file AndroidManifest.xml)"
  {
    sed -nE \
    -e "s/^uses-permission: name='([^']+)'.*/permission:\1/p" \
    -e "s/^uses-feature: name='([^']+)'.*/feature:\1/p" \
    <<<"$badging"
    awk '
      /^          E: (service|receiver) / { kind=$2; pending=1; next }
      pending && /android:name\(/ {
        value=$0; sub(/^.*Raw: "/, "", value); sub(/".*$/, "", value); print kind ":" value
        pending=0
      }
      pending && /^          E: / { pending=0 }
    ' <<<"$tree"
  } | sed -E 's/^permission:dev\.ashiato\.collector(\.dev)?\.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION$/permission:APP.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION/' | sort -u
}

self_test_android() {
  local v; v="$("$root/tools/release/version.sh" source)"
  local dev="$v-dev.20261102"
  local work; work="$(mktemp -d "${TMPDIR:-/tmp}/android-self-test.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$dev" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q \
    -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= :app:assembleDebug)
  local apk="$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" dev_apk="$work/ashiato-dev-self-test.apk"
  cp "$apk" "$dev_apk"
  echo "== Scenario: 開発版のアプリの名前は「あしあと。開発版」で、正式版は「あしあと。」のまま"
  grep -Fq "application-label:'あしあと。開発版'" <<<"$(apk_label "$apk")" || die "開発版のアプリ名が違う"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$v" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q \
    -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= :app:assembleDebug)
  local official="$root/collector-android/app/build/outputs/apk/debug/app-debug.apk"
  grep -Fq "application-label:'あしあと。'" <<<"$(apk_label "$official")" || die "正式版のアプリ名が違う"
  echo "== Scenario: 開発版の携帯の収集アプリは、正式版と同じ権限と常駐の口を持つ"
  diff -u <(apk_manifest "$official") <(apk_manifest "$dev_apk")
  echo "OK: Android の開発版・正式版の名前と宣言を検査した"
}

apk_version() {
  local apk="$1" want="${2:-}" dump aapt_cmd
  [ -f "$apk" ] || die "APK が無い: $apk"
  aapt_cmd="$(android_tool aapt2 || android_tool aapt || true)"
  [ -n "$aapt_cmd" ] || die "aapt が無い"
  dump="$($aapt_cmd dump badging "$apk")"
  local line version_code version_name expected_code
  line="$(grep -Eo "versionCode='[0-9]+' versionName='[^']+'" <<<"$dump" | head -n1)"
  [ -n "$line" ] || die "APK の版番号が読めない: $apk"
  version_code="${line#versionCode=\'}"; version_code="${version_code%%\'*}"
  version_name="${line#*versionName=\'}"; version_name="${version_name%\'}"
  expected_code="$("$root/tools/release/version.sh" code "$version_name")" || die "APK の versionName が不正: $version_name"
  echo "== Scenario: 組み立てた APK の versionCode は、渡した版番号から計算した値"
  if [ -n "$want" ]; then
    # 渡した版番号と比べる（APK 自身の versionName から計算するだけでは、渡した値を無視しても通る。code_verify R25）
    [ "$version_name" = "$want" ] || die "APK の versionName が渡した版番号と違う: $line（期待 $want）"
    expected_code="$("$root/tools/release/version.sh" code "$want")"
  fi
  [ "$version_code" = "$expected_code" ] || die "APK の versionCode が版番号から計算した値と違う: $line（期待 $expected_code）"
  echo "OK: $line"
}

artifact_stamp() {
  local artifact="$1" version="$2" text stamp
  case "$artifact" in
    *.apk) text="$(/usr/bin/unzip -p "$artifact" 'classes*.dex' 2>/dev/null | strings || true)" ;;
    *.tar.gz) text="$(tar -xOzf "$artifact" 2>/dev/null | strings || true)" ;;
    *.zip) text="$(/usr/bin/unzip -p "$artifact" 2>/dev/null | strings || true)" ;;
    *) text="$(strings "$artifact" 2>/dev/null || true)" ;;
  esac
  stamp="$(grep -Eo "${version//./\\.}\\+[0-9a-f]{40}(\\.dirty|\\.debug)*" <<<"$text" | sort -u)"
  [[ -n "$stamp" ]] || die "組み立ての印が読めない: $(basename "$artifact")"
  # Rust/Android の最適化物には、依存する compile unit の印が複数残ることがある。
  # dirty/debug の印を優先して 1 つに絞り、異なる dirty/debug 印が複数なら拒否する。
  local selected
  selected="$(grep -E '\.dirty(\.debug)?$' <<<"$stamp" || true)"
  [[ -n "$selected" ]] || selected="$(grep -E '\.debug$' <<<"$stamp" || true)"
  [[ -n "$selected" ]] && stamp="$selected"
  # .dirty / .debug は数より先に理由として示す（増分の debug 組み立てで古い dex の印が残り、2 つになることがある）
  if grep -Fq '.dirty' <<<"$stamp"; then die "$(basename "$artifact") に .dirty の印がある"; fi
  if grep -Fq '.debug' <<<"$stamp"; then die "$(basename "$artifact") に .debug の印がある"; fi
  [[ "$(wc -l <<<"$stamp")" == 1 ]] || die "組み立ての版番号が 1 つでない: $(basename "$artifact")"
  printf '%s' "$stamp"
}

artifact_file() {
  local artifact="$1" path="$2"
  case "$artifact" in
    *.tar.gz) tar -xOzf "$artifact" "$path" 2>/dev/null ;;
    *.zip|*.apk) /usr/bin/unzip -p "$artifact" "$path" 2>/dev/null ;;
    *) return 1 ;;
  esac
}

check_artifact_files() {
  local artifact="$1" name="$2" path license notices source expected_ref expected_commit stamp_commit
  if [[ "$artifact" == *.apk ]]; then
    path='assets/licenses/'
  else
    path=''
  fi
  for license in LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt; do
    local artifact_path="$path$license"
    if ! artifact_file "$artifact" "$artifact_path" >/dev/null; then
      die "$name に $artifact_path が無い"
    fi
  done
  cmp -s "$EXPECTED_LICENSE" <(artifact_file "$artifact" "$path"LICENSE 2>/dev/null) || die "$name の LICENSE が期待値と違う"
  notices="$(artifact_file "$artifact" "$path"THIRD-PARTY-NOTICES.txt 2>/dev/null || true)"
  if [[ "$artifact" == *.apk ]]; then
    grep -Fq '(gms の中の第三者)' <<<"$notices" || die "$name の依存の一覧に (gms の中の第三者) が無い"
    # 依存の一覧が部品を失っていないこと: 携帯は名指しの位置情報の部品を必ず持つ（code_verify R16・/code-review R27）
    grep -Eq '^== com\.google\.android\.gms:play-services-location ' <<<"$notices" || die "$name の依存の一覧に play-services-location が無い（部品が欠けている）"
  else
    grep -Fq '== rust-std ' <<<"$notices" || die "$name の依存の一覧に == rust-std が無い"
    # rust-std の 1 項だけの一覧は、部品を集めそこねている。サーバは画面に入る Node の部品（react）も持つ
    # pipe にしない（後段の grep -q が先に抜けると、pipefail の下で前段の SIGPIPE が失敗になる。再レビュー）
    awk '/^== / && !/^== rust-std /{f=1} END{exit !f}' <<<"$notices" || die "$name の依存の一覧に rust-std のほかの部品が無い（部品が欠けている）"
    if [[ "$name" == ashiato-server-* ]]; then
      grep -Eq '^== react ' <<<"$notices" || die "$name の依存の一覧に画面の部品（react）が無い（部品が欠けている）"
    fi
  fi
  # 部品の置き場を取り違えると（Windows のバックスラッシュの置き場で dirname が . になる等）、
  # すべての部品に木の LICENSE が入る。部品の本文がすべて木の LICENSE を含む一覧は落とす
  if ! python3 -c '
import re, sys
license=open(sys.argv[1], encoding="utf-8").read().strip()
notices=sys.stdin.read()
bodies=[]
for entry in re.split(r"(?m)^(?=== )", notices):
    lines=entry.splitlines()
    if not lines or not lines[0].startswith("== ") or lines[0].startswith("== rust-std "): continue
    body="\n".join(l for l in lines[1:] if not l.startswith("License:")).strip()
    if body: bodies.append(body)
sys.exit(1 if bodies and all(license in b for b in bodies) else 0)
' "$EXPECTED_LICENSE" <<<"$notices"; then
    die "$name の依存の一覧の部品の本文がすべて木の LICENSE と同じ（部品の置き場を取り違えている）"
  fi
  source="$(artifact_file "$artifact" "$path"SOURCE.txt 2>/dev/null || true)"
  stamp_commit="${ARTIFACT_STAMP#*+}"
  if [[ "$SOURCE_BY_COMMIT" == 1 ]]; then
    expected_commit="$(grep -Eo '^Commit: [0-9a-fA-F]{40}$' <<<"$source" | sed 's/^Commit: //' || true)"
    [[ -n "$expected_commit" && "$expected_commit" == "$stamp_commit" ]] || die "$name の SOURCE.txt が組み立ての印の commit と違う"
  else
    expected_ref="$(grep -Eo '^Ref: v[^[:space:]]+$' <<<"$source" || true)"
    [[ "$expected_ref" == "Ref: v$EXPECTED_VERSION" ]] || die "$name の SOURCE.txt がチャンネルの決まりと違う"
  fi
}

check_apk_fields() {
  local apk="$1" text field value analyzer
  analyzer="$(android_tool apkanalyzer)" || die "apkanalyzer が無いので APK の BuildConfig を検査できない"
  text="$("$analyzer" dex code --class dev.ashiato.collector.BuildConfig "$apk" 2>/dev/null)" \
    || die "$(basename "$apk") の BuildConfig が読めない"
  local found=0
  while IFS='|' read -r field value; do
    [[ -z "$field" ]] && continue
    found=$((found + 1))
    [[ -z "$value" ]] || die "$(basename "$apk") の $field が空でない"
  done < <(sed -nE 's/^\.field public static final (BASE_URL|API_TOKEN|USER_ID):[^=]+ = "(.*)"$/\1|\2/p' <<<"$text")
  [[ "$found" == 3 ]] || die "$(basename "$apk") の BuildConfig の接続先項目が $found 個しか読めない"
}

check_windows_contents() {
  local archive="$1" entries text
  entries="$(/usr/bin/unzip -l "$archive" | awk 'NR > 3 { sub(/^ +[0-9]+ +[0-9-]+ +[0-9:]+ +/, ""); sub(/[[:space:]]+$/, ""); if ($0 != "") print }' | sed '$d')" || die "Windows の zip が読めない: $(basename "$archive")"
  grep -Fxq 'ashiato-server.exe' <<<"$entries" || die "Windows の zip にサーバが無い"
  grep -Fxq 'ashiato-setup.exe' <<<"$entries" || die "Windows の zip にインストーラが無い"
  grep -Fxq 'ashiato-updater.exe' <<<"$entries" || die "Windows の zip に更新係が無い"
  grep -Fxq 'db-roles.sql' <<<"$entries" || die "Windows の zip に db-roles.sql が無い"
  grep -Eq '^pgsql/bin/postgres\.exe$' <<<"$entries" || die "Windows の zip に PostgreSQL が無い"
  # 同梱の PostgreSQL（EDB の MSVC の組み立て）が要る VC の実行時の DLL。まっさらな Windows には無い（D33（仮）・review R28）
  local crt
  for crt in vcruntime140.dll msvcp140.dll; do
    grep -Fxq "pgsql/bin/$crt" <<<"$entries" || die "Windows の zip に pgsql/bin/$crt が無い（まっさらな Windows で同梱の PostgreSQL が起きない）"
  done
  grep -Eq '(^|/)web/|(^|/)admin/' <<<"$entries" && die "Windows の zip に管理画面の配布物がある" || true
  # 同梱した PostgreSQL の許諾（ST55 D13・申し送り C12）。PostgreSQL があるなら、根の THIRD-PARTY-NOTICES.txt に見出しがある
  if grep -Eq '^pgsql/' <<<"$entries"; then
    grep -Fxq 'THIRD-PARTY-NOTICES.txt' <<<"$entries" || die 'Windows の zip に THIRD-PARTY-NOTICES.txt が無い（同梱の PostgreSQL の許諾）'
    local notices
    notices="$(/usr/bin/unzip -p "$archive" THIRD-PARTY-NOTICES.txt)"
    grep -Eq '^== PostgreSQL$' <<<"$notices" || die 'THIRD-PARTY-NOTICES.txt に PostgreSQL の許諾の見出しが無い'
    grep -Eq '^== Microsoft Visual C\+\+ Runtime$' <<<"$notices" || die 'THIRD-PARTY-NOTICES.txt に VC の実行時の DLL の見出しが無い'
    grep -Eq '^== rustls ' <<<"$notices" || die 'THIRD-PARTY-NOTICES.txt に更新係の rustls の見出しが無い'
  fi
  local server_text setup_text updater_text entry
  server_text=""; setup_text=""; updater_text=""
  while IFS= read -r entry; do
    case "$entry" in
      ashiato-server.exe|*/ashiato-server.exe) server_text+="$(/usr/bin/unzip -p "$archive" "$entry" 2>/dev/null | strings || true)"$'\n' ;;
      ashiato-setup.exe|*/ashiato-setup.exe) setup_text+="$(/usr/bin/unzip -p "$archive" "$entry" 2>/dev/null | strings || true)"$'\n' ;;
      ashiato-updater.exe|*/ashiato-updater.exe) updater_text+="$(/usr/bin/unzip -p "$archive" "$entry" 2>/dev/null | strings || true)"$'\n' ;;
    esac
  done <<<"$entries"
  grep -Fq 'ASHIATO_SETUP_TEST_' <<<"$setup_text" && die 'Windows のインストーラに試験差し込みがある'
  grep -Fq 'ASHIATO_SERVICE_TEST_' <<<"$server_text" && die 'Windows のサーバに試験差し込みがある'
  grep -Fq 'ASHIATO_UPDATER_TEST_' <<<"$updater_text" && die 'Windows の更新係に試験差し込みがある'
  grep -Fq 'settings/test-hooks.json' <<<"$updater_text" && die 'Windows の更新係に試験用設定ファイルの差し込みがある'
  grep -Fq 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20' <<<"$updater_text" || die 'Windows の更新係に GitHub releases の宛先が無い'
  # Rust の 2 つは VC の実行時を静的に持つ（.cargo/config.toml の crt-static）。DLL は pgsql\bin にだけあり、
  # 根の 2 つが VCRUNTIME140.dll を読むと、まっさらな Windows ではインストーラが起きない（D33（仮）・review N1）
  local exe
  for exe in ashiato-server.exe ashiato-setup.exe ashiato-updater.exe; do
    /usr/bin/unzip -p "$archive" "$exe" 2>/dev/null | strings | grep -Eiq '(vcruntime|msvcp)1[0-9]{2}[_0-9]*\.dll' &&
      die "Windows の zip の $exe が VC の実行時の DLL を読む（crt-static で組んでいない）"
  done
  return 0
}

check_collector_windows_contents() {
  local archive="$1" entries text
  entries="$(/usr/bin/unzip -l "$archive" | awk 'NR > 3 { sub(/^ +[0-9]+ +[0-9-]+ +[0-9:]+ +/, ""); sub(/[[:space:]]+$/, ""); if ($0 != "") print }' | sed '$d')" || die "PC の収集アプリの zip が読めない"
  grep -Eq '(^|/)ashiato-collector-windows(\.exe)?$' <<<"$entries" || die 'PC の収集アプリの zip に実行ファイルが無い'
  text="$(/usr/bin/unzip -p "$archive" ashiato-collector-windows.exe 2>/dev/null | strings || true)"
  grep -Fq 'ASHIATO_COLLECTOR_TEST_UPDATE_URL' <<<"$text" && die '配る PC の収集アプリに試験差し込みがある'
  grep -Fq 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20' <<<"$text" || die '配る PC の収集アプリに GitHub releases の宛先が無い'
}

check_artifacts() {
  local dir="$1" channel=release allow_debug=0 expected_version="" expected_license="" expected cert apk name
  shift
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --channel) channel="$2"; shift 2 ;;
      --allow-debug-key) allow_debug=1; shift ;;
      --expect-version) expected_version="$2"; shift 2 ;;
      --expect-license) expected_license="$2"; shift 2 ;;
      *) die "不明な引数: $1" ;;
    esac
  done
  if [[ -z "$expected_license" ]]; then expected_license="$root/LICENSE"; fi
  [[ -f "$expected_license" ]] || { echo "NG: 期待する LICENSE が無い: $expected_license" >&2; exit 2; }
  EXPECTED_LICENSE="$expected_license"
  [[ -d "$dir" ]] || die "配布物の置き場が無い: $dir"
  if [[ -z "$expected_version" ]]; then
    expected_version="$(find "$dir" -maxdepth 1 -type f -printf '%f\n' | sed -nE \
      -e 's/^ashiato-server-(.*)-linux-x86_64\.tar\.gz$/\1/p' \
      -e 's/^ashiato-server-(.*)-windows-x86_64\.zip$/\1/p' \
      -e 's/^ashiato-collector-windows-(.*)-x86_64\.zip$/\1/p' \
      -e 's/^ashiato-collector-dev-(.*)\.apk$/\1/p' \
      -e 's/^ashiato-collector-(.*)\.apk$/\1/p' | head -n1)"
  fi
  [[ -n "$expected_version" ]] || die "版番号を名前から読めない"
  local -a expected_names=(
    "ashiato-server-${expected_version}-linux-x86_64.tar.gz"
    "ashiato-server-${expected_version}-windows-x86_64.zip"
    "ashiato-collector-windows-${expected_version}-x86_64.zip"
  )
  if [[ "$channel" == nightly ]]; then
    expected_names+=("ashiato-collector-dev-${expected_version}.apk")
  else
    expected_names+=("ashiato-collector-${expected_version}.apk")
  fi
  EXPECTED_VERSION="$expected_version"
  if [[ "$channel" == nightly || "$allow_debug" == 1 ]]; then SOURCE_BY_COMMIT=1; else SOURCE_BY_COMMIT=0; fi
  for name in "${expected_names[@]}"; do
    [[ -f "$dir/$name" ]] || die "欠けた配布物: $name"
    ARTIFACT_STAMP="$(artifact_stamp "$dir/$name" "$expected_version")" || exit 1
    check_artifact_files "$dir/$name" "$name"
    if [[ "$name" == ashiato-server-*-windows-x86_64.zip ]]; then
      echo '== Scenario: Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない'
      check_windows_contents "$dir/$name"
    fi
    if [[ "$name" == ashiato-collector-windows-*-x86_64.zip ]]; then
      echo '== Scenario: 配る組み立ては宛先を変える口を持たない'
      check_collector_windows_contents "$dir/$name"
    fi
  done
  local windows_archive="$dir/ashiato-server-${expected_version}-windows-x86_64.zip"
  local server_stamp installer_stamp windows_text
  server_stamp="$(artifact_stamp "$windows_archive" "$expected_version")"
  installer_stamp="$(/usr/bin/unzip -p "$windows_archive" ashiato-setup.exe 2>/dev/null | strings | grep -Eo "${expected_version//./\\.}\\+[0-9a-f]{40}" | sort -u)"
  [[ "$server_stamp" == "$installer_stamp" ]] || die 'Windows のサーバとインストーラの版印が違う'
  local updater_stamp
  updater_stamp="$(/usr/bin/unzip -p "$windows_archive" ashiato-updater.exe 2>/dev/null | strings | grep -Eo "${expected_version//./\\.}\\+[0-9a-f]{40}" | sort -u)"
  [[ "$server_stamp" == "$updater_stamp" ]] || die 'Windows のサーバと更新係の版印が違う'
  windows_text="$(/usr/bin/unzip -p "$windows_archive" ashiato-server.exe ashiato-setup.exe ashiato-updater.exe 2>/dev/null | strings || true)"
  grep -Fq 'ASHIATO_SETUP_TEST_' <<<"$windows_text" && die '配るインストーラに試験差し込みがある'
  grep -Fq 'ASHIATO_SERVICE_TEST_' <<<"$windows_text" && die '配るサーバに試験差し込みがある'
  local linux_archive="$dir/ashiato-server-${expected_version}-linux-x86_64.tar.gz"
  if [[ -f "$linux_archive" ]]; then
    local linux_entries
    linux_entries="$(tar -tzf "$linux_archive")"
    echo '== Scenario: Linux の配布物は PostgreSQL を持たない'
    grep -Eq '(^|/)pgsql(/|$)|(^|/)ashiato-setup|(^|/)ashiato-updater' <<<"$linux_entries" && die 'Linux の配布物に PostgreSQL、インストーラ、または更新係がある' || true
  fi
  if [[ "$channel" == nightly ]]; then
    apk="$dir/ashiato-collector-dev-${expected_version}.apk"
    for name in "$dir"/*.apk; do
      [[ -e "$name" ]] || continue
      [[ "$(basename "$name")" == "$(basename "$apk")" ]] || die "nightly に想定外の APK がある: $(basename "$name")"
    done
    check_apk_fields "$apk"
    if [[ "$allow_debug" != 1 ]]; then
      local signer
      signer="$(android_tool apksigner)" || die "apksigner が無い"
      cert="$(apk_cert_sha256 "$signer" "$apk")"
      [[ -n "${ASHIATO_EXPECT_CERT_SHA256:-}" && "$cert" == "$ASHIATO_EXPECT_CERT_SHA256" ]] || die "APK の署名の指紋が期待値と違う"
    fi
  else
    apk="$dir/ashiato-collector-${expected_version}.apk"
    check_apk_fields "$apk"
    if [[ "$allow_debug" != 1 ]]; then
      local signer
      signer="$(android_tool apksigner)" || die "apksigner が無い"
      cert="$(apk_cert_sha256 "$signer" "$apk")"
      [[ -n "${ASHIATO_EXPECT_CERT_SHA256:-}" && "$cert" == "$ASHIATO_EXPECT_CERT_SHA256" ]] || die "APK の署名の指紋が期待値と違う"
    fi
  fi
  echo "OK: $channel の配布物を検査した（版 $expected_version）"
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

# 自己試験が追跡中の lib.rs を一時に書き換えている間だけ控えを持ち、どこで終わっても戻す（final review R16）
SELF_TEST_WORK=""
SELF_TEST_LIB_BACKUP=""
SELF_TEST_LIB="$root/crates/server/src/lib.rs"
self_test_cleanup() {
  if [[ -n "$SELF_TEST_LIB_BACKUP" ]]; then
    cp "$SELF_TEST_LIB_BACKUP" "$SELF_TEST_LIB"
    SELF_TEST_LIB_BACKUP=""
  fi
  [[ -z "$SELF_TEST_WORK" ]] || rm -rf "$SELF_TEST_WORK"
}

self_test() {
  local work; work="$(mktemp -d "${TMPDIR:-/tmp}/artifact-self-test.XXXXXX")"
  SELF_TEST_WORK="$work"
  trap self_test_cleanup EXIT
  trap 'self_test_cleanup; exit 130' INT TERM
  # apksigner の出力の形は build-tools の版で違う（36.x: `Signer #1 …`、37.0.0: `V2 Signer: …`。CI の runner は 37）。
  # 手元の SDK の版に依らず、両方の形を読めることを固定の文字列で確かめる
  local fp=54b02d70bc1ee41c22df5c9bccafd3bc2de77a99e2b14c16ff15842beeb2136c
  [[ "$(printf 'Signer #1 certificate DN: CN=x\nSigner #1 certificate SHA-256 digest: %s\nSigner #1 certificate SHA-1 digest: 00\n' "$fp" | apk_cert_from_print_certs)" == "$fp" ]] \
    || die 'apksigner 36 の形の指紋を読めない'
  [[ "$(printf 'V2 Signer: certificate DN: CN=x\nV2 Signer: certificate SHA-256 digest: %s\nV2 Signer: certificate SHA-1 digest: 00\n' "$fp" | apk_cert_from_print_certs)" == "$fp" ]] \
    || die 'apksigner 37 の形の指紋を読めない'
  [[ "$(printf 'Signer (minSdkVersion=24, maxSdkVersion=32) certificate DN: CN=x\nSigner (minSdkVersion=24, maxSdkVersion=32) certificate SHA-256 digest: %s\nSigner (minSdkVersion=24, maxSdkVersion=32) certificate SHA-1 digest: 00\n' "$fp" | apk_cert_from_print_certs)" == "$fp" ]] \
    || die 'apksigner の SDK 範囲付きの形の指紋を読めない'
  [[ "$(printf 'V2 Scheme Signer: certificate DN: CN=x\nV2 Scheme Signer: certificate SHA-256 digest: %s\nV2 Scheme Signer: certificate SHA-1 digest: 00\n' "$fp" | apk_cert_from_print_certs)" == "$fp" ]] \
    || die 'apksigner の署名者名が変わる形の指紋を読めない'
  # 版番号は生きた出所の版から組む（出所の版を上げても壊れない。final review R10）
  local v; v="$("$root/tools/release/version.sh" source)"
  # 偽の配布物の依存の一覧。rust-std のほかに、自分の本文を持つ Rust と Node の部品を 1 つずつ持つ
  fake_notices() { printf '== rust-std 1\nLicense: MIT OR Apache-2.0\n\n== anyhow 1.0.0\nLicense: MIT\nMIT License of anyhow\n\n== react 18.0.0\nLicense: MIT\nMIT License of react\n\n== rustls 1.0.0\nLicense: MIT\nMIT License of rustls\n'; }
  # ST55 の Windows のサーバの zip の形（インストーラ・PostgreSQL・db-roles.sql）に、ST53 の許諾の 3 点を足した偽物。
  # LICENSE と SOURCE.txt は $work の今の中身を写す
  win_fixture() {
    local stage="$1" stamp_file="$2" zip_out="$3"
    rm -rf "$stage"; mkdir -p "$stage/pgsql/bin"
    cp "$stamp_file" "$stage/ashiato-server.exe"
    cp "$stamp_file" "$stage/ashiato-setup.exe"
    cp "$stamp_file" "$stage/ashiato-updater.exe"
    printf 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20\n' >> "$stage/ashiato-updater.exe"
    cp "$stamp_file" "$stage/pgsql/bin/postgres.exe"
    : >"$stage/pgsql/bin/vcruntime140.dll"; : >"$stage/pgsql/bin/msvcp140.dll"
    cp "$root/tools/db-roles.sql" "$stage/db-roles.sql"
    cp "$work/LICENSE" "$work/SOURCE.txt" "$stage/"
    { fake_notices; printf '\n%s\n' '== PostgreSQL' 'PostgreSQL Database Management System' '== Microsoft Visual C++ Runtime'; } >"$stage/THIRD-PARTY-NOTICES.txt"
    rm -f "$zip_out"; (cd "$stage" && zip -q -r "$zip_out" .)
  }
  local out="$work/out" real="$work/real"; mkdir -p "$out" "$real"
  echo "== Scenario: 配布物が 1 つでも欠ければ公開しない"
  printf '%s+0000000000000000000000000000000000000000' "$v" > "$work/missing-stamp"
  cp "$root/LICENSE" "$work/LICENSE"
  fake_notices >"$work/THIRD-PARTY-NOTICES.txt"
  printf 'Commit: 0000000000000000000000000000000000000000\n' >"$work/SOURCE.txt"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" missing-stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  win_fixture "$work/windows" "$work/missing-stamp" "$out/ashiato-server-$v-windows-x86_64.zip"
  /usr/bin/unzip -l "$out/ashiato-server-$v-windows-x86_64.zip" | grep -Eq '[[:space:]]ashiato-updater\.exe$' || die '偽の Windows zip に更新係が無い'
  mkdir -p "$work/assets/licenses"
  cp "$work/LICENSE" "$work/assets/licenses/LICENSE"
  cp "$work/SOURCE.txt" "$work/assets/licenses/SOURCE.txt"
  cp "$work/THIRD-PARTY-NOTICES.txt" "$work/assets/licenses/THIRD-PARTY-NOTICES.txt"
  cp "$work/missing-stamp" "$work/classes.dex"
  (cd "$work" && zip -q "$out/ashiato-collector-$v.apk" classes.dex assets/licenses/LICENSE assets/licenses/SOURCE.txt assets/licenses/THIRD-PARTY-NOTICES.txt)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-windows-$v-x86_64.zip" <<<"$output" || { echo "$output"; return 1; }
  rm -f "$out"/*
  # Task 7 の追加シナリオは、実ビルドに依存しない小さな配布物で 3 点の検査だけを確認する。
  local mini="$work/mini" mini_tar="$work/mini.tar.gz" mini_zip="$work/mini.zip"
  mkdir -p "$mini"
  cp "$root/LICENSE" "$mini/LICENSE"
  fake_notices >"$mini/THIRD-PARTY-NOTICES.txt"
  printf 'SOURCE\n' >"$mini/SOURCE.txt"
  printf '%s+%040d\n' "$v" 0 >"$mini/stamp"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  (cd "$mini" && zip -q "$mini_zip" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt)
  EXPECTED_LICENSE="$root/LICENSE" ARTIFACT_STAMP="$v+$(printf '%040d' 0)" SOURCE_BY_COMMIT=1 EXPECTED_VERSION="$v"
  echo "== Scenario: 許諾のファイルが欠けた配布物は、公開しない"
  tar -czf "$work/missing-notice.tar.gz" -C "$mini" stamp LICENSE SOURCE.txt
  if output=$(check_artifact_files "$work/missing-notice.tar.gz" missing-notice.tar.gz 2>&1); then return 1; fi
  grep -Fq 'THIRD-PARTY-NOTICES.txt' <<<"$output" || { echo "$output"; return 1; }
  echo "== Scenario: 配布物の LICENSE が木の LICENSE と違えば、公開しない"
  printf 'AGPL の全文だけ\n' >"$mini/LICENSE"
  tar -czf "$work/wrong-license.tar.gz" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  cp "$root/LICENSE" "$mini/LICENSE"
  if output=$(check_artifact_files "$work/wrong-license.tar.gz" wrong-license.tar.gz 2>&1); then return 1; fi
  grep -Fq 'LICENSE が期待値と違う' <<<"$output" || { echo "$output"; return 1; }
  echo "== Scenario: PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る"
  local commit; commit=0123456789abcdef0123456789abcdef01234567
  printf 'Commit: %s\n' "$commit" >"$mini/SOURCE.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_tar" mini.tar.gz >/dev/null
  echo "== Scenario: debug の鍵を許す検査でも、組み立ての印の commit でないソースの場所の配布物は落ちる"
  local bad_source
  for bad_source in "Ref: v$v" 'Commit: main' 'Commit: 0123456' 'Commit: 0123456789abcdef0123456789abcdef01234568'; do
    printf '%s\n' "$bad_source" >"$mini/SOURCE.txt"
    tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
    if output=$(ARTIFACT_STAMP="$v+$commit" SOURCE_BY_COMMIT=1 check_artifact_files "$mini_tar" mini.tar.gz 2>&1); then return 1; fi
    grep -Fq 'SOURCE.txt' <<<"$output" || { echo "$output"; return 1; }
  done
  printf 'Commit: %s\n' "$commit" >"$mini/SOURCE.txt"
  echo "== Scenario: ソースの場所がチャンネルの決まりと合わない配布物は、公開しない"
  if output=$( (EXPECTED_VERSION=0.4.0-beta.1 SOURCE_BY_COMMIT=0 check_artifact_files "$mini_tar" mini.tar.gz) 2>&1); then return 1; fi
  grep -Fq 'SOURCE.txt' <<<"$output" || { echo "$output"; return 1; }
  SOURCE_BY_COMMIT=1
  echo "== Scenario: Rust の実行ファイルの依存の一覧は、標準ライブラリを含む"
  : >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(check_artifact_files "$mini_tar" mini.tar.gz 2>&1); then return 1; fi
  grep -Fq 'rust-std' <<<"$output" || { echo "$output"; return 1; }
  fake_notices >"$mini/THIRD-PARTY-NOTICES.txt"
  echo "== Scenario: 部品の本文がすべて木の LICENSE と同じ依存の一覧の配布物は、公開しない"
  # Windows のバックスラッシュの置き場で dirname が . に化け、全部の部品に木の LICENSE が入った偽物
  { printf '== rust-std 1\nLicense: MIT OR Apache-2.0\n\n'
    for crate in anyhow serde; do printf '== %s 1.0.0\nLicense: MIT\n' "$crate"; cat "$root/LICENSE"; printf '\n'; done
  } >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(check_artifact_files "$mini_tar" mini.tar.gz 2>&1); then return 1; fi
  grep -Fq '部品の本文がすべて木の LICENSE と同じ' <<<"$output" || { echo "$output"; return 1; }
  # 部品が自分の許諾の本文を持てば通る
  printf '== rust-std 1\nLicense: MIT OR Apache-2.0\n\n== anyhow 1.0.0\nLicense: MIT\nMIT License of anyhow\n' >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_tar" mini.tar.gz >/dev/null
  fake_notices >"$mini/THIRD-PARTY-NOTICES.txt"
  echo "== Scenario: 部品を失った依存の一覧（rust-std だけ・画面の部品の無いサーバ・位置情報の部品の無い携帯）の配布物は、公開しない"
  printf '== rust-std 1\nLicense: MIT OR Apache-2.0\n' >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_tar" mini.tar.gz 2>&1); then return 1; fi
  grep -Fq 'rust-std のほかの部品が無い' <<<"$output" || { echo "$output"; return 1; }
  printf '== rust-std 1\nLicense: MIT OR Apache-2.0\n\n== anyhow 1.0.0\nLicense: MIT\nMIT License of anyhow\n' >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_tar" "ashiato-server-$v-linux-x86_64.tar.gz" 2>&1); then return 1; fi
  grep -Fq '画面の部品（react）が無い' <<<"$output" || { echo "$output"; return 1; }
  fake_notices >"$mini/THIRD-PARTY-NOTICES.txt"
  tar -czf "$mini_tar" -C "$mini" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_tar" "ashiato-server-$v-linux-x86_64.tar.gz" >/dev/null
  local nolocation="$work/nolocation"
  mkdir -p "$nolocation/assets/licenses"
  cp "$root/LICENSE" "$nolocation/assets/licenses/LICENSE"
  printf '== com.google.android.gms:play-services-base 18.0.0\nLicense: x\n(gms の中の第三者)\n' >"$nolocation/assets/licenses/THIRD-PARTY-NOTICES.txt"
  printf 'Commit: %s\n' "$commit" >"$nolocation/assets/licenses/SOURCE.txt"
  (cd "$nolocation" && zip -q "$work/nolocation.apk" assets/licenses/LICENSE assets/licenses/SOURCE.txt assets/licenses/THIRD-PARTY-NOTICES.txt)
  if output=$(EXPECTED_LICENSE="$root/LICENSE" ARTIFACT_STAMP="$v+$commit" SOURCE_BY_COMMIT=1 check_artifact_files "$work/nolocation.apk" nolocation.apk 2>&1); then return 1; fi
  grep -Fq 'play-services-location が無い' <<<"$output" || { echo "$output"; return 1; }
  echo "== Scenario: 携帯の APK の依存の一覧は、Google のライブラリの中の第三者を含む"
  local apkmini="$work/apkmini"
  mkdir -p "$apkmini/assets/licenses"
  cp "$root/LICENSE" "$apkmini/assets/licenses/LICENSE"
  printf '== com.google.android.gms:play-services-location 21.0.0\nLicense: x\n(gms の中の第三者)\n' >"$apkmini/assets/licenses/THIRD-PARTY-NOTICES.txt"
  printf 'Commit: %s\n' "$commit" >"$apkmini/assets/licenses/SOURCE.txt"
  (cd "$apkmini" && zip -q "$work/mini.apk" assets/licenses/LICENSE assets/licenses/SOURCE.txt assets/licenses/THIRD-PARTY-NOTICES.txt)
  EXPECTED_LICENSE="$root/LICENSE" ARTIFACT_STAMP="$v+$commit" SOURCE_BY_COMMIT=1 check_artifact_files "$work/mini.apk" mini.apk >/dev/null
  echo "== Scenario: 配布物の LICENSE は、渡した期待する LICENSE と比べる"
  cp "$root/LICENSE" "$work/expected-license"
  printf '%s\n' 'extra expected line' >>"$work/expected-license"
  cp "$work/expected-license" "$mini/LICENSE"
  (cd "$mini" && zip -q -FS "$mini_zip" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt)
  EXPECTED_LICENSE="$work/expected-license" ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_zip" mini.zip >/dev/null
  if output=$(EXPECTED_LICENSE="$root/LICENSE" ARTIFACT_STAMP="$v+$commit" check_artifact_files "$mini_zip" mini.zip 2>&1); then return 1; fi
  grep -Fq 'LICENSE が期待値と違う' <<<"$output" || { echo "$output"; return 1; }
  echo "== 実物の release 成果物を組み立てる"
  ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" server-linux "$real"
  ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" apk "$real"
  # C-02 の release rlib から印を読み、Windows runner の成果物と同じ形の写しにする。
  local stamp
  (cd "$root" && cargo build --release -p ashiato-collector-windows >/dev/null)
  stamp="$(strings -n 8 "${CARGO_TARGET_DIR:-$root/target}/release/libashiato_collector_windows.rlib" | grep -Eo "${v//./\\.}\+[0-9a-f]{40}" | sort -u)"
  [[ -n "$stamp" ]] || die 'C-02 の rlib から組み立ての印が読めない'
  printf '%s' "$stamp" > "$work/stamp"
  printf 'Commit: %s\n' "${stamp#*+}" >"$work/SOURCE.txt"
  fake_notices >"$work/THIRD-PARTY-NOTICES.txt"
  printf 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20\n' >"$work/ashiato-collector-windows.exe"
  win_fixture "$work/windows-real" "$work/stamp" "$real/ashiato-server-$v-windows-x86_64.zip"
  (cd "$work" && zip -q "$real/ashiato-collector-windows-$v-x86_64.zip" stamp ashiato-collector-windows.exe LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt)
  cp "$real"/* "$out/"
  echo "== Scenario: 配る組み立ては試験の差し込みを持たない"
  local bad_windows="$work/bad-windows"
  mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  printf 'ASHIATO_SETUP_TEST_BAD\n' >> "$bad_windows/ashiato-setup.exe"
  printf 'ASHIATO_SERVICE_TEST_BAD\n' >> "$bad_windows/ashiato-server.exe"
  grep -Fq 'ASHIATO_SETUP_TEST_BAD' "$bad_windows/ashiato-setup.exe" || die 'self-test の偽インストーラに試験差し込みが無い'
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then die '試験差し込みのある Windows zip を通した'; fi
  rm -f "$out/bad-windows.zip"
  # 更新係の試験差し込み・試験用設定ファイル・宛先の欠落も配る形では落ちる
  rm -rf "$bad_windows"; mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  printf 'ASHIATO_UPDATER_TEST_BAD\nsettings/test-hooks.json\n' >> "$bad_windows/ashiato-updater.exe"
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then die '更新係の試験差し込みを通した'; fi
  rm -f "$out/bad-windows.zip"
  rm -rf "$bad_windows"; mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  cp "$work/stamp" "$bad_windows/ashiato-updater.exe"
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then die '宛先の無い更新係を通した'; fi
  rm -f "$out/bad-windows.zip"
  rm -rf "$bad_windows"; mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  sed -i 's#https://api.github.com/repos/rai-256/ashiato/releases##g' "$bad_windows/ashiato-updater.exe"
  printf 'https://api.github.com\n' >> "$bad_windows/ashiato-updater.exe"
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then die 'GitHub API だけの更新係を通した'; fi
  rm -f "$out/bad-windows.zip"
  rm -rf "$bad_windows"; mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  printf 'settings/test-hooks.json\nhttps://api.github.com/repos/rai-256/ashiato/releases\n' >> "$bad_windows/ashiato-updater.exe"
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then die '試験用設定ファイルのある更新係を通した'; fi
  rm -f "$out/bad-windows.zip"
  # 根の実行ファイルが VC の実行時の DLL を読めば落ちる（review N1）
  rm -rf "$bad_windows"; mkdir -p "$bad_windows"
  (cd "$bad_windows" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  printf '\0VCRUNTIME140.dll\0' >> "$bad_windows/ashiato-setup.exe"
  (cd "$bad_windows" && zip -q -r "$out/bad-windows.zip" .)
  if output=$(check_windows_contents "$out/bad-windows.zip" 2>&1); then return 1; fi
  grep -Fq 'ashiato-setup.exe が VC の実行時の DLL を読む' <<<"$output" || return 1
  rm -f "$out/bad-windows.zip"
  # PostgreSQL を同梱した zip の許諾の見出しが無ければ落ちる（申し送り C12）
  local unnoticed="$work/unnoticed-windows"
  mkdir -p "$unnoticed"
  (cd "$unnoticed" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  printf '%s\n' 'no heading' >"$unnoticed/THIRD-PARTY-NOTICES.txt"
  (cd "$unnoticed" && zip -q -r "$out/unnoticed-windows.zip" .)
  if output=$(check_windows_contents "$out/unnoticed-windows.zip" 2>&1); then die 'rustls の無い依存一覧を通した'; fi
  grep -Fq 'PostgreSQL の許諾の見出し' <<<"$output" || return 1
  rm -f "$unnoticed/THIRD-PARTY-NOTICES.txt" "$out/unnoticed-windows.zip"
  (cd "$unnoticed" && zip -q -r "$out/unnoticed-windows.zip" .)
  if output=$(check_windows_contents "$out/unnoticed-windows.zip" 2>&1); then die 'rustls の無い依存一覧を通した'; fi
  grep -Fq 'THIRD-PARTY-NOTICES.txt が無い' <<<"$output" || return 1
  rm -f "$out/unnoticed-windows.zip"
  # 更新係だけが持つ rustls の節が無い依存一覧は落ちる
  rm -rf "$unnoticed"; mkdir -p "$unnoticed"
  (cd "$unnoticed" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip")
  sed -i '/^== rustls /d' "$unnoticed/THIRD-PARTY-NOTICES.txt"
  (cd "$unnoticed" && zip -q -r "$out/unnoticed-windows.zip" .)
  if output=$(check_windows_contents "$out/unnoticed-windows.zip" 2>&1); then die 'rustls の無い依存一覧を通した'; fi
  rm -f "$out/unnoticed-windows.zip"
  # VC の実行時の DLL が無い zip は落ちる（review R28）
  local nocrt="$work/nocrt-windows"
  mkdir -p "$nocrt"
  (cd "$nocrt" && unzip -q "$out/ashiato-server-$v-windows-x86_64.zip" && rm -f pgsql/bin/vcruntime140.dll && zip -q -r "$out/nocrt-windows.zip" .)
  if output=$(check_windows_contents "$out/nocrt-windows.zip" 2>&1); then return 1; fi
  grep -Fq 'pgsql/bin/vcruntime140.dll が無い' <<<"$output" || return 1
  rm -f "$out/nocrt-windows.zip"
  echo "== Scenario: インストーラとサーバの版番号はそろう"
  local windows_name="ashiato-server-$v-windows-x86_64.zip" windows_saved="$work/windows-saved.zip"
  cp "$out/$windows_name" "$windows_saved"
  rm -rf "$work/mismatched-windows"; mkdir -p "$work/mismatched-windows"
  (cd "$work/mismatched-windows" && unzip -q "$windows_saved")
  printf '%s+1111111111111111111111111111111111111111\n' "$v" >"$work/mismatched-windows/ashiato-setup.exe"
  (cd "$work/mismatched-windows" && zip -q -r "$out/$windows_name" .)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Eiq '版印|版番号' <<<"$output" || { echo "$output"; return 1; }
  cp "$windows_saved" "$out/$windows_name"
  expected_names=(
    ashiato-server-$v-linux-x86_64.tar.gz
    ashiato-server-$v-windows-x86_64.zip
    ashiato-collector-$v.apk
    ashiato-collector-windows-$v-x86_64.zip
  )
  mapfile -t actual_names < <(find "$out" -maxdepth 1 -type f -printf '%f\n' | sort)
  mapfile -t sorted_expected < <(printf '%s\n' "${expected_names[@]}" | sort)
  [[ "${actual_names[*]}" == "${sorted_expected[*]}" ]] || die "release の配布物の名前または個数が違う"
  echo "== heavy artifact regression: missing notice"
  cp "$out/ashiato-server-$v-linux-x86_64.tar.gz" "$work/missing-notice.tar.gz"
  tar -czf "$work/missing-only.tar.gz" -C "$work" stamp LICENSE SOURCE.txt
  cp "$work/missing-only.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'THIRD-PARTY-NOTICES.txt' <<<"$output" || { echo "$output"; return 1; }
  cp "$work/missing-notice.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  cp "$out/ashiato-server-$v-linux-x86_64.tar.gz" "$work/good-linux.tar.gz"
  echo "== heavy artifact regression: wrong license"
  printf 'AGPL の全文だけ\n' >"$work/wrong-license"
  cp "$work/wrong-license" "$work/LICENSE"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  cp "$root/LICENSE" "$work/LICENSE"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'LICENSE が期待値と違う' <<<"$output" || { echo "$output"; return 1; }
  cp "$work/good-linux.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  echo "== heavy artifact regression: matching source commit"
  check_artifacts "$out" --allow-debug-key --expect-version "$v" >/dev/null
  echo "== heavy artifact regression: bad source"
  printf 'Ref: v%s\n' "$v" >"$work/SOURCE.txt"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'SOURCE.txt' <<<"$output" || { echo "$output"; return 1; }
  cp "$work/good-linux.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  printf 'Commit: %s\n' "${stamp#*+}" >"$work/SOURCE.txt"
  echo "== heavy artifact regression: beta source"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(check_artifacts "$out" --channel beta --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'SOURCE.txt' <<<"$output" || { echo "$output"; return 1; }
  cp "$work/good-linux.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  echo "== heavy artifact regression: rust std"
  : >"$work/THIRD-PARTY-NOTICES.txt"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'rust-std' <<<"$output" || { echo "$output"; return 1; }
  cp "$work/good-linux.tar.gz" "$out/ashiato-server-$v-linux-x86_64.tar.gz"
  fake_notices >"$work/THIRD-PARTY-NOTICES.txt"
  echo "== heavy artifact regression: gms notices"
  cp "$out/ashiato-collector-$v.apk" "$work/no-gms.apk"
  /usr/bin/zip -q -d "$work/no-gms.apk" 'assets/licenses/THIRD-PARTY-NOTICES.txt'
  mkdir -p "$work/gms-missing/assets/licenses"
  : >"$work/gms-missing/assets/licenses/THIRD-PARTY-NOTICES.txt"
  (cd "$work/gms-missing" && /usr/bin/zip -q "$work/no-gms.apk" assets/licenses/THIRD-PARTY-NOTICES.txt)
  cp "$work/no-gms.apk" "$out/ashiato-collector-$v.apk"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq 'gms' <<<"$output" || { echo "$output"; return 1; }
  cp "$real/ashiato-collector-$v.apk" "$out/ashiato-collector-$v.apk"
  echo "== Scenario: 4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ"
  check_artifacts "$out" --allow-debug-key --expect-version "$v" >/dev/null
  echo "== heavy artifact regression: expected license"
  cp "$root/LICENSE" "$work/expected-license"
  check_artifacts "$out" --allow-debug-key --expect-version "$v" --expect-license "$work/expected-license" >/dev/null
  echo "== Scenario: .dirty の印を持つ配布物があれば公開しない"
  cp "$SELF_TEST_LIB" "$work/server-lib.rs"
  SELF_TEST_LIB_BACKUP="$work/server-lib.rs"
  printf '%s\n' '// self-test dirty' >> "$SELF_TEST_LIB"
  local dirty_rc=0
  CARGO_TARGET_DIR="$work/dirty-target" ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" server-linux "$work/dirty" || dirty_rc=$?
  cp "$SELF_TEST_LIB_BACKUP" "$SELF_TEST_LIB"
  SELF_TEST_LIB_BACKUP=""
  [[ "$dirty_rc" == 0 ]] || return 1
  cp "$work/dirty/ashiato-server-$v-linux-x86_64.tar.gz" "$out/"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq "ashiato-server-$v-linux-x86_64.tar.gz" <<<"$output" || { echo "$output"; return 1; }
  grep -Fq '.dirty' <<<"$output" || { echo "$output"; return 1; }
  cp "$real/ashiato-server-$v-linux-x86_64.tar.gz" "$out/"
  echo "== Scenario: .debug の印を持つ配布物があれば公開しない"
  cp "$real/ashiato-collector-$v.apk" "$out/ashiato-collector-$v.apk"
  printf '%s.debug\n' "$stamp" >"$work/classes.dex"
  (cd "$work" && zip -q "$out/ashiato-collector-$v.apk" classes.dex)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-$v.apk" <<<"$output"
  grep -Fq '.debug' <<<"$output"
  cp "$real/ashiato-collector-$v.apk" "$out/"
  echo "== Scenario: 資格情報を焼き込んだ APK があれば公開せず、値を示さない"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$v" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -Pkotlin.incremental=false -q \
    -Pashiato.baseUrl= -Pashiato.apiToken=secret-token-for-test -Pashiato.userId= :app:assembleRelease)
  cp "$root/collector-android/app/build/outputs/apk/release/app-release.apk" "$out/ashiato-collector-$v.apk"
  mkdir -p "$work/credential-assets/assets/licenses"
  for license_file in LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt; do
    /usr/bin/unzip -p "$real/ashiato-collector-$v.apk" "assets/licenses/$license_file" >"$work/credential-assets/assets/licenses/$license_file"
  done
  (cd "$work/credential-assets" && zip -q "$out/ashiato-collector-$v.apk" assets/licenses/LICENSE assets/licenses/THIRD-PARTY-NOTICES.txt assets/licenses/SOURCE.txt)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq API_TOKEN <<<"$output" || { echo "$output"; return 1; }
  if grep -Fq secret-token-for-test <<<"$output"; then
    echo "資格情報の値が検査出力に漏れた" >&2
    return 1
  fi
  cp "$real/ashiato-collector-$v.apk" "$out/"
  echo "== Scenario: 綺麗な release で資格情報の無い配布物は、検査を通る"
  if ! output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then
    echo "$output"
    return 1
  fi
  local release_out="$work/release"
  mkdir -p "$release_out"
  ASHIATO_SOURCE_REF="v$v" ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" server-linux "$release_out"
  ASHIATO_SOURCE_REF="v$v" ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" apk "$release_out"
  printf 'Ref: v%s\n' "$v" >"$work/SOURCE.txt"
  printf 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20\n' >"$work/ashiato-collector-windows.exe"
  win_fixture "$work/release-windows" "$work/stamp" "$release_out/ashiato-server-$v-windows-x86_64.zip"
  (cd "$work" && zip -q "$release_out/ashiato-collector-windows-$v-x86_64.zip" stamp ashiato-collector-windows.exe LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt)
  # --allow-debug-key 無しの経路も、署名の指紋の一致・不一致を検査する（final review R7）。
  local signer cert
  signer="$(android_tool apksigner)" || die 'apksigner が見つからない'
  cert="$(apk_cert_sha256 "$signer" "$release_out/ashiato-collector-$v.apk")"
  [[ -n "$cert" ]] || die '署名の指紋を読めない'
  if ! output=$(ASHIATO_EXPECT_CERT_SHA256="$cert" check_artifacts "$release_out" --expect-version "$v" 2>&1); then
    echo "$output"
    return 1
  fi
  if output=$(ASHIATO_EXPECT_CERT_SHA256=0000 check_artifacts "$release_out" --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq '署名の指紋' <<<"$output" || { echo "$output"; return 1; }
  # apk-version --expect-version は APK の版を渡した版番号と比べる（code_verify R25）
  (apk_version "$real/ashiato-collector-$v.apk" "$v") >/dev/null
  if output=$( (apk_version "$real/ashiato-collector-$v.apk" "$v-beta.2") 2>&1); then return 1; fi
  grep -Fq '渡した版番号と違う' <<<"$output" || { echo "$output"; return 1; }
  local nightly="$work/nightly"; mkdir -p "$nightly"
  local nightly_version="$v-dev.20261102"
  echo "== Scenario: 開発版のリリースの配布物は、開発版の APK を含む 4 つで、同じ版番号を持つ"
  printf '%s+0000000000000000000000000000000000000000' "$nightly_version" > "$work/stamp"
  printf 'Commit: 0000000000000000000000000000000000000000\n' >"$work/SOURCE.txt"
  fake_notices >"$work/THIRD-PARTY-NOTICES.txt"
  printf 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20\n' >"$work/ashiato-collector-windows.exe"
  tar -czf "$nightly/ashiato-server-$nightly_version-linux-x86_64.tar.gz" -C "$work" stamp LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt
  win_fixture "$work/nightly-windows" "$work/stamp" "$nightly/ashiato-server-$nightly_version-windows-x86_64.zip"
  (cd "$work" && zip -q "$nightly/ashiato-collector-windows-$nightly_version-x86_64.zip" stamp ashiato-collector-windows.exe LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt)
  local dev_key="$work/nightly-dev.jks" dev_password=self-test-password dev_b64 dev_cert
  printf '%s\n' "$dev_password" | "$root/tools/release/make-key.sh" dev "$dev_key" >/dev/null
  dev_b64="$(base64 -w0 "$dev_key")"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION="$nightly_version" ASHIATO_APK_UNSIGNED=1 quiet_build "$work/build-nightly.log" "$root/tools/release/build.sh" apk "$work/nightly-build"
  dev_cert="$(keytool -J-Duser.language=en -J-Duser.country=US -list -v -keystore "$dev_key" -storepass "$dev_password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  ASHIATO_EXPECT_DEV_CERT_SHA256="$dev_cert" ASHIATO_SIGNING_KEYSTORE_B64="$dev_b64" ASHIATO_SIGNING_STORE_PASSWORD="$dev_password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$dev_password" \
    "$root/tools/release/sign-apk.sh" "$work/nightly-build/ashiato-collector-$nightly_version.apk" dev "$nightly/ashiato-collector-dev-$nightly_version.apk" >/dev/null
  ASHIATO_EXPECT_CERT_SHA256="$dev_cert" check_artifacts "$nightly" --channel nightly --expect-version "$nightly_version" >/dev/null
  echo "== Scenario: 開発版の置き場に携帯の APK が無ければ、検査は落ちる"
  rm -f "$nightly/ashiato-collector-dev-$nightly_version.apk"
  if output=$(check_artifacts "$nightly" --channel nightly --expect-version "$nightly_version" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-dev-$nightly_version.apk" <<<"$output"
  echo "== Scenario: 開発版の置き場に正式版の名前の APK があれば、検査は落ちてその名前を示す"
  cp "$work/nightly-build/ashiato-collector-$nightly_version.apk" "$nightly/ashiato-collector-dev-$nightly_version.apk"
  touch "$nightly/ashiato-collector-$nightly_version.apk"
  if output=$(ASHIATO_EXPECT_CERT_SHA256="$dev_cert" check_artifacts "$nightly" --channel nightly --expect-version "$nightly_version" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-$nightly_version.apk" <<<"$output"
  rm -f "$nightly/ashiato-collector-$nightly_version.apk"
  echo "== Scenario: 開発版の置き場に正式版の名前の APK があれば、検査は落ちてその名前を示す"
  cp "$nightly/ashiato-collector-dev-$nightly_version.apk" "$nightly/ashiato-collector-$nightly_version.apk"
  if output=$(ASHIATO_EXPECT_CERT_SHA256="$dev_cert" check_artifacts "$nightly" --channel nightly --expect-version "$nightly_version" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-$nightly_version.apk" <<<"$output"
  echo "== Scenario: 1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ"
  if ! output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then
    echo "$output"
    return 1
  fi
  echo "OK: 実物の release、欠落、印、資格情報、名前、チャンネルを検査した"
}

case "${1:-}" in
  apk-version) shift; want=
    if [ "${1:-}" = --expect-version ]; then [ "$#" -ge 2 ] || usage; want="$2"; shift 2; fi
    [ "$#" -ge 1 ] || die "APK が無い"; for apk in "$@"; do apk_version "$apk" "$want"; done ;;
  --self-test) self_test ;;
  --self-test-android) self_test_android ;;
  apk-label) [ "$#" = 2 ] || usage; apk_label "$2" ;;
  apk-manifest) [ "$#" = 2 ] || usage; apk_manifest "$2" ;;
  check) shift; [ "$#" -ge 1 ] || die "置き場が無い"; check_artifacts "$@" ;;
  --self-test-gradle-range) range_self_test ;;
  *)
    if [[ $# -ge 1 && -d "$1" ]]; then dir="$1"; shift; check_artifacts "$dir" "$@"
    else usage
    fi ;;
esac
