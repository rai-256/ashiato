#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
printf '%100s\n%100s\n' a b > "$tmp"

out=$(USAGE_VOLUME_SAMPLE_FILE="$tmp" tools/usage-volume.sh)
printf '%s\n' "$out" | grep -q '^events_per_hour=2$'
printf '%s\n' "$out" | grep -q '^bytes_per_event=101$'
printf '%s\n' "$out" | grep -q '^events_per_day=48$'
printf '%s\n' "$out" | grep -q '^bytes_90_days=436320$'

set +e
USAGE_VOLUME_SAMPLE_FILE="$tmp" USAGE_VOLUME_LIMIT_BYTES=1 tools/usage-volume.sh >/dev/null
rc=$?
set -e
[ "$rc" -eq 1 ] || { echo "上限を超えても rc=$rc" >&2; exit 1; }

# 観測の長さで割る（実機の 10 日の経路と同じ式）。2 件 / 2 日 = 1 日 1 件
out=$(USAGE_VOLUME_SAMPLE_FILE="$tmp" USAGE_VOLUME_SPAN_SECONDS=172800 tools/usage-volume.sh)
printf '%s\n' "$out" | grep -q '^events_per_day=1$' || { echo "観測の長さで割っていない: $out" >&2; exit 1; }
printf '%s\n' "$out" | grep -q '^bytes_90_days=9090$' || { echo "90 日の式が違う: $out" >&2; exit 1; }

# ---- rc=1 は「上限を超えた」だけ。測れなかったら rc=2（deep.md Q10 / code-verify R31）
# 実測: エミュレータが落ちきる前に拾って `adb: device offline` になった回が rc=1 で、上限超過と区別できなかった
fake=$(mktemp -d)
trap 'rm -f "$tmp"; rm -rf "$fake"' EXIT
cat > "$fake/adb" <<'ADB'
#!/usr/bin/env bash
# FAKE_ADB_DEVICES: `adb devices` の本文 / FAKE_ADB_STATE: get-state / FAKE_ADB_APPOPS: appops get の出力
[ "${FAKE_ADB_FAIL:-}" = 1 ] && { echo "adb: device offline" >&2; exit 1; }
args="$*"
case "$args" in
  devices) printf 'List of devices attached\n%b' "${FAKE_ADB_DEVICES:-}" ;;
  *get-state*) echo "${FAKE_ADB_STATE:-device}" ;;
  *"pm path"*) echo "package:/data/app/base.apk" ;;
  *"appops get"*) echo "${FAKE_ADB_APPOPS:-GET_USAGE_STATS: allow}" ;;
  *) exit 1 ;;
esac
ADB
chmod +x "$fake/adb"

expect_rc() {   # expect_rc <期待する rc> <説明> <コマンド…>
  local want=$1 what=$2 got
  shift 2
  set +e
  "$@" >/dev/null 2>&1
  got=$?
  set -e
  [ "$got" -eq "$want" ] || { echo "$what: rc=$got（期待: $want）" >&2; exit 1; }
}
expect_rc 2 "adb が失敗した" env PATH="$fake:$PATH" FAKE_ADB_FAIL=1 ANDROID_HOME=/nonexistent tools/usage-volume.sh
expect_rc 2 "エミュレータが無い" env PATH="$fake:$PATH" ANDROID_HOME=/nonexistent tools/usage-volume.sh
expect_rc 2 "ANDROID_HOME が未設定（set -u）" env -u ANDROID_HOME PATH="$fake:$PATH" tools/usage-volume.sh
expect_rc 2 "採寸が 0 件" env USAGE_VOLUME_SAMPLE_FILE=/dev/null tools/usage-volume.sh
expect_rc 2 "知らない引数" tools/usage-volume.sh --bogus
expect_rc 2 "実機が無い" env PATH="$fake:$PATH" tools/usage-volume.sh --device
expect_rc 2 "実機が 2 台" env PATH="$fake:$PATH" FAKE_ADB_DEVICES='AAA\tdevice\nBBB\tdevice\n' tools/usage-volume.sh --device
expect_rc 2 "実機が offline" env PATH="$fake:$PATH" FAKE_ADB_STATE=offline tools/usage-volume.sh --device AAA
# 実機の経路は本人の端末に許可を付けない。許されていなければ測らずに止まる
expect_rc 2 "実機で利用状況へのアクセスが無い" \
  env PATH="$fake:$PATH" FAKE_ADB_DEVICES='AAA\tdevice\n' FAKE_ADB_APPOPS='GET_USAGE_STATS: default' tools/usage-volume.sh --device
grep -Fq 'DEVICE_WINDOW_SECONDS=864000' tools/usage-volume.sh \
  || { echo "実機の経路が直近 10 日を数えていない" >&2; exit 1; }
grep -q 'appops set "\$PKG" GET_USAGE_STATS' tools/usage-volume.sh && \
  ! sed -n '/^device_sample()/,/^}/p' tools/usage-volume.sh | grep -q 'appops set' \
  || { echo "実機の経路で appops set している（本人の端末の状態を変える）" >&2; exit 1; }

# emulator が起動に失敗して adb が一度も ready を返さなくても、待ちは有限で終わる。
set +e
ADB=/bin/false tools/wait-android-boot.sh emulator-5554 1 0 >/dev/null 2>&1
rc=$?
set -e
[ "$rc" -eq 1 ] || { echo "端末が現れないとき rc=$rc（期待: 1）" >&2; exit 1; }

# 採寸専用テストは usage access とイベント生成を前提にするため、通常の計測 suite には混ぜない。
volume_test=dev.ashiato.collector.UsageVolumeInstrumentedTest
grep -Fq -- "-Pandroid.testInstrumentationRunnerArguments.notClass=\"$volume_test\"" tools/android-emulator.sh \
  || { echo "android-emulator.sh が採寸専用テストを通常 suite から除外していない" >&2; exit 1; }
grep -Fq -- "TEST_CLASS=$volume_test" tools/usage-volume.sh && grep -Fq -- '-e class "$TEST_CLASS"' tools/usage-volume.sh \
  || { echo "usage-volume.sh が採寸専用テストを明示実行していない" >&2; exit 1; }

# 計測 suite は AVD に残った前の台本の状態（usage-volume.sh が付けた usage access の許可）を持ち込まない（R29）。
# 1 段目の gradle より前にアプリを外していること
first_gradle=$(grep -n 'connectedDebugAndroidTest' tools/android-emulator.sh | grep -v '^[0-9]*:#' | head -1 | cut -d: -f1)
uninstall=$(grep -n '^adb uninstall dev.ashiato.collector ' tools/android-emulator.sh | head -1 | cut -d: -f1)
[ -n "$uninstall" ] && [ "$uninstall" -lt "$first_gradle" ] \
  || { echo "android-emulator.sh が suite の前にアプリを外していない" >&2; exit 1; }
