#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <置き場> [--channel <release|beta|nightly>] [--allow-debug-key] [--expect-version <版番号>] | check <置き場> [...] | apk-version <apk...> | --self-test" >&2; exit 2; }

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
  [[ "$(wc -l <<<"$stamp")" == 1 ]] || die "組み立ての版番号が 1 つでない: $(basename "$artifact")"
  if grep -Fq '.dirty' <<<"$stamp"; then die "$(basename "$artifact") に .dirty の印がある"; fi
  if grep -Fq '.debug' <<<"$stamp"; then die "$(basename "$artifact") に .debug の印がある"; fi
}

check_apk_fields() {
  local apk="$1" text field value
  command -v apkanalyzer >/dev/null 2>&1 || die "apkanalyzer が無いので APK の BuildConfig を検査できない"
  text="$(apkanalyzer dex code --class dev.ashiato.collector.BuildConfig "$apk" 2>/dev/null)" \
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
      -e 's/^ashiato-collector-(.*)\.apk$/\1/p' | head -n1)"
  fi
  [[ -n "$expected_version" ]] || die "版番号を名前から読めない"
  local -a expected_names=(
    "ashiato-server-${expected_version}-linux-x86_64.tar.gz"
    "ashiato-server-${expected_version}-windows-x86_64.zip"
    "ashiato-collector-windows-${expected_version}-x86_64.zip"
  )
  [[ "$channel" == nightly ]] || expected_names+=("ashiato-collector-${expected_version}.apk")
  for name in "${expected_names[@]}"; do
    [[ -f "$dir/$name" ]] || die "欠けた配布物: $name"
    artifact_stamp "$dir/$name" "$expected_version"
  done
  if [[ "$channel" == nightly ]]; then
    for apk in "$dir"/*.apk; do [[ -e "$apk" ]] || continue; die "nightly に APK がある: $(basename "$apk")"; done
  else
    apk="$dir/ashiato-collector-${expected_version}.apk"
    check_apk_fields "$apk"
    if [[ "$allow_debug" != 1 ]]; then
      command -v apksigner >/dev/null 2>&1 || die "apksigner が無い"
      cert="$(apksigner verify --print-certs "$apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n1)"
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

self_test() {
  local work; work="$(mktemp -d "${TMPDIR:-/tmp}/artifact-self-test.XXXXXX")"
  trap "rm -f '$root/crates/server/src/.st52-release-self-test-dirty'; rm -rf '$work'" EXIT
  local out="$work/out" real="$work/real"; mkdir -p "$out" "$real"
  echo "== Scenario: 配布物が 1 つでも欠ければ公開しない"
  printf '0.1.0+0000000000000000000000000000000000000000' > "$work/missing-stamp"
  tar -czf "$out/ashiato-server-0.1.0-linux-x86_64.tar.gz" -C "$work" missing-stamp
  (cd "$work" && zip -q "$out/ashiato-server-0.1.0-windows-x86_64.zip" missing-stamp)
  cp "$work/missing-stamp" "$work/classes.dex"
  (cd "$work" && zip -q "$out/ashiato-collector-0.1.0.apk" classes.dex)
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 2>&1); then return 1; fi
  grep -Fq 'ashiato-collector-windows-0.1.0-x86_64.zip' <<<"$output"
  rm -f "$out"/*
  echo "== 実物の release 成果物を組み立てる"
  ASHIATO_BUILD_VERSION=0.1.0 "$root/tools/release/build.sh" server-linux "$real"
  ASHIATO_BUILD_VERSION=0.1.0 "$root/tools/release/build.sh" apk "$real"
  # C-02 の release rlib から印を読み、Windows runner の成果物と同じ形の写しにする。
  local stamp
  (cd "$root" && cargo build --release -p ashiato-collector-windows >/dev/null)
  stamp="$(strings -n 8 "${CARGO_TARGET_DIR:-$root/target}/release/libashiato_collector_windows.rlib" | grep -Eo '0\.1\.0\+[0-9a-f]{40}' | sort -u)"
  [[ -n "$stamp" ]] || die 'C-02 の rlib から組み立ての印が読めない'
  printf '%s' "$stamp" > "$work/stamp"
  (cd "$work" && zip -q "$real/ashiato-server-0.1.0-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$real/ashiato-collector-windows-0.1.0-x86_64.zip" stamp)
  cp "$real"/* "$out/"
  expected_names=(
    ashiato-server-0.1.0-linux-x86_64.tar.gz
    ashiato-server-0.1.0-windows-x86_64.zip
    ashiato-collector-0.1.0.apk
    ashiato-collector-windows-0.1.0-x86_64.zip
  )
  mapfile -t actual_names < <(find "$out" -maxdepth 1 -type f -printf '%f\n' | sort)
  mapfile -t sorted_expected < <(printf '%s\n' "${expected_names[@]}" | sort)
  [[ "${actual_names[*]}" == "${sorted_expected[*]}" ]] || die "release の配布物の名前または個数が違う"
  echo "== Scenario: .dirty の印を持つ配布物があれば公開しない"
  local server_src="$root/crates/server/src/lib.rs" server_backup="$work/server-lib.rs"
  cp "$server_src" "$server_backup"
  printf '%s\n' '// self-test dirty' >> "$server_src"
  if ! CARGO_TARGET_DIR="$work/dirty-target" ASHIATO_BUILD_VERSION=0.1.0 "$root/tools/release/build.sh" server-linux "$work/dirty"; then
    cp "$server_backup" "$server_src"
    return 1
  fi
  cp "$server_backup" "$server_src"
  cp "$work/dirty/ashiato-server-0.1.0-linux-x86_64.tar.gz" "$out/"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 2>&1); then return 1; fi
  grep -Fq 'ashiato-server-0.1.0-linux-x86_64.tar.gz' <<<"$output" || { echo "$output"; return 1; }
  grep -Fq '.dirty' <<<"$output" || { echo "$output"; return 1; }
  cp "$real/ashiato-server-0.1.0-linux-x86_64.tar.gz" "$out/"
  echo "== Scenario: .debug の印を持つ配布物があれば公開しない"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION=0.1.0 ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q \
    -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= :app:assembleDebug)
  cp "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" "$out/ashiato-collector-0.1.0.apk"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 2>&1); then return 1; fi
  grep -Fq 'ashiato-collector-0.1.0.apk' <<<"$output"
  grep -Fq '.debug' <<<"$output"
  cp "$real/ashiato-collector-0.1.0.apk" "$out/"
  echo "== Scenario: 資格情報を焼き込んだ APK があれば公開せず、値を示さない"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION=0.1.0 ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q \
    -Pashiato.baseUrl= -Pashiato.apiToken=secret-token-for-test -Pashiato.userId= :app:assembleRelease)
  cp "$root/collector-android/app/build/outputs/apk/release/app-release.apk" "$out/ashiato-collector-0.1.0.apk"
  if output=$(check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 2>&1); then return 1; fi
  grep -Fq API_TOKEN <<<"$output" || { echo "$output"; return 1; }
  if grep -Fq secret-token-for-test <<<"$output"; then
    echo "資格情報の値が検査出力に漏れた" >&2
    return 1
  fi
  cp "$real/ashiato-collector-0.1.0.apk" "$out/"
  echo "== Scenario: 綺麗な release で資格情報の無い配布物は、検査を通る"
  check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 >/dev/null
  local nightly="$work/nightly"; mkdir -p "$nightly"
  echo "== Scenario: 開発版の配布物の検査は、携帯の APK を除く 3 つで通る"
  printf '0.4.0-dev.20261102+0000000000000000000000000000000000000000' > "$work/stamp"
  tar -czf "$nightly/ashiato-server-0.4.0-dev.20261102-linux-x86_64.tar.gz" -C "$work" stamp
  (cd "$work" && zip -q "$nightly/ashiato-server-0.4.0-dev.20261102-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$nightly/ashiato-collector-windows-0.4.0-dev.20261102-x86_64.zip" stamp)
  check_artifacts "$nightly" --channel nightly --expect-version 0.4.0-dev.20261102 >/dev/null
  echo "== Scenario: 開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す"
  for apk_name in \
    ashiato-collector-dev-0.4.0-dev.20261102.apk \
    ashiato-collector-0.4.0-dev.20261102.apk; do
    touch "$nightly/$apk_name"
    if output=$(check_artifacts "$nightly" --channel nightly --expect-version 0.4.0-dev.20261102 2>&1); then return 1; fi
    grep -Fq "$apk_name" <<<"$output"
    rm -f "$nightly/$apk_name"
  done
  echo "== Scenario: 1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ"
  check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 >/dev/null
  echo "OK: 実物の release、欠落、印、資格情報、名前、チャンネルを検査した"
}

case "${1:-}" in
  apk-version) shift; [ "$#" -ge 1 ] || die "APK が無い"; for apk in "$@"; do apk_version "$apk"; done ;;
  --self-test) self_test ;;
  check) shift; [ "$#" -ge 1 ] || die "置き場が無い"; check_artifacts "$@" ;;
  --self-test-gradle-range) range_self_test ;;
  *)
    if [[ $# -ge 1 && -d "$1" ]]; then dir="$1"; shift; check_artifacts "$dir" "$@"
    else usage
    fi ;;
esac
