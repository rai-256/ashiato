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
# usage-bad-value | （Scenario 外・final R1）採寸の台本も、上限が正の整数でなければ起こさずに rc=2 で止まる
# term-cleanup | （Scenario 外・final R2）端末の待ちの最中に台本へ SIGTERM を送っても、偽のエミュレータは残らない
# only-empty | ONLY が空なら、渡さないときと同じ命令の列で全段を走らせる
# only-base | ONLY を渡さなければ、この change の前の版の台本と同じ命令の列で全段を走らせる
# only-class | ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる
# only-prep | ONLY でも、全体と同じにアニメーションを切り、写真と動画を入れる
# only-order | ONLY の段の前に、アプリを外し、止めて、権限を戻す
# only-keep | ONLY と KEEP=1 を一緒に渡すと、通った後にエミュレータを止めない
# only-fail | ONLY のクラスのテストが落ちたら、rc≠0 で止まり、エミュレータを止める
# only-zero | 渡したクラスの結果が無ければ、0 本として止まる
# only-stale | 前の実行の結果が残っていても、今回の結果として数えない
# only-dedicated | 入れ替えの試験か採寸の試験を渡すと、専用の台本の名前を出して起こさずに止まる
# only-bad | 完全修飾のクラス名 1 つの形でない値を渡すと、起こさずに止まる
# only-st56 | バックアップの試験を渡すと、gradle の計測テストを呼ばずに準備・往復・確かめを順に走らせる
# only-st56-fail | バックアップの試験の確かめが通らなければ、全体と同じ文言で止まる
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
  local root=$1 source_root=${2:-${ASHIATO_TEST_SOURCE_ROOT:-$ROOT}}
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
  # 計測テストの台本が呼ぶ隣の台本は本物を写す（無い版の台本はこれを呼ばない。確認バッチ 20261108-0519 で ST56 と重なった）
  if [ -f "$source_root/tools/android-backup-roundtrip.sh" ]; then
    cp "$source_root/tools/android-backup-roundtrip.sh" "$root/tools/"
  fi
  cat > "$root/tools/android-env.sh" <<'ENV'
#!/usr/bin/env bash
export ANDROID_HOME="${FAKE_ANDROID_HOME:?}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
ENV
  cat > "$root/tools/android-media-fixtures.sh" <<'MEDIA'
#!/usr/bin/env bash
echo media-fixtures >> "${FAKE_LOG:?}"
echo media-fixtures >> "${FAKE_SEQ_LOG:?}"
MEDIA
  # ST58 の入れ替えの段の台本は APK を 2 つ組んで端末に入れるので、偽の端末では呼ばれたことだけを残す
  cat > "$root/tools/android-st58-replace.sh" <<'ST58'
#!/usr/bin/env bash
echo st58-replace >> "${FAKE_LOG:?}"
echo st58-replace >> "${FAKE_SEQ_LOG:?}"
ST58
  cat > "$root/collector-android/gradlew" <<'GRADLE'
