#!/usr/bin/env bash
# 共有の未送信置き場に 90 日たまった場合の容量を、UsageStatsManager の実物から見積もる（tasks 7.1）。
#
#   tools/usage-volume.sh                    # エミュレータ: 画面遷移を N 回流し込み、直近 1 時間を数える
#   tools/usage-volume.sh --device [serial]  # 実機: 取得元が既に持っている**直近 10 日の実イベント**を数える
#   USAGE_VOLUME_SAMPLE_FILE=<jsonl> tools/usage-volume.sh   # 採寸済みの行から計算だけ（試験用）
#
# 終了コード（deep.md の Q10。7.1 の「rc=1 なら本人に返す」を取り違えないため）:
#   0 = 上限（既定 1 GiB = 2 GB 上限の半分）に収まる
#   1 = **上限を超えた**（これだけ。Q1「種別を絞るか」の再問の合図）
#   2 = 測れなかった（端末・エミュレータ・ビルド・adb の失敗、観測が足りない）。上限とは無関係
#
# エミュレータの経路の件数は流し込みの回数 N で決まる（code-verify R31。本人の使い方の観測ではない）。
# 本人の使い方で判定するのは実機の経路（行き先 ST14: 実機で数えてしきい値を確定する）。
#
# 実機の経路は**本人の端末の状態を変えない**: アプリ本体は入れ直さず（入っている収集アプリをそのまま使う）、
# 「利用状況へのアクセス」も付けない（許されていなければ rc=2 で止まる）。入れるのは計測用の APK だけで、
# 終わったら外し、書いた採寸ファイルも消す。
set -uo pipefail
cd "$(dirname "$0")/.."

PKG=dev.ashiato.collector
TEST_CLASS=dev.ashiato.collector.UsageVolumeInstrumentedTest
RUNNER=dev.ashiato.collector.test/androidx.test.runner.AndroidJUnitRunner
DEVICE_WINDOW_SECONDS=864000   # 10 日。取得元がイベントを持っている長さ（design / deep.md Q10）

mode=emulator
device_serial=${USAGE_VOLUME_DEVICE:-}
[ -z "$device_serial" ] || mode=device
if [ "${1:-}" = "--device" ]; then
  mode=device
  device_serial=${2:-$device_serial}
elif [ $# -gt 0 ]; then
  echo "error: 引数は --device [serial] だけ" >&2
  exit 2
fi
[ -z "${USAGE_VOLUME_SAMPLE_FILE:-}" ] || mode=sample

verdict=$(mktemp)
trap 'rm -f "$verdict"' EXIT

# 本体は subshell で走らせ、**どこで落ちても rc=2** に寄せる（`set -e` / `set -u` / pipefail の失敗、
# adb の `device offline`、gradle の失敗）。上限の判定だけは rc ではなく $verdict に書く ——
# rc で運ぶと、たまたま同じ値を返した道具の失敗と区別できない。
main() {
  set -euo pipefail
  local limit=${USAGE_VOLUME_LIMIT_BYTES:-1073741824}
  local sample="" span="" workdir
  started=""   # 大域: EXIT の trap は main を抜けた後に走るので、local だと見えない
  workdir=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "cleanup_main '$workdir'" EXIT

  case "$mode" in
    sample)
      sample=$USAGE_VOLUME_SAMPLE_FILE
      span=${USAGE_VOLUME_SPAN_SECONDS:-3600}
      ;;
    emulator)
      emulator_sample "$workdir"
      sample=$workdir/usage-volume.jsonl
      span=3600
      ;;
    device)
      device_sample "$workdir"
      sample=$workdir/usage-volume.jsonl
      span=$(sed -n 's/^span_seconds=//p' "$workdir/usage-volume.meta")
      [ -n "$span" ] || { echo "error: 観測した長さ（span_seconds）が読めない" >&2; return 2; }
      # 1 日に満たない観測を 1 日へ引き伸ばすと、数分の偏りがそのまま 90 日の判定になる
      [ "$span" -ge 86400 ] || { echo "error: 端末が持っているイベントが 1 日に満たない（${span} 秒）。判定できない" >&2; return 2; }
      ;;
  esac

  [[ "$span" =~ ^[1-9][0-9]*$ ]] || { echo "error: 観測の長さは正の整数（秒）: $span" >&2; return 2; }
  [ -s "$sample" ] || { echo "error: 実測が 0 件" >&2; return 2; }
  local events total_bytes bytes_per_event events_per_hour events_per_day bytes_90_days
  events=$(wc -l < "$sample" | tr -d ' ')
  total_bytes=$(wc -c < "$sample" | tr -d ' ')
  bytes_per_event=$(( (total_bytes + events - 1) / events ))
  events_per_hour=$(( (events * 3600 + span - 1) / span ))
  events_per_day=$(( (events * 86400 + span - 1) / span ))
  bytes_90_days=$(( bytes_per_event * events_per_day * 90 ))

  printf 'mode=%s\n' "$mode"
  printf 'events=%s\n' "$events"
  printf 'span_seconds=%s\n' "$span"
  printf 'events_per_hour=%s\n' "$events_per_hour"
  printf 'bytes_per_event=%s\n' "$bytes_per_event"
  printf 'events_per_day=%s\n' "$events_per_day"
  printf 'bytes_90_days=%s\n' "$bytes_90_days"
  printf 'limit_bytes=%s\n' "$limit"

  if [ "$bytes_90_days" -le "$limit" ]; then
    printf 'within_limit=true\n'
    echo within > "$verdict"
  else
    printf 'within_limit=false\n'
    echo over > "$verdict"
  fi
}

