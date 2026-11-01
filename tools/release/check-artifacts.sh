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
  [[ "$(wc -l <<<"$stamp")" == 1 ]] || die "組み立ての版番号が 1 つでない: $(basename "$artifact")"
  if grep -Fq '.dirty' <<<"$stamp"; then die "$(basename "$artifact") に .dirty の印がある"; fi
  if grep -Fq '.debug' <<<"$stamp"; then die "$(basename "$artifact") に .debug の印がある"; fi
}

check_apk_fields() {
  local apk="$1" text field value
  if command -v apkanalyzer >/dev/null 2>&1 && apkanalyzer dex code --class dev.ashiato.collector.BuildConfig "$apk" >/dev/null 2>&1; then
    text="$(apkanalyzer dex code --class dev.ashiato.collector.BuildConfig "$apk")"
    while IFS='|' read -r field value; do
      [[ -z "$field" ]] && continue
      [[ -z "$value" ]] || die "$(basename "$apk") の $field が空でない"
    done < <(sed -nE 's/^\.field public static final (BASE_URL|API_TOKEN|USER_ID):[^=]+ = "(.*)"$/\1|\2/p' <<<"$text")
  else
    text="$(/usr/bin/unzip -p "$apk" 'classes*.dex' 2>/dev/null | strings || true)"
    if grep -Fq secret-token-for-test <<<"$text"; then die "$(basename "$apk") の API_TOKEN が空でない"; fi
    if grep -Eiq 'https?://[^[:space:]]+' <<<"$text"; then die "$(basename "$apk") の BASE_URL が空でない"; fi
    if grep -Eiq 'user-[[:alnum:]_-]+' <<<"$text"; then die "$(basename "$apk") の USER_ID が空でない"; fi
  fi
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
  local work="$TMPDIR"/artifact-self-test.XXXXXX; work="$(mktemp -d "$work")"; trap 'rm -rf "$work"' RETURN
  local out="$work/out"; mkdir -p "$out"
  echo "== Scenario: 配布物が 1 つでも欠ければ公開しない"
  touch "$out/ashiato-server-0.1.0-linux-x86_64.tar.gz"; if (check_artifacts "$out" --allow-debug-key --expect-version 0.1.0 >/dev/null 2>&1); then return 1; fi
  echo "== Scenario: 開発版の配布物の検査は、携帯の APK を除く 3 つで通る"
  printf '0.4.0-dev.20261102+0000000000000000000000000000000000000000' > "$work/stamp"
  tar -czf "$out/ashiato-server-0.4.0-dev.20261102-linux-x86_64.tar.gz" -C "$work" stamp
  (cd "$work" && zip -q "$out/ashiato-server-0.4.0-dev.20261102-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$out/ashiato-collector-windows-0.4.0-dev.20261102-x86_64.zip" stamp)
  if (check_artifacts "$out" --channel nightly --expect-version 0.4.0-dev.20261102); then :; else return 1; fi
  echo "== Scenario: 開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す"
  touch "$out/ashiato-collector-dev-0.4.0-dev.20261102.apk"; if output=$(check_artifacts "$out" --channel nightly --expect-version 0.4.0-dev.20261102 2>&1); then return 1; fi; grep -Fq 'ashiato-collector-dev-0.4.0-dev.20261102.apk' <<<"$output"
  echo "== Scenario: 1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ"
  local release="$work/release"; mkdir -p "$release"
  printf '0.1.0+0000000000000000000000000000000000000000' > "$work/stamp"
  tar -czf "$release/ashiato-server-0.1.0-linux-x86_64.tar.gz" -C "$work" stamp
  (cd "$work" && zip -q "$release/ashiato-server-0.1.0-windows-x86_64.zip" stamp)
  (cd "$work" && zip -q "$release/ashiato-collector-windows-0.1.0-x86_64.zip" stamp)
  cp "$work/stamp" "$work/classes.dex"
  (cd "$work" && zip -q "$release/ashiato-collector-0.1.0.apk" classes.dex)
  check_artifacts "$release" --allow-debug-key --expect-version 0.1.0 >/dev/null
  echo "== Scenario: .dirty の印を持つ配布物があれば公開しない"
  printf '0.1.0+0000000000000000000000000000000000000000.dirty' > "$work/stamp"
  tar -czf "$release/ashiato-server-0.1.0-linux-x86_64.tar.gz" -C "$work" stamp
  if (check_artifacts "$release" --allow-debug-key --expect-version 0.1.0 >/dev/null 2>&1); then return 1; fi
  echo "== Scenario: .debug の印を持つ配布物があれば公開しない"
  printf '0.1.0+0000000000000000000000000000000000000000.debug' > "$work/stamp"
  tar -czf "$release/ashiato-server-0.1.0-linux-x86_64.tar.gz" -C "$work" stamp
  if (check_artifacts "$release" --allow-debug-key --expect-version 0.1.0 >/dev/null 2>&1); then return 1; fi
  echo "== Scenario: 資格情報を焼き込んだ APK があれば公開せず、値を示さない"
  printf '0.1.0+0000000000000000000000000000000000000000' > "$work/stamp"
  tar -czf "$release/ashiato-server-0.1.0-linux-x86_64.tar.gz" -C "$work" stamp
  printf '0.1.0+0000000000000000000000000000000000000000 API_TOKEN secret-token-for-test' > "$work/classes.dex"
  (cd "$work" && zip -q "$release/ashiato-collector-0.1.0.apk" classes.dex)
  if output=$(check_artifacts "$release" --allow-debug-key --expect-version 0.1.0 2>&1); then return 1; fi
  grep -Fq API_TOKEN <<<"$output"; ! grep -Fq secret-token-for-test <<<"$output"
  echo "== Scenario: 綺麗な release で資格情報の無い配布物は、検査を通る"
  printf '0.1.0+0000000000000000000000000000000000000000' > "$work/stamp"
  cp "$work/stamp" "$work/classes.dex"
  rm -f "$release/ashiato-collector-0.1.0.apk"
  (cd "$work" && zip -q "$release/ashiato-collector-0.1.0.apk" classes.dex)
  check_artifacts "$release" --allow-debug-key --expect-version 0.1.0 >/dev/null
  echo "OK: 欠落、印、資格情報、名前、チャンネルを検査した"
}

case "${1:-}" in
  apk-version) shift; [ "$#" -ge 1 ] || die "APK が無い"; for apk in "$@"; do apk_version "$apk"; done ;;
  --self-test) self_test ;;
  check) shift; [ "$#" -ge 1 ] || die "置き場が無い"; check_artifacts "$@" ;;
  --self-test-gradle-range) range_self_test ;;
  *) echo "使い方: $0 apk-version <apk...>|--self-test-gradle-range" >&2; exit 2 ;;
esac