#!/usr/bin/env bash
set -euo pipefail
echo gradle >> "${FAKE_LOG:?}"
printf 'gradle %s\n' "$*" >> "${FAKE_SEQ_LOG:?}"
if [[ "$*" == *connectedDebugAndroidTest* ]] && [ -z "${FAKE_GRADLE_NO_XML:-}" ]; then
  mkdir -p app/build/outputs/androidTest-results
  classname=fake
  for arg in "$@"; do
    case "$arg" in
      -Pandroid.testInstrumentationRunnerArguments.class=*) classname=${arg#*=} ;;
    esac
  done
  classname=${FAKE_GRADLE_CLASS:-$classname}
  printf '<testsuite classname="%s" tests="1"/>\n' "$classname" > app/build/outputs/androidTest-results/TEST-fake-_app-.xml
fi
if [ -n "${FAKE_GRADLE_RC:-}" ]; then
  exit "$FAKE_GRADLE_RC"
fi
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
printf 'adb %s\n' "$*" >> "${FAKE_SEQ_LOG:?}"
case "$*" in
  devices)
    printf 'List of devices attached\n'
    for entry in ${FAKE_EXISTING_DEVICES:-}; do
      serial=${entry%%:*}; state=${entry#*:}
      printf '%s\t%s\n' "$serial" "$state"
    done
    [ -f "$FAKE_VISIBLE" ] && printf 'emulator-5554\tdevice\n' || true ;;
  wait-for-device) while [ ! -f "$FAKE_VISIBLE" ]; do sleep 1; done; echo device ;;
  get-state) if [ -f "$FAKE_VISIBLE" ] || printf '%s\n' "${FAKE_EXISTING_DEVICES:-}" | grep -q 'emulator-5554:device'; then echo device; else echo no device; fi ;;
  *'shell getprop ro.boot.qemu.avd_name'*)
    case "$*" in
      *'emulator-5556'*) echo "${FAKE_EXISTING_AVD:-other-avd}" ;;
      *) echo ashiato-api35 ;;
    esac ;;
  *'shell getprop sys.boot_completed'*)
    if [ "${FAKE_ADB_MODE:-normal}" = hang ]; then sleep 1000; fi
    if [ "${FAKE_EMU:-ok}" = late-die ]; then echo 0; exit 0; fi
    [ -f "$FAKE_VISIBLE" ] && echo 1 || echo 0 ;;
  *'shell am instrument'*'St56BackupTest#verifyRestored'*) printf '%s\n' "${FAKE_VERIFY_OUT:-OK (1 test)}" ;;
  *'shell am instrument'*) printf 'OK (1 test)\n' ;;
  *'shell bmgr enabled'*) printf 'Backup Manager currently enabled\n' ;;
  *'shell bmgr transport'*) printf 'Selected transport com.android.localtransport/.LocalTransport\n' ;;
  *'shell bmgr backupnow'*) printf 'Package dev.ashiato.collector with result: Success\n' ;;
  *'shell bmgr restore'*) printf 'restoreFinished: 0\n' ;;
  *'exec-out run-as'*'usage-volume.jsonl'*) printf '{"event":"fake"}\n' ;;
  *'exec-out run-as'*'usage-volume.meta'*) printf 'span_seconds=86400\n' ;;
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
  local device_timeout=${8:-} boot_attempts=${9:-} adb_timeout=${10:-} only=${11-} only_set=${11+set}
  : > "$root/adb.log"; : > "$root/avd.log"; : > "$root/run.log"; : > "$root/seq.log"
  rm -f "$root/visible" "$root/child"
  local envs=(HOME="$root/home" ANDROID_AVD_HOME="$root/avd" PATH="$root/bin:$PATH"
    FAKE_ANDROID_HOME="$root/sdk" FAKE_VISIBLE="$root/visible" FAKE_CHILD="$root/child"
    FAKE_ROOT="$root"
    FAKE_ARGS="$root/args" FAKE_LOG="$root/run.log" FAKE_ADB_LOG="$root/adb.log"
    FAKE_SEQ_LOG="$root/seq.log"
    FAKE_AVD_LOG="$root/avd.log" FAKE_AVD_EXISTS="$avd_exists" FAKE_EMU="$fake_emu" FAKE_ADB_MODE="$adb_mode"
    ASHIATO_EMULATOR_DEVICE_TIMEOUT="$device_timeout" ASHIATO_EMULATOR_BOOT_ATTEMPTS="$boot_attempts" ASHIATO_ADB_CALL_TIMEOUT="$adb_timeout"
    ASHIATO_KVM_DEVICE="$kvm_device" FAKE_EXISTING_DEVICES="${FAKE_EXISTING_DEVICES:-}"
    FAKE_EXISTING_AVD="${FAKE_EXISTING_AVD:-}")
  set +e
  local only_env=(-u ONLY)
  [ -n "$only_set" ] && only_env=(ONLY="$only")
  env "${only_env[@]}" "${envs[@]}" ${keep:+KEEP=1} timeout 30 "$root/tools/android-emulator.sh" >"$root/stdout" 2>"$root/stderr"
  rc=$?
  set -e
  if [ "$rc" -eq 124 ]; then
    echo "見張りで打ち切った: $key" >&2
  fi
  echo "$rc"
}

filtered_seq() {
  local root=$1
  grep -Ev 'getprop|devices|get-state|wait-for-device|emu avd name' "$root/seq.log"
}

assert_kvm_failure() {
  local mode=$1 root rc
  new_tree; root=$NEW_ROOT
  if [ "$mode" = denied ]; then
    : > "$root/kvm"; chmod 000 "$root/kvm"
    run_emulator "$root" '' 1 kvm-denied ok normal "$root/kvm" >"$root/rc"
  else
    run_emulator "$root" '' 1 kvm-missing ok normal "$root/no-kvm" >"$root/rc"
  fi
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] && grep -Fq 'error: /dev/kvm を開けない' "$root/stderr" \
    && grep -Fq 'verify-run' "$root/stderr" && [ ! -s "$root/run.log" ] && ! grep -q 'emu kill' "$root/adb.log" \
    && { [ "$mode" = denied ] || [ ! -e "$root/no-kvm" ]; }  # 調べるだけで作らない（code-verify R11）
}

