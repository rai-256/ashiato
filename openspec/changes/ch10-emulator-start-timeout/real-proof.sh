#!/usr/bin/env bash
# CH10 Task 6: 本物の SDK/KVM で起動失敗・再利用・正常経路を確かめる。
set -u -o pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
CHANGE_DIR="$ROOT/openspec/changes/ch10-emulator-start-timeout"
PROOF="$CHANGE_DIR/real-proof.txt"
CASE=${1:-}
TMP=${REAL_PROOF_TMP:-}
SDK_SOURCE=${ANDROID_HOME:-}
JDK_SOURCE=${JAVA_HOME:-}
ORIGINAL_HOME=${HOME:?}
SAME_SERIAL=
SAME_PID=
SAME_PORT=

signal_descendants() {
  local signal=$1 parent=$2 child
  while read -r child; do
    [ -n "$child" ] || continue
    signal_descendants "$signal" "$child"
    kill -"$signal" "$child" 2>/dev/null || true
  done < <(pgrep -P "$parent" 2>/dev/null || true)
}

cleanup_same_avd() {
  local rc=$?
  if [ -n "$SAME_SERIAL" ]; then
    adb -s "$SAME_SERIAL" emu kill >/dev/null 2>&1 || true
  fi
  if [ -n "$SAME_PID" ]; then
    signal_descendants TERM "$SAME_PID"
    kill -TERM "$SAME_PID" 2>/dev/null || true
    wait "$SAME_PID" 2>/dev/null || true
    signal_descendants KILL "$SAME_PID"
    kill -KILL "$SAME_PID" 2>/dev/null || true
    wait "$SAME_PID" 2>/dev/null || true
  fi
  cleanup_tmp
  return "$rc"
}

die() {
  echo "error: $*" >&2
  return 1
}

count_accel_lines() {
  local log=$1
  [ -f "$log" ] || { echo 0; return 0; }
  grep -Ec 'requires hardware acceleration|permissions to use KVM' "$log" || true
}

prepare_env() {
  local port=$1
  [ -n "$SDK_SOURCE" ] || { echo 'error: ANDROID_HOME が設定されていない' >&2; return 1; }
  mkdir -p "$TMP/home" "$TMP/avd"
  mkdir -p "$TMP/home/.local/opt"
  ln -sfn "$SDK_SOURCE" "$TMP/home/.local/opt/android-sdk"
  [ -z "$JDK_SOURCE" ] || ln -sfn "$JDK_SOURCE" "$TMP/home/.local/opt/jdk21"
  export HOME="$TMP/home"
  export ANDROID_AVD_HOME="$TMP/avd"
  export ANDROID_ADB_SERVER_PORT="$port"
  export PATH="$ANDROID_HOME/emulator:$PATH"
}

ensure_avd() {
  if ! avdmanager list avd -c 2>/dev/null | grep -qx ashiato-api35; then
    echo no | avdmanager create avd -n ashiato-api35 \
      -k 'system-images;android-35;google_apis;x86_64' -d pixel_6 >/dev/null
  fi
}

case_dies() {
  local log="$TMP/dies-emulator.log" out="$TMP/dies.out" rc
  prepare_env "$((19000 + $$ % 1000))"
  export EMU_LOG="$log"
  set +e
  timeout 30 env EMU_LOG="$log" HOME="$HOME" ANDROID_AVD_HOME="$ANDROID_AVD_HOME" \
    ANDROID_ADB_SERVER_PORT="$ANDROID_ADB_SERVER_PORT" \
    unshare -rm bash -c 'mount --bind /dev/null /dev/kvm && exec "$1"' bash \
    "$ROOT/tools/android-emulator.sh" >"$out" 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 1 ] || { die "dies の rc が $rc"; return 1; }
  grep -Fq 'error: エミュレータが起動の途中で終わった' "$out" || { die "dies の終了理由が無い"; return 1; }
  grep -Fq 'error: エミュレータが /dev/kvm' "$out" || { die "dies の D17 の終了理由が無い"; return 1; }
  grep -Fq '/dev/kvm' "$out" || { die "dies の /dev/kvm の語が無い"; return 1; }
  printf 'dies-kvm-form: mount --bind /dev/null /dev/kvm\n' >> "$PROOF"
}

