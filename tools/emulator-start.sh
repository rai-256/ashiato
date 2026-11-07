#!/usr/bin/env bash
# エミュレータの起動と、adb に現れて起動完了するまでの待ちを共有する部品（CH10）。
#
# source tools/emulator-start.sh の後、次の関数を呼ぶ:
#   emu_check_env; emu_check_kvm; emu_start <AVD> <引数…>; emu_wait_device [serial]
#   emu_wait_boot [serial]; emu_fail <文言>; emu_stop_own
#
# 環境変数                 既定       用途
# ASHIATO_KVM_DEVICE       /dev/kvm   起動前に読み書きで開く場所
# ASHIATO_EMULATOR_DEVICE_TIMEOUT 120 adb に端末が現れるまでの秒
# ASHIATO_EMULATOR_BOOT_ATTEMPTS  180 起動完了を調べる回数（2 秒ごと）
# ASHIATO_ADB_CALL_TIMEOUT 10        起動完了・設定の adb shell 1 回の秒
#
# 起動直後に終わる・端末が現れない経路で無期限に待たず、理由とログを残すために使う。

emu_positive_int() {
  local name=$1 value=$2
  [[ "$value" =~ ^[1-9][0-9]*$ ]] || {
    echo "error: $name は正の整数でなければならない" >&2
    return 1
  }
}

# 3 つの上限の値を、エミュレータを起こす前（AVD を作る前）に確かめる。起こしてから落ちて残さないため（final R1）
emu_check_env() {
  emu_positive_int ASHIATO_EMULATOR_DEVICE_TIMEOUT "${ASHIATO_EMULATOR_DEVICE_TIMEOUT:-120}" || return 1
  emu_positive_int ASHIATO_EMULATOR_BOOT_ATTEMPTS "${ASHIATO_EMULATOR_BOOT_ATTEMPTS:-180}" || return 1
  emu_positive_int ASHIATO_ADB_CALL_TIMEOUT "${ASHIATO_ADB_CALL_TIMEOUT:-10}" || return 1
}

emu_pid_alive() {
  local stat
  [ -n "${EMU_PID:-}" ] || return 1
  kill -0 "$EMU_PID" 2>/dev/null || return 1
  stat=$(ps -o stat= -p "$EMU_PID" 2>/dev/null || true)
  [[ "$stat" == Z* ]] && return 1
  return 0
}

emu_check_kvm() {
  local device=${ASHIATO_KVM_DEVICE:-/dev/kvm}
  if [ ! -e "$device" ] || ! ( exec 3<>"$device" ) 2>/dev/null; then
    echo "error: /dev/kvm を開けない（調べた場所: $device）。エミュレータを起こさない。verify-run で走らせ直す（Codex なら AGENTS.md の「/dev/kvm」の手順で sandbox の外を申請する）" >&2
    return 1
  fi
}

emu_signal_descendants() {
  local signal=$1 parent child
  parent=$2
  while read -r child; do
    [ -n "$child" ] || continue
    emu_signal_descendants "$signal" "$child"
    kill -"$signal" "$child" 2>/dev/null || true
  done < <(pgrep -P "$parent" 2>/dev/null || true)
}

emu_start() {
  local avd=$1
  shift
  EMU_LOG=${EMU_LOG:-collector-android/app/build/outputs/emulator.log}
  mkdir -p "$(dirname "$EMU_LOG")"
  echo "== エミュレータを起動（画面なし）"
  "$ANDROID_HOME/emulator/emulator" -avd "$avd" "$@" >"$EMU_LOG" 2>&1 &
  EMU_PID=$!
}

