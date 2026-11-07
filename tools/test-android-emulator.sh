#!/usr/bin/env bash
# android-emulator.sh の正常経路と、起動の失敗経路を実 SDK・実端末なしで固定する試験（CH10）。
# 鍵 | Scenario
# args | 正常に立つとき、今と同じ引数でエミュレータを起こす
# stages | 正常に最後まで走ると、6 段の結果を残してエミュレータを止める
# keep | KEEP=1 なら、通った後にエミュレータを止めない
# missing | emulator かシステムイメージが無いときは、今の文言で止まる
# avd-create | AVD が無ければ作ってから起こす
# usage-shape | 計測テストの台本の行の形の検査は、今のまま通る
# kvm-missing | 開けない場所を渡すと、エミュレータを起こさずに /dev/kvm の文言で止まる
# kvm-denied | 在っても読み書きできない場所を渡しても、同じく起こさずに止まる
# kvm-sign | /dev/kvm を開けないときの文言は、verify-run の環境の欠落の判定に掛かる
# dies | 起こしたエミュレータが起動直後に終わると、すぐ止まって出力の末尾を出す
# tail40 | 標準エラーに出す出力の末尾は 40 行まで
# late-die | 端末が見えた後、起動の完了を待つ間に終わっても、すぐ止まる
# device-default | 上限を渡さなければ、端末を最大 120 秒待つ
# bad-value | 上限に正の整数でない値を渡すと、起こさずに止まる
# device-timeout | 生きたまま端末が現れなければ、上限で止まる
# getprop-hang | 起動の完了の問い合わせが返らなくても、回数の上限で止まる
# keep-cleanup | 上限で止まったとき、KEEP=1 でも自分の起こしたプロセスは残らない
# no-named-kill | 起動の途中で終わったとき、名指しの端末を止めない
# same-avd | 同じ AVD が既に動いていると、起こさず・止めずに理由つきで止まる
# other-device | 実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす
# usage-dies | 採寸の台本で起こしたエミュレータが起動直後に終わると、すぐ rc=2 で止まる
# usage-kvm | 採寸の台本でも /dev/kvm を開けなければ、起こさずに rc=2 で止まる
# usage-reuse | 採寸の台本は、adb に見えているエミュレータがあれば起こさない
# 本物の台本を使い捨て木へ写し、隣に偽 SDK/adb/gradle を置いて、起動引数・6 段・KEEP・既存エラー・AVD 作成・
# 起動失敗時の出力と後片付けを確かめる。失敗経路を部品に集めるのは、起動できない計測テストが無期限に待たないため（CH10）。
# 実行: tools/test-android-emulator.sh [--self-test]
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SELF_TEST=${1:-}
case "$SELF_TEST" in
  ""|--self-test) ;;
  *) echo "使い方: $0 [--self-test]" >&2; exit 2 ;;
esac

