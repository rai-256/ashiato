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
grep -Fq -- "-e class $volume_test" tools/usage-volume.sh \
  || { echo "usage-volume.sh が採寸専用テストを明示実行していない" >&2; exit 1; }