case_timeout() {
  local out="$TMP/timeout.out" wrapper_pid script_pid= emu_pid= rc remaining pid baseline_pids args
  prepare_env "$((20000 + $$ % 1000))"
  export ASHIATO_EMULATOR_DEVICE_TIMEOUT=1
  # 並走する実 emulator をこの回の残りと数えない。
  baseline_pids=$(pgrep -f -- '[-]avd ashiato-api35' || true)
  set +e
  timeout 30 "$ROOT/tools/android-emulator.sh" >"$out" 2>&1 &
  wrapper_pid=$!
  for _ in 1 2 3 4 5; do
    script_pid=$(pgrep -P "$wrapper_pid" -n 2>/dev/null || true)
    if [ -n "$script_pid" ]; then
      for pid in $(pgrep -P "$script_pid" 2>/dev/null || true); do
        args=$(ps -o args= -p "$pid" 2>/dev/null || true)
        if [[ "$args" == *"-avd ashiato-api35"* ]]; then
          emu_pid=$pid
          break 2
        fi
      done
    fi
    sleep 1
  done
  wait "$wrapper_pid"
  rc=$?
  set -e
  sleep 10
  # emulator の qemu 子が終了処理を完了するまで、上限の確認後に少しだけ待つ。
  sleep 5
  # 角括弧で pgrep 自身（引数に検索文字列を持つ）を数えない。
  remaining=0
  while read -r pid; do
    [ -n "$pid" ] || continue
    grep -qxF "$pid" <<<"$baseline_pids" || remaining=$((remaining + 1))
  done < <(pgrep -f -- '[-]avd ashiato-api35' || true)
  [ "$rc" -eq 1 ] || { die "timeout の rc が $rc"; return 1; }
  grep -Fq '1 秒待っても端末が adb に現れない' "$out" || { die "timeout の終了理由が無い"; return 1; }
  [ -n "$emu_pid" ] || { die "timeout の台本が出した EMU_PID を取得できない"; return 1; }
  if kill -0 "$emu_pid" 2>/dev/null; then
    # zombie（stat が Z で始まる）は終わったものとして数えない。`[ … != Z* ]` は文字列の比較で除けなかった
    case "$(ps -o stat= -p "$emu_pid" 2>/dev/null | tr -d ' ')" in
      Z*) ;;
      *) die "timeout の台本が出した EMU_PID が残っている"; return 1 ;;
    esac
  fi
  [ "$remaining" -eq 0 ] || { die "timeout 後の -avd 過程が $remaining 本残っている"; return 1; }
  return 0
}

case_same_avd() {
  local log="$ROOT/collector-android/app/build/outputs/emulator.log"
  local out="$TMP/same-avd.out" name raw_name= rc accel same_port
  prepare_env "$((21000 + $$ % 1000))"
  # 実 AVD と既定 adb server を使う。検証用の専用 server では emulator が見えない。
  export HOME="$ORIGINAL_HOME"
  export ANDROID_AVD_HOME="$ORIGINAL_HOME/.android/avd"
  unset ANDROID_ADB_SERVER_PORT
  same_port=$((5600 + RANDOM % 400))
  while pgrep -f -- "[-]port $same_port" >/dev/null 2>&1; do
    same_port=$((same_port + 1))
  done
  SAME_PORT=$same_port
  ensure_avd
  mkdir -p "$(dirname "$log")"
  : > "$log"
  "$ANDROID_HOME/emulator/emulator" -avd ashiato-api35 -no-window \
    -port "$same_port" -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none \
    -no-snapshot >"$log" 2>&1 &
  SAME_PID=$!
  trap cleanup_same_avd EXIT
  for _ in $(seq 1 60); do
    SAME_SERIAL=$(adb devices 2>/dev/null | awk -v expected="emulator-$same_port" '$1 == expected && $2 == "device" { print $1; exit }')
    if [ -n "$SAME_SERIAL" ]; then
      break
    fi
    sleep 1
  done
  [ -n "$SAME_SERIAL" ] || { die "same-avd の端末が adb に現れない"; return 1; }
  for _ in $(seq 1 60); do
    name=$(timeout 1 adb -s "$SAME_SERIAL" shell getprop ro.boot.qemu.avd_name 2>/dev/null | tr -d '\r' | sed -n '1p')
    [ -n "$name" ] && break
    sleep 1
  done
  if [ -z "$name" ]; then
    name=$(timeout 1 adb -s "$SAME_SERIAL" emu avd name 2>/dev/null | tr -d '\r' | sed -n '1p')
  fi
  raw_name=$(timeout 1 adb -s "$SAME_SERIAL" emu avd name 2>/dev/null | tr -d '\r' | sed -n '1p')
  printf 'same-avd-name: %s\n' "$raw_name" >> "$PROOF"
  printf 'same-avd-getprop-name: %s\n' "$name" >> "$PROOF"
  [ "$name" = ashiato-api35 ] || { die "AVD 名が $name"; return 1; }
  set +e
  timeout 30 "$ROOT/tools/android-emulator.sh" >"$out" 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 1 ] || { die "same-avd の rc が $rc"; return 1; }
  grep -Fq 'error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている' "$out" \
    || { die "same-avd の終了理由が無い"; return 1; }
  accel=$(count_accel_lines "$log")
  printf 'same-avd-accel-lines: %s\n' "$accel" >> "$PROOF"
  [ "$accel" -eq 0 ] || { die "same-avd の加速不足文言が $accel 行ある"; return 1; }
}