assert_kvm_sign() {
  local root rc
  new_tree; root=$NEW_ROOT
  run_emulator "$root" '' 1 kvm-sign ok normal "$root/no-kvm" >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] || return 1
  if [ -f "$ROOT/.harness2/scripts/evidence.py" ]; then
    python3 -I - "$root/stderr" "$ROOT" <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, str(Path(sys.argv[2], ".harness2/scripts").resolve()))
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
  # 出力の全文は決まった置き場に残る（後から理由を読むため。code-verify R13）
  if [ "$mode" = dies ] && { ! grep -Fq 'error: エミュレータの出力全文: collector-android/app/build/outputs/emulator.log' "$root/stderr" \
    || ! grep -Fq 'x86_64 emulation currently requires hardware acceleration!' "$root/collector-android/app/build/outputs/emulator.log"; }; then
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
    default) run_emulator "$root" '' 1 device-default ok normal "$root/kvm" >"$root/rc"; grep -Fq '最大 120 秒' "$root/stdout" \
      && grep -Fq '起動の完了を最大 180 回（2 秒ごと。1 回の問い合わせは最大 10 秒）' "$root/stdout" ;;  # code-verify R14
    timeout) run_emulator "$root" '' 1 device-timeout sleep normal "$root/kvm" 2 >"$root/rc"; grep -Fq 'error: 2 秒待っても端末が adb に現れない' "$root/stderr" ;;
    bad)
      # spec の WHEN は `0` と `abc` の 2 回（code-verify R12）
      local value
      for value in 0 abc; do
        run_emulator "$root" '' 1 bad-value ok normal "$root/kvm" "$value" >"$root/rc"
        grep -Eq '^error: .*ASHIATO_EMULATOR_DEVICE_TIMEOUT' "$root/stderr" && [ ! -s "$root/run.log" ] && [ "$(cat "$root/rc")" -eq 1 ] || return 1
      done ;;
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
  # 段の数は写した台本の keep_stage の呼び出しの数（CH10 の時点では 6。ほかの Story が段を足しても、どの段も結果を残すことを見る。
  # 6 を決め打つと ST56 の St56BackupTest の 2 段で落ちた —— 確認バッチ 20261108-0519）
  local stages
  stages=$(grep -c '^keep_stage$' "$root/tools/android-emulator.sh")
  [ "$stages" -ge 6 ] || return 1
  [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq "$stages" ] || return 1
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
  FAKE_EXISTING_DEVICES='emulator-5554:device' FAKE_EXISTING_AVD='ashiato-api35' \
    run_emulator "$root" '' 1 same-avd >"$root/rc"
  rc=$(cat "$root/rc")
  [ "$rc" -eq 1 ] \
    && grep -Eq 'error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている（emulator-5554）.*adb -s emulator-5554 emu kill' "$root/stderr" \
    && [ ! -s "$root/run.log" ] \
    && grep -Fq 'shell getprop ro.boot.qemu.avd_name' "$root/adb.log" \
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

run_usage() {
  local root=$1 key=$2 fake_emu=${3:-ok} kvm_device=${4:-$root/kvm} existing=${5:-} device_timeout=${6:-} rc
  : > "$root/adb.log"; : > "$root/avd.log"; : > "$root/run.log"; : > "$root/seq.log"
  rm -f "$root/visible" "$root/child"
  local envs=(HOME="$root/home" ANDROID_AVD_HOME="$root/avd" PATH="$root/bin:$PATH"
    ANDROID_HOME="$root/sdk" FAKE_ANDROID_HOME="$root/sdk" FAKE_VISIBLE="$root/visible" FAKE_CHILD="$root/child"
    FAKE_ROOT="$root" FAKE_ARGS="$root/args" FAKE_LOG="$root/run.log" FAKE_ADB_LOG="$root/adb.log"
    FAKE_SEQ_LOG="$root/seq.log"
    FAKE_AVD_LOG="$root/avd.log" FAKE_AVD_EXISTS=1 FAKE_EMU="$fake_emu" FAKE_ADB_MODE=normal
    ASHIATO_KVM_DEVICE="$kvm_device" FAKE_EXISTING_DEVICES="$existing"
    ASHIATO_EMULATOR_DEVICE_TIMEOUT="$device_timeout")
  set +e
  env -u ONLY "${envs[@]}" timeout 30 "$root/tools/usage-volume.sh" >"$root/stdout" 2>"$root/stderr"
  rc=$?
  set -e
  if [ "$rc" -eq 124 ]; then
    echo "見張りで打ち切った: $key" >&2
  fi
  echo "$rc"
}

assert_usage_dies() {
  local root rc
  new_tree; root=$NEW_ROOT
  rc=$(run_usage "$root" usage-dies die)
  if [ "$rc" -ne 2 ] || ! grep -Fq 'error: エミュレータが起動の途中で終わった' "$root/stderr" \
    || [ "$(cat "$root/args")" != '-avd ashiato-api35 -wipe-data -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot' ] \
    || grep -Fq 'emu kill' "$root/adb.log" \
    || [ -n "$(pgrep -f "$root/" || true)" ]; then
    cat "$root/stderr" >&2
    return 1
  fi
}

