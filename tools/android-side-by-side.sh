#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$root/tools/android-env.sh"
export PATH="$ANDROID_HOME/emulator:$PATH"
source_version="$("$root/tools/release/version.sh" source)"
dev_version="$source_version-dev.20261102"
serial="${ANDROID_SERIAL:-emulator-5554}"
started=0
payload="$(mktemp "${TMPDIR:-/tmp}/side-by-side.XXXXXX")"
printf 'side-by-side\n' > "$payload"
cleanup() {
  rm -f "$payload"
  if [[ "$started" == 1 ]]; then adb -s "$serial" emu kill >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

if ! adb -s "$serial" get-state >/dev/null 2>&1; then
  avd="${AVD:-ashiato-api35}"
  image="system-images;android-35;google_apis;x86_64"
  if ! avdmanager list avd -c 2>/dev/null | grep -qx "$avd"; then
    echo no | avdmanager create avd -n "$avd" -k "$image" -d pixel_6 >/dev/null
  fi
  emulator -avd "$avd" -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -no-snapshot >/dev/null 2>&1 &
  adb wait-for-device
  for _ in $(seq 1 180); do
    [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] && break
    sleep 2
  done
  [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] || { echo "エミュレータの起動が終わらない" >&2; exit 1; }
  started=1
fi
adb -s "$serial" wait-for-device >/dev/null

build_and_install() {
  local version="$1"
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$version" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q \
    -Pashiato.baseUrl= -Pashiato.apiToken= -Pashiato.userId= :app:assembleDebug)
  adb -s "$serial" install -r "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" >/dev/null
}

build_and_install "$source_version"
data_dir="/data/user/0/dev.ashiato.collector"
adb -s "$serial" shell run-as dev.ashiato.collector mkdir -p "$data_dir/files/outbox"
adb -s "$serial" push "$payload" /data/local/tmp/side-by-side-payload >/dev/null
adb -s "$serial" shell run-as dev.ashiato.collector cp /data/local/tmp/side-by-side-payload "$data_dir/files/outbox/pending.jsonl"
official_count="$(adb -s "$serial" shell run-as dev.ashiato.collector wc -l "$data_dir/files/outbox/pending.jsonl" | tr -d '\r' | awk '{print $1}')"
[[ "$official_count" == 1 ]] || { echo "正式版の未送信件数を置けない" >&2; exit 1; }

build_and_install "$dev_version"
packages="$(adb -s "$serial" shell pm list packages)"
grep -Fq 'package:dev.ashiato.collector' <<<"$packages"
grep -Fq 'package:dev.ashiato.collector.dev' <<<"$packages"
dev_count="$(adb -s "$serial" shell run-as dev.ashiato.collector wc -l "$data_dir/files/outbox/pending.jsonl" | tr -d '\r' | awk '{print $1}')"
[[ "$dev_count" == "$official_count" ]] || { echo "正式版の未送信記録が変わった" >&2; exit 1; }
echo "== Scenario: 開発版を入れても、正式版の端末内の記録は残る"
echo "OK: 正式版と開発版を同じ端末に入れ、正式版の未送信 $official_count 件を確認した"
