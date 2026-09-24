#!/usr/bin/env bash
# エミュレータへ画面遷移を N 回流し、UsageStatsManager から直近 1 時間を取得して、
# 共有の未送信置き場に 90 日たまった場合の容量を実測から見積もる。
set -euo pipefail
cd "$(dirname "$0")/.."

limit=${USAGE_VOLUME_LIMIT_BYTES:-1073741824}
sample=${USAGE_VOLUME_SAMPLE_FILE:-}
started=""
generated_sample=""

cleanup() {
  [ -z "$started" ] || adb -s "$started" emu kill >/dev/null 2>&1 || true
  [ -z "$generated_sample" ] || rm -f "$generated_sample"
}
trap cleanup EXIT

if [ -z "$sample" ]; then
  # shellcheck disable=SC1091
  . ./tools/android-env.sh
  export PATH="$ANDROID_HOME/emulator:$PATH"
  serial=${SERIAL:-$(adb devices | awk '$1 ~ /^emulator-/ && $2 == "device" { print $1; exit }')}
  if [ -z "$serial" ]; then
    avd=${AVD:-ashiato-api35}
    image=system-images\;android-35\;google_apis\;x86_64
    [ -x "$ANDROID_HOME/emulator/emulator" ] || { echo "error: emulator が無い" >&2; exit 2; }
    [ -d "$ANDROID_HOME/system-images/android-35/google_apis/x86_64" ] \
      || { echo "error: $image が無い" >&2; exit 2; }
    if ! avdmanager list avd -c 2>/dev/null | grep -qx "$avd"; then
      echo no | avdmanager create avd -n "$avd" -k "$image" -d pixel_6 >/dev/null
    fi
    emulator -avd "$avd" -wipe-data -no-window -gpu swiftshader_indirect -noaudio \
      -no-boot-anim -camera-back none -no-snapshot >/dev/null 2>&1 &
    serial=emulator-5554
    started=$serial
    adb -s "$serial" wait-for-device
    for _ in $(seq 1 180); do
      [ "$(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ] && break
      sleep 2
    done
    [ "$(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ] \
      || { echo "error: エミュレータの起動が終わらない" >&2; exit 2; }
  fi

  (cd collector-android && ./gradlew -q -Dkotlin.compiler.execution.strategy=in-process \
    :app:assembleDebug :app:assembleDebugAndroidTest -Pashiato.baseUrl=http://127.0.0.1:18787)
  adb -s "$serial" install -r collector-android/app/build/outputs/apk/debug/app-debug.apk >/dev/null
  adb -s "$serial" install -r collector-android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk >/dev/null
  adb -s "$serial" shell appops set dev.ashiato.collector GET_USAGE_STATS allow

  n=${USAGE_VOLUME_EVENTS:-40}
  [ "$n" -gt 0 ] 2>/dev/null || { echo "error: USAGE_VOLUME_EVENTS は正の整数" >&2; exit 2; }
  for i in $(seq 1 "$n"); do
    if [ $((i % 2)) -eq 0 ]; then
      adb -s "$serial" shell am start -W -a android.settings.SETTINGS >/dev/null
    else
      adb -s "$serial" shell am start -W -a android.intent.action.MAIN -c android.intent.category.HOME >/dev/null
    fi
  done
  # 前回の計測ファイルを残したままにすると、計測テストが落ちても古い値で緑になってしまう。
  adb -s "$serial" shell run-as dev.ashiato.collector rm files/usage-volume.jsonl >/dev/null 2>&1 || true
  adb -s "$serial" shell am instrument -w \
    -e class dev.ashiato.collector.UsageVolumeInstrumentedTest \
    dev.ashiato.collector.test/androidx.test.runner.AndroidJUnitRunner >/dev/null
  sample=$(mktemp)
  generated_sample=$sample
  adb -s "$serial" exec-out run-as dev.ashiato.collector cat files/usage-volume.jsonl > "$sample"
fi

[ -s "$sample" ] || { echo "error: 1 時間ぶんの実測が 0 件" >&2; exit 2; }
events_per_hour=$(wc -l < "$sample" | tr -d ' ')
total_bytes=$(wc -c < "$sample" | tr -d ' ')
bytes_per_event=$(( (total_bytes + events_per_hour - 1) / events_per_hour ))
events_per_day=$(( events_per_hour * 24 ))
bytes_90_days=$(( bytes_per_event * events_per_day * 90 ))

printf 'events_per_hour=%s\n' "$events_per_hour"
printf 'bytes_per_event=%s\n' "$bytes_per_event"
printf 'events_per_day=%s\n' "$events_per_day"
printf 'bytes_90_days=%s\n' "$bytes_90_days"
printf 'limit_bytes=%s\n' "$limit"

[ "$bytes_90_days" -le "$limit" ] || exit 1
