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
  : > "$TMP/null-source"
  : > "$TMP/kvm-fallback"
  export EMU_LOG="$log"
  set +e
  timeout 30 env EMU_LOG="$log" HOME="$HOME" ANDROID_AVD_HOME="$ANDROID_AVD_HOME" \
    ANDROID_ADB_SERVER_PORT="$ANDROID_ADB_SERVER_PORT" \
    unshare -rm bash -c 'mount --bind /dev/null "$2" && mount --make-rprivate /dev && mount -t tmpfs tmpfs /dev && ln -s /proc/self/fd /dev/fd && ln -s /proc/self/stdin /dev/stdin && ln -s /proc/self/stdout /dev/stdout && ln -s /proc/self/stderr /dev/stderr && : > /dev/kvm && mount --bind "$2" /dev/kvm && (exec 3<>/dev/kvm) 2>/dev/null || { umount /dev/kvm; mount --bind "$3" /dev/kvm; } && exec "$1"' bash \
    "$ROOT/tools/android-emulator.sh" "$TMP/null-source" "$TMP/kvm-fallback" >"$out" 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 1 ] || die "dies の rc が $rc"
  grep -Fq 'error: エミュレータが起動の途中で終わった' "$out" || die "dies の終了理由が無い"
  grep -Fq '/dev/kvm' "$out" || die "dies の /dev/kvm の語が無い"
}

case_timeout() {
  local out="$TMP/timeout.out" pid rc remaining
  prepare_env "$((20000 + $$ % 1000))"
  export ASHIATO_EMULATOR_DEVICE_TIMEOUT=1
  set +e
  timeout 30 "$ROOT/tools/android-emulator.sh" >"$out" 2>&1 &
  pid=$!
  wait "$pid"
  rc=$?
  set +e
  sleep 10
  # 角括弧で pgrep 自身（引数に検索文字列を持つ）を数えない。
  remaining=$(pgrep -f -- '[-]avd ashiato-api35' | wc -l)
  [ "$rc" -eq 1 ] || die "timeout の rc が $rc"
  grep -Fq '1 秒待っても端末が adb に現れない' "$out" || die "timeout の終了理由が無い"
  if kill -0 "$pid" 2>/dev/null; then
    die "timeout の台本 PID が残っている"
  fi
  [ "$remaining" -eq 0 ] || die "timeout 後の -avd 過程が $remaining 本残っている"
  return 0
}

case_same_avd() {
  local log="$ROOT/collector-android/app/build/outputs/emulator.log"
  local out="$TMP/same-avd.out" serial= name pid rc accel
  prepare_env "$((21000 + $$ % 1000))"
  # emulator は ANDROID_ADB_SERVER_PORT を引き継がず既定の adb server に接続する。
  unset ANDROID_ADB_SERVER_PORT
  # 既存の実 AVD を使い、AVD 作成・初回 userdata 展開を same-avd の 30 秒監視から外す。
  export ANDROID_AVD_HOME="$ORIGINAL_HOME/.android/avd"
  ensure_avd
  mkdir -p "$(dirname "$log")"
  : > "$log"
  "$ANDROID_HOME/emulator/emulator" -avd ashiato-api35 -no-window \
    -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none \
    -no-snapshot >"$log" 2>&1 &
  pid=$!
  trap 'if [ -n "$serial" ]; then adb -s "$serial" emu kill >/dev/null 2>&1 || true; fi; kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' RETURN
  for _ in $(seq 1 60); do
    serial=$(adb devices 2>/dev/null | awk '$1 ~ /^emulator-/ && $2 == "device" { print $1; exit }')
    if [ -n "$serial" ]; then
      break
    fi
    sleep 1
  done
  [ -n "$serial" ] || die "same-avd の端末が adb に現れない"
  for _ in $(seq 1 60); do
    name=$(timeout 1 adb -s "$serial" shell getprop ro.boot.qemu.avd_name 2>/dev/null | sed -n '1p')
    [ -n "$name" ] && break
    sleep 1
  done
  if [ -z "$name" ]; then
    name=$(timeout 1 adb -s "$serial" emu avd name 2>/dev/null | sed -n '1p')
  fi
  printf 'same-avd-name: %s\n' "$name" >> "$PROOF"
  if [ "$name" != ashiato-api35 ]; then
    die "AVD 名が $name"
    return 1
  fi
  set +e
  timeout 30 "$ROOT/tools/android-emulator.sh" >"$out" 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 1 ] || die "same-avd の rc が $rc"
  grep -Fq 'error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている' "$out" \
    || die "same-avd の終了理由が無い"
  accel=$(count_accel_lines "$log")
  printf 'same-avd-accel-lines: %s\n' "$accel" >> "$PROOF"
  [ "$accel" -eq 0 ] || die "same-avd の加速不足文言が $accel 行ある"
  trap - RETURN
  adb -s "$serial" emu kill >/dev/null 2>&1 || true
  wait "$pid" 2>/dev/null || true
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
