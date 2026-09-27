#!/usr/bin/env bash
# adb の端末待ちを有限回にする。emulator 自体が起動に失敗しても呼び出し元を止め続けない。
set -u

serial=${1:?serial が必要}
attempts=${2:-180}
delay=${3:-2}
adb_cmd=${ADB:-adb}

for _ in $(seq 1 "$attempts"); do
  if [ "$("$adb_cmd" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; then
    exit 0
  fi
  sleep "$delay"
done
exit 1