cleanup_main() {
  [ -z "${started:-}" ] || adb -s "$started" emu kill >/dev/null 2>&1 || true
  rm -rf "$1"
}

build() {
  (cd collector-android && ./gradlew -q -Dkotlin.compiler.execution.strategy=in-process "$@")
}

# 採寸用の計測テストを 1 本だけ走らせ、書いた 2 つのファイルを $1 へ持ち帰る。
run_probe() {
  local serial=$1 dir=$2 window=$3 out
  adb -s "$serial" shell run-as "$PKG" rm -f files/usage-volume.jsonl files/usage-volume.meta >/dev/null 2>&1 || true
  # `am instrument -w` は試験が落ちても rc=0 を返す。結果の行で判定する
  out=$(adb -s "$serial" shell am instrument -w -e class "$TEST_CLASS" -e windowSeconds "$window" "$RUNNER") || return 2
  printf '%s\n' "$out" | grep -q '^OK (1 test)' \
    || { printf 'error: 採寸の計測テストが通らない:\n%s\n' "$out" >&2; return 2; }
  # 呼び出し側が `|| rc=$?` で包むと errexit が効かないので、1 つずつ明示して返す
  adb -s "$serial" exec-out run-as "$PKG" cat files/usage-volume.jsonl > "$dir/usage-volume.jsonl" || return 2
  adb -s "$serial" exec-out run-as "$PKG" cat files/usage-volume.meta > "$dir/usage-volume.meta" || return 2
  adb -s "$serial" shell run-as "$PKG" rm -f files/usage-volume.jsonl files/usage-volume.meta >/dev/null 2>&1 || true
}