TMP_ROOTS=()
cleanup() {
  local root pid
  for root in "${TMP_ROOTS[@]:-}"; do
    # root が空だと pgrep -f "/" が利用者の全プロセスに一致して kill する（2026-11-07 にセッションごと落ちた）
    case "$root" in /tmp/*|"${TMPDIR:-/tmp}"/*) ;; *) continue ;; esac
    while read -r pid; do
      [ "$pid" = "$$" ] || [ "$pid" = "$PPID" ] || kill "$pid" 2>/dev/null || true
    done < <(pgrep -f "$root/" || true)
    rm -rf "$root"
  done
}
trap cleanup EXIT

make_tree() {
  local root=$1 source_root=${2:-$ROOT}
  mkdir -p "$root/tools" "$root/collector-android" "$root/sdk/emulator" \
    "$root/sdk/system-images/android-35/google_apis/x86_64" "$root/bin" "$root/home" "$root/avd"
  : > "$root/kvm"
  if [ -n "${ASHIATO_TEST_SCRIPT_REV:-}" ]; then
    git -C "$source_root" show "${ASHIATO_TEST_SCRIPT_REV}:tools/android-emulator.sh" > "$root/tools/android-emulator.sh"
  else
    cp "$source_root/tools/android-emulator.sh" "$root/tools/"
  fi
  cp "$source_root/tools/emulator-start.sh" "$root/tools/"
  cp "$source_root/tools/usage-volume.sh" "$root/tools/"
  cp "$source_root/tools/test-usage-volume.sh" "$root/tools/"
  cat > "$root/tools/android-env.sh" <<'ENV'
#!/usr/bin/env bash
export ANDROID_HOME="${FAKE_ANDROID_HOME:?}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
ENV
  cat > "$root/tools/android-media-fixtures.sh" <<'MEDIA'
#!/usr/bin/env bash
echo media-fixtures >> "${FAKE_LOG:?}"
MEDIA
  cat > "$root/collector-android/gradlew" <<'GRADLE'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p app/build/outputs/androidTest-results
printf '<testsuite classname="fake" tests="1"/>\n' > app/build/outputs/androidTest-results/TEST-fake-_app-.xml
echo gradle >> "${FAKE_LOG:?}"
GRADLE
  cat > "$root/sdk/emulator/emulator" <<'EMU'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" > "${FAKE_ARGS:?}"
printf 'emulator %s\n' "$*" >> "${FAKE_LOG:?}"
fake_sleep() { exec -a "${FAKE_ROOT}/ch10-fake-emulator" sleep "$1"; }
fake_sleep 1000 & child=$!
echo "$child" > "${FAKE_CHILD:?}"
cleanup() { kill "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; }
trap cleanup EXIT
case "${FAKE_EMU:-ok}" in
  ok) : > "${FAKE_VISIBLE:?}"; fake_sleep 1000 ;;
  die) echo 'x86_64 emulation currently requires hardware acceleration!' >&2; exit 1 ;;
  tail) for i in $(seq 1 100); do echo "line-$i" >&2; done; exit 1 ;;
  sleep) fake_sleep 1000 ;;
  late-die) : > "${FAKE_VISIBLE:?}"; sleep 2; exit 1 ;;
  *) echo "unknown FAKE_EMU" >&2; exit 2 ;;
esac
EMU
  cat > "$root/bin/adb" <<'ADB'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${FAKE_ADB_LOG:?}"
case "$*" in
  devices)
    printf 'List of devices attached\n'
    for entry in ${FAKE_EXISTING_DEVICES:-}; do
      serial=${entry%%:*}; state=${entry#*:}
      printf '%s\t%s\n' "$serial" "$state"
    done
    [ -f "$FAKE_VISIBLE" ] && printf 'emulator-5554\tdevice\n' ;;
  wait-for-device) while [ ! -f "$FAKE_VISIBLE" ]; do sleep 1; done; echo device ;;
  get-state) [ -f "$FAKE_VISIBLE" ] && echo device || echo no device ;;
  *'shell getprop sys.boot_completed'*)
    if [ "${FAKE_ADB_MODE:-normal}" = hang ]; then sleep 1000; fi
    if [ "${FAKE_EMU:-ok}" = late-die ]; then echo 0; exit 0; fi
    [ -f "$FAKE_VISIBLE" ] && echo 1 || echo 0 ;;
  *'shell settings put global'*) : ;;
  *'emu avd name'*)
    case "$*" in
      *'emulator-5556 emu avd name'*) echo "${FAKE_EXISTING_AVD:-other-avd}" ;;
      *) echo ashiato-api35 ;;
    esac ;;
  *'emu kill'*) echo kill >> "$FAKE_LOG" ;;
  *) : ;;
esac
ADB
  cat > "$root/bin/avdmanager" <<'AVD'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${FAKE_AVD_LOG:?}"
case "$*" in
  'list avd -c') [ "${FAKE_AVD_EXISTS:-1}" = 1 ] && echo ashiato-api35 ;;
  'create avd'*) : ;;
esac
AVD
  chmod +x "$root/tools/"*.sh "$root/collector-android/gradlew" "$root/sdk/emulator/emulator" "$root/bin/"*
}

# $(new_tree) で呼ぶとサブシェルで TMP_ROOTS に足されず後始末が空振りするので、結果は NEW_ROOT で返す
NEW_ROOT=
new_tree() {
  NEW_ROOT=$(mktemp -d)
  TMP_ROOTS+=("$NEW_ROOT")
  make_tree "$NEW_ROOT"
}

run_emulator() {
  local root=$1 keep=${2:-} avd_exists=${3:-1} key=${4:-emulator} rc
  local fake_emu=${5:-ok} adb_mode=${6:-normal} kvm_device=${7:-$root/kvm}
  local device_timeout=${8:-} boot_attempts=${9:-} adb_timeout=${10:-}
  : > "$root/adb.log"; : > "$root/avd.log"; : > "$root/run.log"
  rm -f "$root/visible" "$root/child"
  local envs=(HOME="$root/home" ANDROID_AVD_HOME="$root/avd" PATH="$root/bin:$PATH"
    FAKE_ANDROID_HOME="$root/sdk" FAKE_VISIBLE="$root/visible" FAKE_CHILD="$root/child"
    FAKE_ROOT="$root"
    FAKE_ARGS="$root/args" FAKE_LOG="$root/run.log" FAKE_ADB_LOG="$root/adb.log"
    FAKE_AVD_LOG="$root/avd.log" FAKE_AVD_EXISTS="$avd_exists" FAKE_EMU="$fake_emu" FAKE_ADB_MODE="$adb_mode"
    ASHIATO_EMULATOR_DEVICE_TIMEOUT="$device_timeout" ASHIATO_EMULATOR_BOOT_ATTEMPTS="$boot_attempts" ASHIATO_ADB_CALL_TIMEOUT="$adb_timeout"
    ASHIATO_KVM_DEVICE="$kvm_device" FAKE_EXISTING_DEVICES="${FAKE_EXISTING_DEVICES:-}"
    FAKE_EXISTING_AVD="${FAKE_EXISTING_AVD:-}")
  set +e
  env "${envs[@]}" ${keep:+KEEP=1} timeout 30 "$root/tools/android-emulator.sh" >"$root/stdout" 2>"$root/stderr"
  rc=$?
  set -e
  if [ "$rc" -eq 124 ]; then
    echo "見張りで打ち切った: $key" >&2
  fi
  echo "$rc"
}

assert_kvm_failure() {
  local mode=$1 root rc
  new_tree; root=$NEW_ROOT
  if [ "$mode" = denied ]; then
    : > "$root/kvm"; chmod 000 "$root/kvm"
    [ "$(id -u)" -eq 0 ] && return 0
    run_emulator "$root" '' 1 kvm-denied ok normal "$root/kvm" >"$root/rc"
  else
    run_emulator "$root" '' 1 kvm-missing ok normal "$root/no-kvm" >"$root/rc"
  fi
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] && grep -Fq 'error: /dev/kvm を開けない' "$root/stderr" \
    && grep -Fq 'verify-run' "$root/stderr" && [ ! -s "$root/run.log" ] && ! grep -q 'emu kill' "$root/adb.log"
}

assert_kvm_sign() {
  local root rc
  new_tree; root=$NEW_ROOT
  run_emulator "$root" '' 1 kvm-sign ok normal "$root/no-kvm" >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] || return 1
  if [ -f "$ROOT/.harness2/scripts/evidence.py" ]; then
    python3 -I - "$root/stderr" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, str(Path(".harness2/scripts").resolve()))
from evidence import INFRA_SIGNS

if not INFRA_SIGNS.search(Path(sys.argv[1]).read_text(encoding="utf-8")):
    raise SystemExit(1)
PY
  else
    grep -Fq '/dev/kvm' "$root/stderr"
  fi
}

assert_dies() {
  local mode=$1 root rc
  new_tree; root=$NEW_ROOT
  run_emulator "$root" '' 1 "$mode" "$([ "$mode" = tail40 ] && echo tail || echo die)" normal "$root/kvm" >"$root/rc"
  rc=$(cat "$root/rc")
  if [ "$rc" -ne 1 ] || ! grep -Fq 'error: エミュレータが起動の途中で終わった' "$root/stderr" \
    || ! grep -Fq 'emulator.log' "$root/stderr" || { [ "$mode" = dies ] && ! grep -Fq '/dev/kvm' "$root/stderr"; }; then
    sed -n '1,120p' "$root/stderr" >&2
    return 1
  fi
  if [ "$mode" = tail40 ]; then
    if grep -Fq line-60 "$root/stderr" || ! grep -Fq line-61 "$root/stderr" || ! grep -Fq line-100 "$root/stderr"; then
      sed -n '1,120p' "$root/stderr" >&2
      return 1
    fi
  fi
}

assert_late_die() {
  local root rc
  new_tree; root=$NEW_ROOT
  run_emulator "$root" '' 1 late-die late-die normal "$root/kvm" >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] && grep -Fq 'error: エミュレータが起動の途中で終わった' "$root/stderr" && ! grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log"
}

assert_device_wait() {
  local kind=$1 root rc
  new_tree; root=$NEW_ROOT
  case "$kind" in
    default) run_emulator "$root" '' 1 device-default ok normal "$root/kvm" >"$root/rc"; grep -Fq '最大 120 秒' "$root/stdout" ;;
    timeout) run_emulator "$root" '' 1 device-timeout sleep normal "$root/kvm" 2 >"$root/rc"; grep -Fq 'error: 2 秒待っても端末が adb に現れない' "$root/stderr" ;;
    bad) run_emulator "$root" '' 1 bad-value ok normal "$root/kvm" 0 >"$root/rc"; grep -Fq 'ASHIATO_EMULATOR_DEVICE_TIMEOUT' "$root/stderr" && [ ! -s "$root/run.log" ] ;;
    hang) run_emulator "$root" '' 1 getprop-hang ok hang "$root/kvm" '' 2 1 >"$root/rc"; grep -Fq 'error: 起動が終わらない' "$root/stderr" ;;
  esac
  rc=$(cat "$root/rc")
  if [ "$kind" = default ]; then [ "$rc" -eq 0 ]; else [ "$rc" -eq 1 ]; fi
}

assert_keep_cleanup() {
  local root rc
  new_tree; root=$NEW_ROOT
  run_emulator "$root" 1 1 keep-cleanup sleep normal "$root/kvm" 2 >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] && ! grep -q 'emu kill' "$root/adb.log" && [ -z "$(pgrep -f "$root/" || true)" ]
}

assert_normal() {
  local root=${1:-} rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  rc=$(run_emulator "$root" '' 1 normal)
  [ "$rc" -eq 0 ] || { cat "$root/stderr" >&2; return 1; }
  [ "$(cat "$root/args")" = '-avd ashiato-api35 -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot' ] || return 1
  [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq 6 ] || return 1
  grep -Fq 'settings put global window_animation_scale 0' "$root/adb.log" || return 1
  grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log" || return 1
}

assert_keep() {
  local root=${1:-} rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  rc=$(run_emulator "$root" 1 1 keep)
  [ "$rc" -eq 0 ] || { cat "$root/stderr" >&2; return 1; }
  ! grep -q 'emu kill' "$root/adb.log" || return 1
}

assert_missing() {
  local root rc
  new_tree; root=$NEW_ROOT; rm "$root/sdk/emulator/emulator"
  rc=$(run_emulator "$root" '' 1 missing-emulator)
  { [ "$rc" -eq 1 ] && grep -Fq 'error: emulator が無い。' "$root/stderr"; } || return 1
  new_tree; root=$NEW_ROOT; rm -rf "$root/sdk/system-images/android-35/google_apis/x86_64"
  rc=$(run_emulator "$root" '' 1 missing-system-image)
  { [ "$rc" -eq 1 ] && grep -Fq 'error: system-images;android-35;google_apis;x86_64 が無い' "$root/stderr"; } || return 1
}

assert_avd_create() {
  local root rc
  new_tree; root=$NEW_ROOT; rc=$(run_emulator "$root" '' 0 avd-create)
  { [ "$rc" -eq 0 ] && grep -Fq 'create avd -n ashiato-api35' "$root/avd.log"; } || return 1
}

assert_same_avd() {
  local root rc
  new_tree; root=$NEW_ROOT
  FAKE_EXISTING_DEVICES='R5CT00000:device emulator-5556:device' FAKE_EXISTING_AVD='ashiato-api35' \
    run_emulator "$root" '' 1 same-avd >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] \
    && grep -Fq 'error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている（emulator-5556）' "$root/stderr" \
    && [ ! -s "$root/run.log" ] \
    && ! grep -q 'emu kill' "$root/adb.log"
}

assert_other_device() {
  local root rc
  new_tree; root=$NEW_ROOT
  FAKE_EXISTING_DEVICES='R5CT00000:device emulator-5556:device' FAKE_EXISTING_AVD='other-avd' \
    run_emulator "$root" '' 1 other-device >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 0 ] && grep -Fq -- '-avd ashiato-api35' "$root/args" \
    && grep -Fq 'emulator-5556' "$root/adb.log"
}

self_test() {
  local root rc
  new_tree; root=$NEW_ROOT
  sed -i 's/ -no-snapshot//' "$root/tools/android-emulator.sh"
  if assert_normal "$root"; then
    echo 'self-test: -no-snapshot の破壊を検出できない' >&2
    return 1
  fi
  new_tree; root=$NEW_ROOT
  sed -i 's/\[ "${KEEP:-}" = "1" \] || //' "$root/tools/android-emulator.sh"
  if assert_keep "$root"; then
    echo 'self-test: KEEP の破壊を検出できない' >&2
    return 1
  fi
}

if [ "$SELF_TEST" = --self-test ]; then
  self_test
  exit 0
fi

ONLY=${ASHIATO_TEST_ONLY:-}
case "$ONLY" in
  ""|args|stages|keep|missing|avd-create|usage-shape|kvm-missing|kvm-denied|kvm-sign|dies|tail40|late-die|device-default|bad-value|device-timeout|getprop-hang|keep-cleanup|no-named-kill|same-avd|other-device|usage-dies|usage-kvm|usage-reuse) ;;
  *) echo "未知の ASHIATO_TEST_ONLY: $ONLY" >&2; exit 2 ;;
esac

run_only() { [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; }

run_only args && { echo 'Scenario: 正常に立つとき、今と同じ引数でエミュレータを起こす'; assert_normal; }
run_only stages && { echo 'Scenario: 正常に最後まで走ると、6 段の結果を残してエミュレータを止める'; assert_normal; }
run_only keep && { echo 'Scenario: KEEP=1 なら、通った後にエミュレータを止めない'; assert_keep; }
run_only missing && { echo 'Scenario: emulator かシステムイメージが無いときは、今の文言で止まる'; assert_missing; }
run_only avd-create && { echo 'Scenario: AVD が無ければ作ってから起こす'; assert_avd_create; }
run_only same-avd && { echo 'Scenario: 同じ AVD が既に動いていると、起こさず・止めずに理由つきで止まる'; assert_same_avd; }
run_only other-device && { echo 'Scenario: 実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす'; assert_other_device; }
run_only usage-shape && { echo 'Scenario: 計測テストの台本の行の形の検査は、今のまま通る'; tools/test-usage-volume.sh; }
run_only kvm-missing && { echo 'Scenario: 開けない場所を渡すと、エミュレータを起こさずに /dev/kvm の文言で止まる'; assert_kvm_failure missing; }
run_only kvm-denied && { echo 'Scenario: 在っても読み書きできない場所を渡しても、同じく起こさずに止まる'; [ "$(id -u)" -eq 0 ] && echo 'root のため飛ばす: kvm-denied' || assert_kvm_failure denied; }
run_only kvm-sign && { echo 'Scenario: /dev/kvm を開けないときの文言は、verify-run の環境の欠落の判定に掛かる'; assert_kvm_sign; }
run_only dies && { echo 'Scenario: 起こしたエミュレータが起動直後に終わると、すぐ止まって出力の末尾を出す'; assert_dies dies; }
run_only tail40 && { echo 'Scenario: 標準エラーに出す出力の末尾は 40 行まで'; assert_dies tail40; }
run_only late-die && { echo 'Scenario: 端末が見えた後、起動の完了を待つ間に終わっても、すぐ止まる'; assert_late_die; }
run_only device-default && { echo 'Scenario: 上限を渡さなければ、端末を最大 120 秒待つ'; assert_device_wait default; }
run_only bad-value && { echo 'Scenario: 上限に正の整数でない値を渡すと、起こさずに止まる'; assert_device_wait bad; }
run_only device-timeout && { echo 'Scenario: 生きたまま端末が現れなければ、上限で止まる'; assert_device_wait timeout; }
run_only getprop-hang && { echo 'Scenario: 起動の完了の問い合わせが返らなくても、回数の上限で止まる'; assert_device_wait hang; }
run_only keep-cleanup && { echo 'Scenario: 上限で止まったとき、KEEP=1 でも自分の起こしたプロセスは残らない'; assert_keep_cleanup; }
run_only no-named-kill && { echo 'Scenario: 起動の途中で終わったとき、名指しの端末を止めない'; assert_late_die; }
true
