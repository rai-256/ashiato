#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
quiet_build() {
  local log="$1"; shift
  "$@" >"$log" 2>&1 || { tail -n 40 "$log" >&2; return 1; }
}
. "$root/tools/release/android-tools.sh"
usage() { echo "使い方: $0 <置き場> [...] | check <置き場> [...] | apk-version <apk...> | apk-label <apk> | apk-manifest <apk> | --self-test | --self-test-android" >&2; exit 2; }

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

check_artifacts() {
  local dir="$1" channel=release allow_debug=0 expected_version="" expected cert apk name
  shift
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --channel) channel="$2"; shift 2 ;;
      --allow-debug-key) allow_debug=1; shift ;;
      --expect-version) expected_version="$2"; shift 2 ;;
      *) die "不明な引数: $1" ;;
    esac
  done
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
  for name in "${expected_names[@]}"; do
    [[ -f "$dir/$name" ]] || die "欠けた配布物: $name"
    artifact_stamp "$dir/$name" "$expected_version"
  done
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
  local out="$work/out" real="$work/real"; mkdir -p "$out" "$real"
  echo "== Scenario: 配布物が 1 つでも欠ければ公開しない"
  printf '%s+0000000000000000000000000000000000000000' "$v" > "$work/missing-stamp"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$work" missing-stamp
  (cd "$work" && zip -q "$out/ashiato-server-$v-windows-x86_64.zip" missing-stamp)
  cp "$work/missing-stamp" "$work/classes.dex"
  (cd "$work" && zip -q "$out/ashiato-collector-$v.apk" classes.dex)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-windows-$v-x86_64.zip" <<<"$output"
  rm -f "$out"/*
  echo "== 実物の release 成果物を組み立てる"
  ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" server-linux "$real"
  ASHIATO_BUILD_VERSION="$v" "$root/tools/release/build.sh" apk "$real"
  # C-02 の release rlib から印を読み、Windows runner の成果物と同じ形の写しにする。
  local stamp
  (cd "$root" && cargo build --release -p ashiato-collector-windows >/dev/null)
  stamp="$(strings -n 8 "${CARGO_TARGET_DIR:-$root/target}/release/libashiato_collector_windows.rlib" | grep -Eo "${v//./\\.}\+[0-9a-f]{40}" | sort -u)"
  [[ -n "$stamp" ]] || die 'C-02 の rlib から組み立ての印が読めない'
  printf '%s' "$stamp" > "$work/stamp"
  (cd "$work" && zip -q "$real/ashiato-server-$v-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$real/ashiato-collector-windows-$v-x86_64.zip" stamp)
  cp "$real"/* "$out/"
  expected_names=(
    ashiato-server-$v-linux-x86_64.tar.gz
    ashiato-server-$v-windows-x86_64.zip
    ashiato-collector-$v.apk
    ashiato-collector-windows-$v-x86_64.zip
  )
  mapfile -t actual_names < <(find "$out" -maxdepth 1 -type f -printf '%f\n' | sort)
  mapfile -t sorted_expected < <(printf '%s\n' "${expected_names[@]}" | sort)
  [[ "${actual_names[*]}" == "${sorted_expected[*]}" ]] || die "release の配布物の名前または個数が違う"
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
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$v" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -Pkotlin.incremental=false -q \
    -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= :app:assembleDebug)
  cp "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" "$out/ashiato-collector-$v.apk"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq "ashiato-collector-$v.apk" <<<"$output"
  grep -Fq '.debug' <<<"$output"
  cp "$real/ashiato-collector-$v.apk" "$out/"
  echo "== Scenario: 資格情報を焼き込んだ APK があれば公開せず、値を示さない"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$v" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -Pkotlin.incremental=false -q \
    -Pashiato.baseUrl= -Pashiato.apiToken=secret-token-for-test -Pashiato.userId= :app:assembleRelease)
  cp "$root/collector-android/app/build/outputs/apk/release/app-release.apk" "$out/ashiato-collector-$v.apk"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq API_TOKEN <<<"$output" || { echo "$output"; return 1; }
  if grep -Fq secret-token-for-test <<<"$output"; then
    echo "資格情報の値が検査出力に漏れた" >&2
    return 1
  fi
  cp "$real/ashiato-collector-$v.apk" "$out/"
  echo "== Scenario: 綺麗な release で資格情報の無い配布物は、検査を通る"
  check_artifacts "$out" --allow-debug-key --expect-version "$v" >/dev/null
  # --allow-debug-key 無しの経路: 署名の指紋を apksigner（PATH に無ければ SDK の下）で読んで比べる（final review R7）
  local signer cert
  signer="$(android_tool apksigner)" || die 'apksigner が見つからない'
  cert="$(apk_cert_sha256 "$signer" "$out/ashiato-collector-$v.apk")"
  [[ -n "$cert" ]] || die '署名の指紋を読めない'
  ASHIATO_EXPECT_CERT_SHA256="$cert" check_artifacts "$out" --expect-version "$v" >/dev/null
  if output=$(ASHIATO_EXPECT_CERT_SHA256=0000 check_artifacts "$out" --expect-version "$v" 2>&1); then return 1; fi
  grep -Fq '署名の指紋' <<<"$output" || { echo "$output"; return 1; }
  # apk-version --expect-version は APK の版を渡した版番号と比べる（code_verify R25）
  (apk_version "$real/ashiato-collector-$v.apk" "$v") >/dev/null
  if output=$( (apk_version "$real/ashiato-collector-$v.apk" "$v-beta.2") 2>&1); then return 1; fi
  grep -Fq '渡した版番号と違う' <<<"$output" || { echo "$output"; return 1; }
  local nightly="$work/nightly"; mkdir -p "$nightly"
  local nightly_version="$v-dev.20261102"
  echo "== Scenario: 開発版のリリースの配布物は、開発版の APK を含む 4 つで、同じ版番号を持つ"
  printf '%s+0000000000000000000000000000000000000000' "$nightly_version" > "$work/stamp"
  tar -czf "$nightly/ashiato-server-$nightly_version-linux-x86_64.tar.gz" -C "$work" stamp
  (cd "$work" && zip -q "$nightly/ashiato-server-$nightly_version-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$nightly/ashiato-collector-windows-$nightly_version-x86_64.zip" stamp)
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
  check_artifacts "$out" --allow-debug-key --expect-version "$v" >/dev/null
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