emulator_sample() {
  local dir=$1 serial avd image n i
  serial=${SERIAL:-$(adb devices | awk '$1 ~ /^emulator-/ && $2 == "device" { print $1; exit }')}
  if [ -z "$serial" ]; then
    avd=${AVD:-ashiato-api35}
    image=system-images\;android-35\;google_apis\;x86_64
    [ -x "$ANDROID_HOME/emulator/emulator" ] || { echo "error: emulator が無い" >&2; return 2; }
    [ -d "$ANDROID_HOME/system-images/android-35/google_apis/x86_64" ] \
      || { echo "error: $image が無い" >&2; return 2; }
    if ! avdmanager list avd -c 2>/dev/null | grep -qx "$avd"; then
      echo no | avdmanager create avd -n "$avd" -k "$image" -d pixel_6 >/dev/null
    fi
    "$ANDROID_HOME/emulator/emulator" -avd "$avd" -wipe-data -no-window -gpu swiftshader_indirect -noaudio \
      -no-boot-anim -camera-back none -no-snapshot >/dev/null 2>&1 &
    serial=emulator-5554
    started=$serial
    tools/wait-android-boot.sh "$serial" \
      || { echo "error: エミュレータの起動が終わらない" >&2; return 2; }
  fi

  build :app:assembleDebug :app:assembleDebugAndroidTest -Pashiato.baseUrl=http://127.0.0.1:18787
  adb -s "$serial" install -r collector-android/app/build/outputs/apk/debug/app-debug.apk >/dev/null
  adb -s "$serial" install -r collector-android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk >/dev/null
  adb -s "$serial" shell appops set "$PKG" GET_USAGE_STATS allow

  n=${USAGE_VOLUME_EVENTS:-40}
  [ "$n" -gt 0 ] 2>/dev/null || { echo "error: USAGE_VOLUME_EVENTS は正の整数" >&2; return 2; }
  for i in $(seq 1 "$n"); do
    if [ $((i % 2)) -eq 0 ]; then
      adb -s "$serial" shell am start -W -a android.settings.SETTINGS >/dev/null
    else
      adb -s "$serial" shell am start -W -a android.intent.action.MAIN -c android.intent.category.HOME >/dev/null
    fi
  done
  run_probe "$serial" "$dir" 3600
  # この AVD を次に使う tools/android-emulator.sh へ「利用状況へのアクセス」の許可を持ち込まない
  # （code-verify R29: AVD の userdata は起動をまたいで残り、suite の中で UsageAccess / Retention が落ちた）
  adb -s "$serial" uninstall "$PKG.test" >/dev/null 2>&1 || true
  adb -s "$serial" uninstall "$PKG" >/dev/null 2>&1 || true
}

device_sample() {
  local dir=$1 serial=$device_serial mode_op
  if [ -z "$serial" ]; then
    # 実機（emulator- でないもの）がちょうど 1 台のときだけ選ぶ。取り違えて別の端末を数えない
    serial=$(adb devices | awk 'NR > 1 && $2 == "device" && $1 !~ /^emulator-/ { print $1 }')
    [ "$(printf '%s' "$serial" | grep -c .)" -eq 1 ] \
      || { echo "error: 実機がちょうど 1 台つながっていない（--device <serial> で選ぶ）: ${serial:-なし}" >&2; return 2; }
  fi
  [ "$(adb -s "$serial" get-state 2>/dev/null)" = device ] || { echo "error: $serial が使えない（adb get-state）" >&2; return 2; }
  adb -s "$serial" shell pm path "$PKG" | grep -q '^package:' \
    || { echo "error: $serial に収集アプリ（$PKG）が入っていない" >&2; return 2; }
  mode_op=$(adb -s "$serial" shell appops get "$PKG" GET_USAGE_STATS | tr -d '\r')
  printf '%s\n' "$mode_op" | grep -q 'GET_USAGE_STATS: allow' \
    || { echo "error: $serial で「利用状況へのアクセス」が許されていない（このスクリプトは付けない）: $mode_op" >&2; return 2; }

  build :app:assembleDebugAndroidTest
  adb -s "$serial" install -r collector-android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk >/dev/null
  local rc=0
  run_probe "$serial" "$dir" "$DEVICE_WINDOW_SECONDS" || rc=$?
  adb -s "$serial" uninstall "$PKG.test" >/dev/null 2>&1 || true
  return "$rc"
}

( main )
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "usage-volume: 測れなかった（rc=$rc）。上限の判定ではない" >&2
  exit 2
fi
case "$(cat "$verdict")" in
  within) exit 0 ;;
  over) exit 1 ;;
  *) echo "usage-volume: 判定が書かれていない" >&2; exit 2 ;;
esac