assert_usage_kvm() {
  local root rc
  new_tree; root=$NEW_ROOT
  rc=$(run_usage "$root" usage-kvm ok "$root/no-kvm")
  [ "$rc" -eq 2 ] \
    && grep -Fq 'error: /dev/kvm を開けない' "$root/stderr" \
    && [ ! -s "$root/run.log" ] && [ ! -e "$root/no-kvm" ]
}

assert_usage_reuse() {
  local root rc
  new_tree; root=$NEW_ROOT
  rc=$(run_usage "$root" usage-reuse ok "$root/kvm" 'emulator-5554:device')
  if [ "$rc" -ne 0 ] || grep -Fq 'emulator ' "$root/run.log" || ! grep -Fq 'emulator-5554' "$root/adb.log"; then
    cat "$root/stdout" "$root/stderr" "$root/adb.log" >&2
    return 1
  fi
}

# final R1: 採寸の台本は値を起こす前に確かめる（起こしてから落ちると偽のエミュレータが残った）
assert_usage_bad_value() {
  local root rc
  new_tree; root=$NEW_ROOT
  rc=$(run_usage "$root" usage-bad-value ok "$root/kvm" '' 0)
  if [ "$rc" -ne 2 ] || ! grep -Fq 'error: ASHIATO_EMULATOR_DEVICE_TIMEOUT' "$root/stderr" \
    || [ -s "$root/run.log" ] || [ -n "$(pgrep -f "$root/" || true)" ]; then
    cat "$root/stderr" "$root/run.log" >&2
    return 1
  fi
}

# final R2: 端末の待ちの最中に止められても、EXIT の後片付けが自分の起こしたものを止める
assert_term_cleanup() {
  local root pid rc
  new_tree; root=$NEW_ROOT
  : > "$root/adb.log"; : > "$root/avd.log"; : > "$root/run.log"
  rm -f "$root/visible" "$root/child"
  env HOME="$root/home" ANDROID_AVD_HOME="$root/avd" PATH="$root/bin:$PATH" \
    FAKE_ANDROID_HOME="$root/sdk" FAKE_VISIBLE="$root/visible" FAKE_CHILD="$root/child" FAKE_ROOT="$root" \
    FAKE_ARGS="$root/args" FAKE_LOG="$root/run.log" FAKE_ADB_LOG="$root/adb.log" FAKE_AVD_LOG="$root/avd.log" \
    FAKE_SEQ_LOG="$root/seq.log" \
    env -u ONLY \
    FAKE_AVD_EXISTS=1 FAKE_EMU=sleep FAKE_ADB_MODE=normal ASHIATO_EMULATOR_DEVICE_TIMEOUT=30 \
    ASHIATO_KVM_DEVICE="$root/kvm" FAKE_EXISTING_DEVICES='' FAKE_EXISTING_AVD='' \
    "$root/tools/android-emulator.sh" >"$root/stdout" 2>"$root/stderr" &
  pid=$!
  for _ in $(seq 1 20); do
    [ -s "$root/child" ] && grep -Fq '端末が adb に現れるのを' "$root/stdout" && break
    sleep 0.5
  done
  if [ ! -s "$root/child" ]; then
    kill -KILL "$pid" 2>/dev/null || true
    echo 'term-cleanup: 偽のエミュレータが起きない' >&2
    return 1
  fi
  sleep 1
  kill -TERM "$pid"
  set +e
  wait "$pid"
  rc=$?
  set -e
  # emu_stop_own は TERM の後に最大 5 秒待つ。待ちの外へ漏れた分も数えるため少しだけ余裕を見る
  sleep 1
  if [ "$rc" -eq 0 ] || [ -n "$(pgrep -f "$root/" || true)" ]; then
    echo "term-cleanup: rc=$rc / 残り: $(pgrep -af "$root/" || true)" >&2
    return 1
  fi
}

