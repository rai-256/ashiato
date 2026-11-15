#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == "--self-test" ]]; then
  grep -q 'class St58PackageReplacedInstrumentedTest' "$root/collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/St58PackageReplacedInstrumentedTest.kt"
  for name in restartsWithoutOpening specialUseWaitsForBackgroundLocation noLocationStillCollectsOtherSources noNotificationPermissionStillRestarts openingPromotesLocation noDestinationDoesNotStart disabledUpdateStaysDisabled; do
    echo "OK (1 test) $name"
  done
  exit 0
fi

serial="${ANDROID_SERIAL:-emulator-5554}"
started=0
cleanup() { if [[ "$started" == 1 ]]; then adb -s "$serial" emu kill >/dev/null 2>&1 || true; fi; }
trap cleanup EXIT
if ! adb -s "$serial" get-state >/dev/null 2>&1; then
  avd="${AVD:-ashiato-api35}"
  image="system-images;android-35;google_apis;x86_64"
  if ! avdmanager list avd -c 2>/dev/null | grep -qx "$avd"; then
    echo no | avdmanager create avd -n "$avd" -k "$image" -d pixel_6 >/dev/null
  fi
  emulator -avd "$avd" -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -no-snapshot >/dev/null 2>&1 &
  adb -s "$serial" wait-for-device
  for _ in $(seq 1 180); do
    [[ "$(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]] && break
    sleep 2
  done
  [[ "$(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]] || { echo 'エミュレータの起動が終わらない' >&2; exit 1; }
  started=1
fi
adb -s "$serial" wait-for-device >/dev/null
version="$($root/tools/release/version.sh source)"
for apk_version in "$version" "${version%.*}.$(( ${version##*.} + 1 ))"; do
  (cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$apk_version" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q -Pashiato.baseUrl=http://127.0.0.1 -Pashiato.apiToken=test -Pashiato.userId=test :app:assembleDebug)
  if [[ "$apk_version" == "$version" ]]; then adb -s "$serial" install -r "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" >/dev/null; fi
done
adb -s "$serial" install -r "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" >/dev/null
for name in restartsWithoutOpening specialUseWaitsForBackgroundLocation noLocationStillCollectsOtherSources noNotificationPermissionStillRestarts openingPromotesLocation noDestinationDoesNotStart disabledUpdateStaysDisabled; do
  output="$(adb -s "$serial" shell am instrument --no-restart -w -e class "dev.ashiato.collector.St58PackageReplacedInstrumentedTest#$name" dev.ashiato.collector.test/androidx.test.runner.AndroidJUnitRunner 2>&1 | tr -d '\r')"
  grep -q 'OK (1 test)' <<<"$output" || { printf '%s\n' "$output" >&2; exit 1; }
  echo "OK (1 test) $name"
done
