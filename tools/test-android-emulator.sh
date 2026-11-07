#!/usr/bin/env bash
# android-emulator.sh の正常経路を、実 SDK・実端末なしで固定する試験（CH10）。
# 本物の台本を使い捨て木へ写し、隣に偽 SDK/adb/gradle を置いて、起動引数・6 段・KEEP・既存エラー・AVD 作成を確かめる。
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
  cp "$source_root/tools/android-emulator.sh" "$root/tools/"
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
sleep 1000 & child=$!
echo "$child" > "${FAKE_CHILD:?}"
cleanup() { kill "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; }
trap cleanup EXIT
case "${FAKE_EMU:-ok}" in
  ok) : > "${FAKE_VISIBLE:?}"; sleep 1000 ;;
  die) echo 'fake emulator died' >&2; exit 1 ;;
  sleep) sleep 1000 ;;
  late-die) : > "${FAKE_VISIBLE:?}"; sleep 2; exit 1 ;;
  *) echo "unknown FAKE_EMU" >&2; exit 2 ;;
esac
EMU
  cat > "$root/bin/adb" <<'ADB'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${FAKE_ADB_LOG:?}"
case "$*" in
  devices) printf 'List of devices attached\n'; [ -f "$FAKE_VISIBLE" ] && printf 'emulator-5554\tdevice\n' ;;
  wait-for-device|get-state) echo device ;;
  *'shell getprop sys.boot_completed'*) [ -f "$FAKE_VISIBLE" ] && echo 1 || echo 0 ;;
  *'shell settings put global'*) : ;;
  *'emu avd name'*) echo ashiato-api35 ;;
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
  local root=$1 keep=${2:-} avd_exists=${3:-1} rc
  : > "$root/adb.log"; : > "$root/avd.log"; : > "$root/run.log"
  rm -f "$root/visible" "$root/child"
  local envs=(HOME="$root/home" ANDROID_AVD_HOME="$root/avd" PATH="$root/bin:$PATH"
    FAKE_ANDROID_HOME="$root/sdk" FAKE_VISIBLE="$root/visible" FAKE_CHILD="$root/child"
    FAKE_ARGS="$root/args" FAKE_LOG="$root/run.log" FAKE_ADB_LOG="$root/adb.log"
    FAKE_AVD_LOG="$root/avd.log" FAKE_AVD_EXISTS="$avd_exists" FAKE_EMU=ok)
  set +e
  env "${envs[@]}" ${keep:+KEEP=1} timeout 30 "$root/tools/android-emulator.sh" >"$root/stdout" 2>"$root/stderr"
  rc=$?
  set -e
  echo "$rc"
}

assert_normal() {
  local root rc
  new_tree; root=$NEW_ROOT
  rc=$(run_emulator "$root")
  [ "$rc" -eq 0 ] || { cat "$root/stderr" >&2; return 1; }
  [ "$(cat "$root/args")" = '-avd ashiato-api35 -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot' ]
  [ "$(find "$root/collector-android/app/build/outputs" -maxdepth 1 -type d -name 'androidTest-results-stage*' | wc -l)" -eq 6 ]
  grep -Fq 'settings put global window_animation_scale 0' "$root/adb.log"
  grep -Fq -- '-s emulator-5554 emu kill' "$root/adb.log"
}

assert_keep() {
  local root rc
  new_tree; root=$NEW_ROOT; rc=$(run_emulator "$root" 1)
  [ "$rc" -eq 0 ] || { cat "$root/stderr" >&2; return 1; }
  ! grep -q 'emu kill' "$root/adb.log"
}

assert_missing() {
  local root rc
  new_tree; root=$NEW_ROOT; rm "$root/sdk/emulator/emulator"
  rc=$(run_emulator "$root")
  [ "$rc" -eq 1 ] && grep -Fq 'error: emulator が無い。' "$root/stderr"
  new_tree; root=$NEW_ROOT; rm -rf "$root/sdk/system-images/android-35/google_apis/x86_64"
  rc=$(run_emulator "$root")
  [ "$rc" -eq 1 ] && grep -Fq 'error: system-images;android-35;google_apis;x86_64 が無い' "$root/stderr"
}

assert_avd_create() {
  local root rc
  new_tree; root=$NEW_ROOT; rc=$(run_emulator "$root" '' 0)
  [ "$rc" -eq 0 ] && grep -Fq 'create avd -n ashiato-api35' "$root/avd.log"
}

self_test() {
  local root rc
  new_tree; root=$NEW_ROOT
  sed -i 's/ -no-snapshot//' "$root/tools/android-emulator.sh"
  rc=$(run_emulator "$root")
  if [ "$rc" -eq 0 ] && [ "$(cat "$root/args")" = '-avd ashiato-api35 -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot' ]; then
    echo 'self-test: -no-snapshot の破壊を検出できない' >&2
    return 1
  fi
  new_tree; root=$NEW_ROOT
  sed -i 's/\[ "${KEEP:-}" = "1" \] || //' "$root/tools/android-emulator.sh"
  rc=$(run_emulator "$root" 1)
  if [ "$rc" -eq 0 ] && ! grep -q 'emu kill' "$root/adb.log"; then
    echo 'self-test: KEEP の破壊を検出できない' >&2
    return 1
  fi
}

if [ "$SELF_TEST" = --self-test ]; then
  self_test
  exit 0
fi

echo 'Scenario: 正常に立つとき、今と同じ引数でエミュレータを起こす'; assert_normal
echo 'Scenario: 正常に最後まで走ると、6 段の結果を残してエミュレータを止める'; assert_normal
echo 'Scenario: KEEP=1 なら、通った後にエミュレータを止めない'; assert_keep
echo 'Scenario: emulator かシステムイメージが無いときは、今の文言で止まる'; assert_missing
echo 'Scenario: AVD が無ければ作ってから起こす'; assert_avd_create
echo 'Scenario: 計測テストの台本の行の形の検査は、今のまま通る'
tools/test-usage-volume.sh
