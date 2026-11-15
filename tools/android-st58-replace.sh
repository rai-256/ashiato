#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ANDROID_HOME/emulator:$PATH"
if [[ "${1:-}" == "--self-test" ]]; then
  test_output=$'OK (1 test) restartsWithoutOpening\nOK (1 test) specialUseWaitsForBackgroundLocation\nOK (1 test) noLocationStillCollectsOtherSources\nOK (1 test) noNotificationPermissionStillRestarts\nOK (1 test) openingPromotesLocation\nOK (1 test) noDestinationDoesNotStart\nOK (1 test) disabledUpdateStaysDisabled'
  required=(restartsWithoutOpening specialUseWaitsForBackgroundLocation noLocationStillCollectsOtherSources noNotificationPermissionStillRestarts openingPromotesLocation noDestinationDoesNotStart disabledUpdateStaysDisabled)
  check_output() { for name in "${required[@]}"; do grep -qx "OK (1 test) $name" <<<"$1" || return 1; done; }
  check_output "$test_output"
  missing_output="$(printf '%s\n' "$test_output" | sed "/OK (1 test) ${required[0]}/d")"
  if check_output "$missing_output"; then echo 'self-test: 欠落検知に失敗' >&2; exit 1; fi
  ! check_output "$missing_output"
  grep -q 'class St58PackageReplacedInstrumentedTest' "$root/collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/St58PackageReplacedInstrumentedTest.kt"
  echo 'OK'
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
artifact_dir="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-st58-replace.XXXXXX")"
cleanup_artifacts() { rm -f "$artifact_dir"/*; rmdir "$artifact_dir"; }
trap cleanup_artifacts EXIT
adb -s "$serial" uninstall dev.ashiato.collector >/dev/null 2>&1 || true
old_apk="$artifact_dir/app-old.apk"
new_apk="$artifact_dir/app-new.apk"
(cd "$root/collector-android" && ASHIATO_BUILD_VERSION="${version}-beta.1" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q -Pashiato.baseUrl=http://127.0.0.1 -Pashiato.apiToken=test -Pashiato.userId=test :app:assembleDebug)
cp "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" "$old_apk"
(cd "$root/collector-android" && ASHIATO_BUILD_VERSION="$version" ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q -Pashiato.baseUrl=http://127.0.0.1 -Pashiato.apiToken=test -Pashiato.userId=test :app:assembleDebug)
cp "$root/collector-android/app/build/outputs/apk/debug/app-debug.apk" "$new_apk"
adb -s "$serial" install "$old_apk" >/dev/null
old_code="$(adb -s "$serial" shell dumpsys package dev.ashiato.collector | sed -n 's/.*versionCode=\([0-9]*\).*/\1/p' | head -1)"
[[ -n "$old_code" ]] || { echo '旧版の versionCode が読めない' >&2; exit 1; }
adb -s "$serial" shell pm grant dev.ashiato.collector android.permission.ACCESS_FINE_LOCATION || true
adb -s "$serial" shell pm grant dev.ashiato.collector android.permission.ACCESS_BACKGROUND_LOCATION || true
adb -s "$serial" shell pm grant dev.ashiato.collector android.permission.POST_NOTIFICATIONS || true
adb -s "$serial" install -r "$new_apk" >/dev/null
new_code="$(adb -s "$serial" shell dumpsys package dev.ashiato.collector | sed -n 's/.*versionCode=\([0-9]*\).*/\1/p' | head -1)"
[[ -n "$new_code" && "$old_code" != "$new_code" ]] || { echo "版が入れ替わっていない: $old_code -> $new_code" >&2; exit 1; }
(cd "$root/collector-android" && ./gradlew --no-daemon --no-build-cache --no-configuration-cache -q :app:assembleDebugAndroidTest)
adb -s "$serial" install -r "$root/collector-android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk" >/dev/null
adb -s "$serial" logcat -c
adb -s "$serial" shell monkey -p dev.ashiato.collector 1 >/dev/null
sleep 2
for name in restartsWithoutOpening specialUseWaitsForBackgroundLocation noLocationStillCollectsOtherSources noNotificationPermissionStillRestarts openingPromotesLocation noDestinationDoesNotStart disabledUpdateStaysDisabled; do
  case "$name" in
    specialUseWaitsForBackgroundLocation)
      adb -s "$serial" shell pm revoke dev.ashiato.collector android.permission.ACCESS_BACKGROUND_LOCATION >/dev/null 2>&1 || true ;;
    noLocationStillCollectsOtherSources)
      adb -s "$serial" shell pm revoke dev.ashiato.collector android.permission.ACCESS_FINE_LOCATION >/dev/null 2>&1 || true ;;
    noNotificationPermissionStillRestarts)
      adb -s "$serial" shell pm revoke dev.ashiato.collector android.permission.POST_NOTIFICATIONS >/dev/null 2>&1 || true ;;
    openingPromotesLocation)
      adb -s "$serial" shell pm grant dev.ashiato.collector android.permission.ACCESS_FINE_LOCATION >/dev/null 2>&1 || true
      adb -s "$serial" shell am start -n dev.ashiato.collector/.MainActivity >/dev/null
      adb -s "$serial" shell uiautomator dump /sdcard/st58-ui.xml >/dev/null 2>&1 || true
      adb -s "$serial" shell input keyevent 4 >/dev/null 2>&1 || true
      adb -s "$serial" emu geo fix 139.6917 35.6895 >/dev/null 2>&1 || true ;;
  esac
  if [[ -z "$(adb -s "$serial" shell pidof dev.ashiato.collector 2>/dev/null | tr -d '\r' || true)" ]]; then
    adb -s "$serial" shell monkey -p dev.ashiato.collector 1 >/dev/null
    for _ in $(seq 1 20); do
      [[ -n "$(adb -s "$serial" shell pidof dev.ashiato.collector 2>/dev/null | tr -d '\r' || true)" ]] && break
      sleep 1
    done
  fi
  adb -s "$serial" shell am stopservice -n dev.ashiato.collector/.LocationService >/dev/null 2>&1 || true
  before="$(adb -s "$serial" shell pidof dev.ashiato.collector 2>/dev/null | tr -d '\r' || true)"
  [[ -n "$before" ]] || { echo "対象プロセスが起きない: $name" >&2; exit 1; }
  set +e
  output="$(adb -s "$serial" shell am instrument --no-restart -w -e class "dev.ashiato.collector.St58PackageReplacedInstrumentedTest#$name" dev.ashiato.collector.test/androidx.test.runner.AndroidJUnitRunner 2>&1 | tr -d '\r')"
  instrument_rc=$?
  set -e
  if [[ "$instrument_rc" != 0 ]]; then
    printf 'instrument が失敗: %s\n%s\n' "$name" "$output" >&2
    exit 1
  fi
  grep -q 'OK (1 test)' <<<"$output" || { printf '試験が失敗: %s\n%s\n' "$name" "$output" >&2; exit 1; }
  sleep 2
  after="$(adb -s "$serial" shell pidof dev.ashiato.collector 2>/dev/null | tr -d '\r' || true)"
  [[ -z "$before" || -z "$after" || "$before" == "$after" ]] || { echo "プロセスが入れ替わった: $name" >&2; exit 1; }
  case "$name" in
    specialUseWaitsForBackgroundLocation|noLocationStillCollectsOtherSources)
      adb -s "$serial" logcat -d -v brief | grep -q 'foreground_special_use' || { echo "specialUse のログが無い: $name" >&2; exit 1; } ;;
    openingPromotesLocation)
      adb -s "$serial" logcat -d -v brief | grep -q 'foreground_location' || { echo 'location への昇格ログが無い' >&2; exit 1; }
      ! adb -s "$serial" shell dumpsys notification | grep -q 'id=12' || { echo '通知 ID 12 が残っている' >&2; exit 1; } ;;
  esac
  echo "OK (1 test) $name"
done