self_test() {
  local root rc base_root changed_root base_seq changed_seq source_root base_rev
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

  source_root=$(mktemp -d)
  TMP_ROOTS+=("$source_root")
  cp -R "$ROOT/tools" "$source_root/tools"
  git -C "$source_root" init -q
  git -C "$source_root" add tools
  git -C "$source_root" -c user.name=test -c user.email=test@example.invalid commit -qm base
  base_rev=$(git -C "$source_root" rev-parse HEAD)
  awk '
    /adb shell pm reset-permissions            #/ && !removed { removed=1; next }
    { print }
  ' "$source_root/tools/android-emulator.sh" >"$source_root/android-emulator.sh.tmp"
  mv "$source_root/android-emulator.sh.tmp" "$source_root/tools/android-emulator.sh"
  chmod +x "$source_root/tools/android-emulator.sh"
  if ASHIATO_TEST_SOURCE_ROOT=$source_root ASHIATO_TEST_BASE_REV=$base_rev assert_only_base; then
    echo 'self-test: only-base が命令削除を検出できない' >&2
    return 1
  fi
  git -C "$source_root" checkout -q -- tools/android-emulator.sh
  awk '
    /adb shell pm reset-permissions            #/ && !removed { saved=$0; removed=1; next }
    removed && /^keep_stage$/ && !inserted { print; print saved; inserted=1; next }
    { print }
  ' "$source_root/tools/android-emulator.sh" >"$source_root/android-emulator.sh.tmp"
  mv "$source_root/android-emulator.sh.tmp" "$source_root/tools/android-emulator.sh"
  chmod +x "$source_root/tools/android-emulator.sh"
  if ASHIATO_TEST_SOURCE_ROOT=$source_root ASHIATO_TEST_BASE_REV=$base_rev assert_only_base; then
    echo 'self-test: only-base が命令順序変更を検出できない' >&2
    return 1
  fi

  new_tree; root=$NEW_ROOT
  sed -i 's/-Pandroid.testInstrumentationRunnerArguments.class="$ONLY")/-Pandroid.testInstrumentationRunnerArguments.notClass="$ONLY")/' "$root/tools/android-emulator.sh"
  if assert_only_class "$root"; then
    echo 'self-test: ONLY の class= 絞りの破壊を検出できない' >&2
    return 1
  fi

  new_tree; root=$NEW_ROOT
  sed -i "/grep -rqF --include='TEST-\*.xml'/c\  if false; then" "$root/tools/android-emulator.sh"
  if assert_only_zero "$root"; then
    echo 'self-test: ONLY の XML の確かめの破壊を検出できない' >&2
    return 1
  fi

  new_tree; root=$NEW_ROOT
  if assert_only_no_xml "$root"; then
    echo 'self-test: ONLY の XML 無しを検出できない' >&2
    return 1
  fi

  new_tree; root=$NEW_ROOT
  sed -i 's/rm -rf "\$OUT\/androidTest-results"/:/' "$root/tools/android-emulator.sh"
  if assert_only_stale "$root"; then
    echo 'self-test: ONLY の前の結果削除の破壊を検出できない' >&2
    return 1
  fi
}

