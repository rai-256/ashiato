#!/usr/bin/env bash
# エミュレータを立てて、収集側の計測テスト（collector-android/app/src/androidTest）を走らせる（2026-09-14）。
#
#   ./tools/android-emulator.sh            # AVD が無ければ作る → 起動（画面なし）→ connectedDebugAndroidTest → 止める
#   KEEP=1 ./tools/android-emulator.sh     # 終わってもエミュレータを止めない（続けて手で触るとき）
#
# 前提: tools/android-env.sh の SDK に emulator と system-images;android-35;google_apis;x86_64 が入っていること
#   sdkmanager --install "emulator" "system-images;android-35;google_apis;x86_64"
# WSL2 では /dev/kvm が要る（無ければ数十倍遅い）。CI（ubuntu）は KVM を有効にしてから同じことをする。
#
# 実機を adb で繋いでいるときはエミュレータを立てず、そのまま同じテストを実機で走らせる:
#   (cd collector-android && ./gradlew :app:connectedDebugAndroidTest -Pashiato.baseUrl=http://127.0.0.1:18787)
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
. ./tools/android-env.sh
export PATH="$ANDROID_HOME/emulator:$PATH"

AVD="${AVD:-ashiato-api35}"
IMAGE="system-images;android-35;google_apis;x86_64"
[ -x "$ANDROID_HOME/emulator/emulator" ] || { echo "error: emulator が無い。sdkmanager --install \"emulator\" \"$IMAGE\"" >&2; exit 1; }
[ -d "$ANDROID_HOME/system-images/android-35/google_apis/x86_64" ] || { echo "error: $IMAGE が無い" >&2; exit 1; }

if ! avdmanager list avd -c 2>/dev/null | grep -qx "$AVD"; then
  echo "== AVD $AVD を作る"
  echo no | avdmanager create avd -n "$AVD" -k "$IMAGE" -d pixel_6 >/dev/null
fi

cleanup() { [ "${KEEP:-}" = "1" ] || { adb -s "${SERIAL:-emulator-5554}" emu kill >/dev/null 2>&1 || true; }; }
trap cleanup EXIT

echo "== エミュレータを起動（画面なし）"
emulator -avd "$AVD" -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot >/dev/null 2>&1 &
adb wait-for-device
for _ in $(seq 1 180); do
  [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] && break
  sleep 2
done
[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] || { echo "error: 起動が終わらない" >&2; exit 1; }
adb shell settings put global window_animation_scale 0 >/dev/null
adb shell settings put global transition_animation_scale 0 >/dev/null
adb shell settings put global animator_duration_scale 0 >/dev/null

echo "== 計測テスト"
# -Pashiato.baseUrl は平文 HTTP を 127.0.0.1（端末の中のテスト用サーバ）へ許すためだけ。送信先には使われない
(cd collector-android && ./gradlew -q :app:connectedDebugAndroidTest -Pashiato.baseUrl=http://127.0.0.1:18787)
echo "== 結果: collector-android/app/build/reports/androidTests/connected/"