# 自分の起こした PID とその子孫に TERM、5 秒たっても生きていれば KILL。EMU_PID が無い・終わっていれば何もしない
emu_stop_own() {
  local deadline
  emu_pid_alive || return 0
  emu_signal_descendants TERM "$EMU_PID"
  kill -TERM "$EMU_PID" 2>/dev/null || true
  deadline=$((SECONDS + 5))
  while emu_pid_alive && [ "$SECONDS" -lt "$deadline" ]; do sleep 1; done
  if emu_pid_alive; then
    emu_signal_descendants KILL "$EMU_PID"
    kill -KILL "$EMU_PID" 2>/dev/null || true
  fi
  wait "$EMU_PID" 2>/dev/null || true
  # bash は終わった背景の job を自分で回収するので、PID は空いて別のプロセスに使われうる。止めた後は持たない
  EMU_PID=
}

emu_fail() {
  local message=$1 log=${EMU_LOG:-collector-android/app/build/outputs/emulator.log}
  echo "error: $message" >&2
  if [ -n "${EMU_PID:-}" ] && ! emu_pid_alive; then
    wait "$EMU_PID" 2>/dev/null || true
    EMU_PID=
  fi
  if [ -f "$log" ]; then
    tail -n 40 "$log" >&2
    echo "error: エミュレータの出力全文: $log" >&2
    if grep -Eq 'requires hardware acceleration|permissions to use KVM' "$log"; then
      echo "error: エミュレータが /dev/kvm（ハードウェアの加速）を使えなかった。verify-run で走らせ直す（Codex なら AGENTS.md の「/dev/kvm」の手順）" >&2
    fi
  fi
  emu_stop_own
}

emu_wait_device() {
  local serial=${1:-} limit=${ASHIATO_EMULATOR_DEVICE_TIMEOUT:-120} state deadline call_timeout
  emu_positive_int ASHIATO_EMULATOR_DEVICE_TIMEOUT "$limit" || { emu_stop_own; return 1; }
  echo "== 端末が adb に現れるのを最大 $limit 秒待つ"
  # 回数でなく締め切りで数える。adb get-state が遅くても上限を大きく超えない（final R4）
  deadline=$((SECONDS + limit))
  while :; do
    if ! emu_pid_alive; then
      emu_fail 'エミュレータが起動の途中で終わった'
      return 1
    fi
    call_timeout=$((deadline - SECONDS))
    [ "$call_timeout" -le 5 ] || call_timeout=5
    [ "$call_timeout" -ge 1 ] || call_timeout=1
    if [ -n "$serial" ]; then
      state=$(timeout "$call_timeout" adb -s "$serial" get-state 2>/dev/null || true)
    else
      state=$(timeout "$call_timeout" adb get-state 2>/dev/null || true)
    fi
    [ "$state" = device ] && return 0
    [ "$SECONDS" -lt "$deadline" ] || break
    sleep 1
  done
  emu_fail "$limit 秒待っても端末が adb に現れない"
  return 1
}

emu_wait_boot() {
  local serial=${1:-} attempts=${ASHIATO_EMULATOR_BOOT_ATTEMPTS:-180} call_timeout=${ASHIATO_ADB_CALL_TIMEOUT:-10} state i
  emu_positive_int ASHIATO_EMULATOR_BOOT_ATTEMPTS "$attempts" || { emu_stop_own; return 1; }
  emu_positive_int ASHIATO_ADB_CALL_TIMEOUT "$call_timeout" || { emu_stop_own; return 1; }
  # timing-ok: 180 回 × 2 秒で、混雑時に長くなる側へだけ外れるエミュレータ起動待ち
  echo "== 起動の完了を最大 $attempts 回（2 秒ごと。1 回の問い合わせは最大 $call_timeout 秒）待つ"
  for ((i=0; i<attempts; i++)); do
    if ! emu_pid_alive; then
      emu_fail 'エミュレータが起動の途中で終わった'
      return 1
    fi
    if [ -n "$serial" ]; then
      state=$(timeout "$call_timeout" adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
    else
      state=$(timeout "$call_timeout" adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
    fi
    [ "$state" = 1 ] && return 0
    [ "$i" -lt $((attempts - 1)) ] && sleep 2
  done
  emu_fail '起動が終わらない'
  return 1
}