case_ok() {
  local log="$ROOT/collector-android/app/build/outputs/emulator.log" out="$TMP/ok.out" rc accel stage
  prepare_env "$((22000 + $$ % 1000))"
  unset ANDROID_ADB_SERVER_PORT
  export EMU_LOG="$log"
  set +e
  timeout 30m "$ROOT/tools/android-emulator.sh" >"$out" 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 0 ] || die "ok の rc が $rc"
  for stage in 1 2 3 4 5 6; do
    find "$ROOT/collector-android/app/build/outputs/androidTest-results-stage$stage" \
      -type f -name '*.xml' -print -quit 2>/dev/null | grep -q . \
      || die "stage$stage の XML が無い"
  done
  accel=$(count_accel_lines "$log")
  printf 'ok-accel-lines: %s\n' "$accel" >> "$PROOF"
  [ "$accel" -eq 0 ] || die "ok の加速不足文言が $accel 行ある"
}

run_case() {
  case "$1" in
    dies) case_dies ;;
    timeout) case_timeout ;;
    same-avd) case_same_avd ;;
    ok) case_ok ;;
    *) echo "使い方: $0 {dies|timeout|same-avd|ok}" >&2; return 2 ;;
  esac
}

cleanup_tmp() {
  local rc=$? attempt
  for attempt in $(seq 1 30); do
    rm -rf "$TMP" 2>/dev/null && { return "$rc"; }
    sleep 1
  done
  echo "warning: 一時 AVD の後片付けが完了しない: $TMP" >&2
  return "$rc"
}

if [ "$CASE" = __internal ]; then
  TMP=${REAL_PROOF_TMP:?}
  run_case "${2:-}"
  exit $?
fi

case "$CASE" in
  dies|timeout|same-avd|ok) ;;
  *) echo "使い方: $0 {dies|timeout|same-avd|ok}" >&2; exit 2 ;;
esac

TMP=$(mktemp -d)
export REAL_PROOF_TMP="$TMP"
trap cleanup_tmp EXIT
mkdir -p "$CHANGE_DIR"
sed -i -e "/^${CASE}:/d" -e "/^${CASE}-/d" "$PROOF" 2>/dev/null || true
set +e
if [ "$CASE" = ok ]; then
  timeout 30m "$0" __internal "$CASE" >"$TMP/runner.out" 2>&1
elif [ "$CASE" = same-avd ]; then
  # 一時 AVD の cold boot 準備は 30 秒を越えることがある。本体の拒否経路は case 内の timeout 30 で監視する。
  timeout 90 "$0" __internal "$CASE" >"$TMP/runner.out" 2>&1
else
  timeout 30 "$0" __internal "$CASE" >"$TMP/runner.out" 2>&1
fi
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
  printf '%s: pass\n' "$CASE" >> "$PROOF"
else
  printf '%s: fail\n' "$CASE" >> "$PROOF"
  cat "$TMP/runner.out" >&2
fi
exit "$rc"