assert_only_empty() {
  local root unset_root empty_root unset_seq empty_seq unset_stages empty_stages unset_replace empty_replace keep_stages
  new_tree; unset_root=$NEW_ROOT
  run_emulator "$unset_root" '' 1 only-empty >"$unset_root/rc"
  new_tree; empty_root=$NEW_ROOT
  run_emulator "$empty_root" '' 1 only-empty ok normal "$empty_root/kvm" '' '' '' '' >"$empty_root/rc"
  unset_seq=$(filtered_seq "$unset_root")
  empty_seq=$(filtered_seq "$empty_root")
  keep_stages=$(grep -c '^keep_stage$' "$unset_root/tools/android-emulator.sh")
  unset_stages=$(find "$unset_root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)
  empty_stages=$(find "$empty_root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)
  unset_replace=$(grep -c '^st58-replace$' "$unset_root/seq.log")
  empty_replace=$(grep -c '^st58-replace$' "$empty_root/seq.log")
  [ "$(cat "$unset_root/rc")" -eq 0 ] && [ "$(cat "$empty_root/rc")" -eq 0 ] \
    && [ "$unset_seq" = "$empty_seq" ] \
    && [ "$unset_stages" -eq "$empty_stages" ] \
    && [ "$unset_stages" -eq "$keep_stages" ] \
    && [ "$unset_replace" -eq 1 ] && [ "$empty_replace" -eq 1 ]
}

assert_only_base() {
  local base=${ASHIATO_TEST_BASE_REV:-} current_root base_root current_seq base_seq prior_script_rev prior_script_rev_set
  if [ -z "$base" ]; then
    echo '比べる元の版が無いので飛ばす: only-base'
    return 0
  fi
  prior_script_rev_set=${ASHIATO_TEST_SCRIPT_REV+x}
  prior_script_rev=${ASHIATO_TEST_SCRIPT_REV:-}
  unset ASHIATO_TEST_SCRIPT_REV
  new_tree; current_root=$NEW_ROOT
  run_emulator "$current_root" '' 1 only-base >"$current_root/rc"
  ASHIATO_TEST_SCRIPT_REV=$base
  new_tree; base_root=$NEW_ROOT
  if [ -n "$prior_script_rev_set" ]; then
    export ASHIATO_TEST_SCRIPT_REV=$prior_script_rev
  else
    unset ASHIATO_TEST_SCRIPT_REV
  fi
  run_emulator "$base_root" '' 1 only-base >"$base_root/rc"
  current_seq=$(filtered_seq "$current_root")
  base_seq=$(filtered_seq "$base_root")
  [ "$(cat "$current_root/rc")" -eq 0 ] && [ "$(cat "$base_root/rc")" -eq 0 ] \
    && [ "$current_seq" = "$base_seq" ]
}

assert_only_class() {
  local root=${1:-} class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  rc=$(run_emulator "$root" '' 1 only-class ok normal "$root/kvm" '' '' '' "$class")
  [ "$rc" -eq 0 ] \
    && [ "$(grep -c 'connectedDebugAndroidTest' "$root/seq.log")" -eq 1 ] \
    && grep -Fq -- "-Pandroid.testInstrumentationRunnerArguments.class=$class" "$root/seq.log" \
    && ! grep -q 'notClass\|annotation' "$root/seq.log" \
    && [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq 1 ] \
    && ! grep -q 'st58-replace\|bmgr' "$root/seq.log" \
    && grep -Fq "== 絞った実行（ONLY=$class）。全体ではない" "$root/stderr" \
    && grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log"
}

assert_only_prep() {
  local root class='dev.ashiato.collector.MediaPremisesInstrumentedTest' gradle_line key rc
  new_tree; root=$NEW_ROOT
  rc=$(run_emulator "$root" '' 1 only-prep ok normal "$root/kvm" '' '' '' "$class")
  gradle_line=$(grep -n 'connectedDebugAndroidTest' "$root/seq.log" | cut -d: -f1)
  [ "$rc" -eq 0 ] && [ "$(grep -c '^media-fixtures$' "$root/seq.log")" -eq 1 ] \
    && [ "$(grep -n 'media-fixtures' "$root/seq.log" | cut -d: -f1)" -lt "$gradle_line" ] || return 1
  for key in window_animation_scale transition_animation_scale animator_duration_scale; do
    grep -Fq "settings put global $key 0" "$root/seq.log" \
      && [ "$(grep -n "settings put global $key 0" "$root/seq.log" | cut -d: -f1)" -lt "$gradle_line" ] || return 1
  done
}

assert_only_order() {
  local root class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  new_tree; root=$NEW_ROOT
  rc=$(run_emulator "$root" '' 1 only-order ok normal "$root/kvm" '' '' '' "$class")
  [ "$rc" -eq 0 ] \
    && awk '/uninstall dev.ashiato.collector.test/{u=NR} /uninstall dev.ashiato.collector$/{a=NR} /shell am force-stop dev.ashiato.collector/{f=NR} /shell pm reset-permissions/{p=NR} /connectedDebugAndroidTest/{g=NR} END{exit !(u && a && f && p && g && u<a && a<f && f<p && p<g)}' "$root/seq.log"
}

assert_only_keep() {
  local root class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  new_tree; root=$NEW_ROOT
  rc=$(run_emulator "$root" 1 1 only-keep ok normal "$root/kvm" '' '' '' "$class")
  [ "$rc" -eq 0 ] && ! grep -q 'emu kill' "$root/adb.log"
}

assert_only_fail() {
  local root class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  new_tree; root=$NEW_ROOT
  export FAKE_GRADLE_RC=1
  rc=$(run_emulator "$root" '' 1 only-fail ok normal "$root/kvm" '' '' '' "$class")
  unset FAKE_GRADLE_RC
  [ "$rc" -ne 0 ] && grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log" \
    && ! grep -Fq 'のテストが 1 本も走っていない' "$root/stderr"
}

assert_only_zero() {
  local root=${1:-} class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  export FAKE_GRADLE_CLASS=dev.ashiato.collector.Other
  rc=$(run_emulator "$root" '' 1 only-zero ok normal "$root/kvm" '' '' '' "$class")
  unset FAKE_GRADLE_CLASS
  [ "$rc" -eq 1 ] && grep -Fq "error: ONLY=$class のテストが 1 本も走っていない" "$root/stderr"
}

assert_only_no_xml() {
  local root=${1:-} class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  export FAKE_GRADLE_NO_XML=1
  rc=$(run_emulator "$root" '' 1 only-zero ok normal "$root/kvm" '' '' '' "$class")
  unset FAKE_GRADLE_NO_XML
  [ "$rc" -eq 1 ] && grep -Fq "error: ONLY=$class のテストが 1 本も走っていない" "$root/stderr"
}

assert_only_stale() {
  local root=${1:-} class='dev.ashiato.collector.MediaPremisesInstrumentedTest' rc
  [ -n "$root" ] || { new_tree; root=$NEW_ROOT; }
  mkdir -p "$root/collector-android/app/build/outputs/androidTest-results/connected"
  printf '<testsuite classname="%s"/>\n' "$class" > "$root/collector-android/app/build/outputs/androidTest-results/connected/TEST-stale-_app-.xml"
  export FAKE_GRADLE_CLASS=dev.ashiato.collector.Other
  rc=$(run_emulator "$root" '' 1 only-stale ok normal "$root/kvm" '' '' '' "$class")
  unset FAKE_GRADLE_CLASS
  [ "$rc" -eq 1 ] && grep -Fq "error: ONLY=$class のテストが 1 本も走っていない" "$root/stderr"
}

assert_only_dedicated() {
  local value script root rc
  for value in \
    dev.ashiato.collector.St58PackageReplacedInstrumentedTest \
    dev.ashiato.collector.UsageVolumeInstrumentedTest; do
    if [ "$value" = dev.ashiato.collector.St58PackageReplacedInstrumentedTest ]; then
      script=tools/android-st58-replace.sh
    else
      script=tools/usage-volume.sh
    fi
    new_tree; root=$NEW_ROOT
    rc=$(run_emulator "$root" '' 1 only-dedicated ok normal "$root/kvm" '' '' '' "$value")
    [ "$rc" -eq 1 ] && [ ! -s "$root/run.log" ] && [ ! -s "$root/avd.log" ] \
      && grep -Fq "error: ONLY=$value は $script で走らせる" "$root/stderr" || return 1
  done
}

assert_only_bad() {
  local value root rc
  for value in MediaPremisesInstrumentedTest dev.ashiato.collector.A,dev.ashiato.collector.B \
    dev.ashiato.collector.St56BackupTest#prepareBackup 'dev.ashiato.collector.A B'; do
    new_tree; root=$NEW_ROOT
    rc=$(run_emulator "$root" '' 1 only-bad ok normal "$root/kvm" '' '' '' "$value")
    [ "$rc" -eq 1 ] && [ ! -s "$root/run.log" ] && [ ! -s "$root/avd.log" ] \
      && grep -Fq "error: ONLY=$value" "$root/stderr" || return 1
  done
}

assert_only_st56() {
  local root class='dev.ashiato.collector.St56BackupTest' rc
  new_tree; root=$NEW_ROOT
  rc=$(run_emulator "$root" '' 1 only-st56 ok normal "$root/kvm" '' '' '' "$class")
  [ "$rc" -eq 0 ] \
    && ! grep -q 'connectedDebugAndroidTest' "$root/seq.log" \
    && grep -Fq 'gradle -q :app:installDebug :app:installDebugAndroidTest' "$root/seq.log" \
    && grep -Fq 'shell bmgr enable true' "$root/seq.log" \
    && grep -Fq 'St56BackupTest#prepareBackup' "$root/seq.log" \
    && grep -Fq 'backupnow' "$root/seq.log" \
    && grep -Fq 'bmgr restore' "$root/seq.log" \
    && grep -Fq 'St56BackupTest#verifyRestored' "$root/seq.log" \
    && awk '/gradle -q :app:installDebug :app:installDebugAndroidTest/{install=NR} /shell bmgr enable true/{enable=NR} /St56BackupTest#prepareBackup/{prepare=NR} /backupnow/{backup=NR} /bmgr restore/{restore=NR} /St56BackupTest#verifyRestored/{verify=NR} END{exit !(install < enable && enable < prepare && prepare < backup && backup < restore && restore < verify)}' "$root/seq.log" \
    && ! grep -q 'st58-replace' "$root/seq.log" \
    && grep -Fq 'OK (1 test)' "$root/stdout" \
    && [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq 0 ] \
    && [ ! -d "$root/collector-android/app/build/outputs/androidTest-results" ]
}

assert_only_st56_fail() {
  local root class='dev.ashiato.collector.St56BackupTest' rc
  new_tree; root=$NEW_ROOT
  export FAKE_VERIFY_OUT='FAILURES!!!'
  rc=$(run_emulator "$root" '' 1 only-st56-fail ok normal "$root/kvm" '' '' '' "$class")
  unset FAKE_VERIFY_OUT
  [ "$rc" -eq 1 ] \
    && grep -Fq 'error: St56BackupTest の復元の確かめが通らない' "$root/stderr" \
    && ! grep -q 'connectedDebugAndroidTest' "$root/seq.log" \
    && grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log" \
    && [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq 0 ]
}

if [ "$SELF_TEST" = --self-test ]; then
  self_test
  exit 0
fi

CASE=${ASHIATO_TEST_ONLY:-}
case "$CASE" in
  ""|args|stages|keep|missing|avd-create|usage-shape|kvm-missing|kvm-denied|kvm-sign|dies|tail40|late-die|device-default|bad-value|device-timeout|getprop-hang|keep-cleanup|no-named-kill|same-avd|other-device|usage-dies|usage-kvm|usage-reuse|usage-bad-value|term-cleanup|only-empty|only-base|only-class|only-prep|only-order|only-keep|only-fail|only-zero|only-stale|only-dedicated|only-bad|only-st56|only-st56-fail) ;;
  *) echo "未知の ASHIATO_TEST_ONLY: $CASE" >&2; exit 2 ;;
