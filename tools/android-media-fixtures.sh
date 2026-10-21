#!/usr/bin/env bash
# 計測テスト（MediaPremisesInstrumentedTest）が使う写真と動画を、**別のアプリが入れたもの**として端末の一覧に入れる（ST09 tasks 2.2）。
#
# 位置を伏せるのは他のアプリの項目だけ（自分が入れた項目は伏せない）。なので計測テストの中で insert せず、
# `adb push` + メディアの再走査で入れる（所有者は com.android.shell）。`tools/android-emulator.sh` と CI の
# `android-instrumented` が、計測テストの前に呼ぶ。何度呼んでも同じ（上書きして再走査）。
set -euo pipefail
cd "$(dirname "$0")/.."
command -v adb >/dev/null 2>&1 || . ./tools/android-env.sh

M=collector-android/app/src/test/resources/media
PHOTO=/sdcard/Pictures/ashiato-premise-camera.jpg
VIDEO=/sdcard/Movies/ashiato-premise-video.mp4

# 起動の完了（sys.boot_completed）の直後は /sdcard がまだ繋がっていないことがある
# （実測 2026-10-21 CI: `adb push` が `secure_mkdirs() failed: No such file or directory`）。置き先が作れるまで待つ
for i in $(seq 1 90); do
  adb shell 'mkdir -p /sdcard/Pictures /sdcard/Movies' >/dev/null 2>&1 && break
  [ "$i" = 90 ] && { echo "error: 90 秒待っても /sdcard に置き先を作れない（外部の保存領域が繋がらない）" >&2; exit 1; }
  sleep 1
done

adb push "$M/camera-gps.jpg" "$PHOTO" >/dev/null
adb push "$M/video-gps.mp4" "$VIDEO" >/dev/null
for f in "$PHOTO" "$VIDEO"; do
  adb shell content call --uri content://media/ --method scan_file --arg "$f" >/dev/null
done

# 入ったことを確かめる（入っていなければ計測テストが「一覧に無い」で落ちるので、ここで原因を言う）
for kind in images video; do
  adb shell content query --uri "content://media/external/$kind/media" --projection _display_name | grep -q 'ashiato-premise-' \
    || { echo "error: $kind の一覧に ashiato-premise-* が入らなかった" >&2; exit 1; }
done
echo "== 計測テストの写真と動画を端末の一覧に入れた"