esac

run_only() { [ -z "$CASE" ] || [ "$CASE" = "$1" ]; }

run_only args && { echo 'Scenario: 正常に立つとき、今と同じ引数でエミュレータを起こす'; assert_normal; }
run_only stages && { echo 'Scenario: 正常に最後まで走ると、6 段の結果を残してエミュレータを止める'; assert_normal; }
run_only keep && { echo 'Scenario: KEEP=1 なら、通った後にエミュレータを止めない'; assert_keep; }
run_only missing && { echo 'Scenario: emulator かシステムイメージが無いときは、今の文言で止まる'; assert_missing; }
run_only avd-create && { echo 'Scenario: AVD が無ければ作ってから起こす'; assert_avd_create; }
run_only same-avd && { echo 'Scenario: 同じ AVD が既に動いていると、起こさず・止めずに理由つきで止まる'; assert_same_avd; }
run_only other-device && { echo 'Scenario: 実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす'; assert_other_device; }
run_only usage-dies && { echo 'Scenario: 採寸の台本で起こしたエミュレータが起動直後に終わると、すぐ rc=2 で止まる'; assert_usage_dies; }
run_only usage-kvm && { echo 'Scenario: 採寸の台本でも /dev/kvm を開けなければ、起こさずに rc=2 で止まる'; assert_usage_kvm; }
run_only usage-reuse && { echo 'Scenario: 採寸の台本は、adb に見えているエミュレータがあれば起こさない'; assert_usage_reuse; }
run_only usage-shape && { echo 'Scenario: 計測テストの台本の行の形の検査は、今のまま通る'; "$ROOT/tools/test-usage-volume.sh"; }
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
run_only usage-bad-value && { echo '== 採寸の台本の上限の値（Scenario 外・final R1）'; assert_usage_bad_value; }
run_only term-cleanup && { echo '== 待ちの最中の SIGTERM の後片付け（Scenario 外・final R2）'; assert_term_cleanup; }
run_only only-empty && { echo 'Scenario: ONLY が空なら、渡さないときと同じ命令の列で全段を走らせる'; assert_only_empty; }
run_only only-base && { echo 'Scenario: ONLY を渡さなければ、この change の前の版の台本と同じ命令の列で全段を走らせる'; assert_only_base; }
run_only only-class && { echo 'Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる'; assert_only_class || exit 1; }
run_only only-prep && { echo 'Scenario: ONLY でも、全体と同じにアニメーションを切り、写真と動画を入れる'; assert_only_prep || exit 1; }
run_only only-order && { echo 'Scenario: ONLY の段の前に、アプリを外し、止めて、権限を戻す'; assert_only_order || exit 1; }
run_only only-keep && { echo 'Scenario: ONLY と KEEP=1 を一緒に渡すと、通った後にエミュレータを止めない'; assert_only_keep || exit 1; }
run_only only-fail && { echo 'Scenario: ONLY のクラスのテストが落ちたら、rc≠0 で止まり、エミュレータを止める'; assert_only_fail || exit 1; }
run_only only-zero && { echo 'Scenario: 渡したクラスの結果が無ければ、0 本として止まる'; assert_only_zero || exit 1; }
run_only only-stale && { echo 'Scenario: 前の実行の結果が残っていても、今回の結果として数えない'; assert_only_stale || exit 1; }
run_only only-dedicated && { echo 'Scenario: 入れ替えの試験か採寸の試験を渡すと、専用の台本の名前を出して起こさずに止まる'; assert_only_dedicated || exit 1; }
run_only only-bad && { echo 'Scenario: 完全修飾のクラス名 1 つの形でない値を渡すと、起こさずに止まる'; assert_only_bad || exit 1; }
run_only only-st56 && { echo 'Scenario: バックアップの試験を渡すと、gradle の計測テストを呼ばずに準備・往復・確かめを順に走らせる'; assert_only_st56 || exit 1; }
run_only only-st56-fail && { echo 'Scenario: バックアップの試験の確かめが通らなければ、全体と同じ文言で止まる'; assert_only_st56_fail || exit 1; }
true
