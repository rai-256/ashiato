#!/usr/bin/env bash
set -euo pipefail

install_on_emulators() {
  local apk="$1" name
  local -a emulators=()
  mapfile -t emulators < <(
    adb devices 2>/dev/null |
      awk 'NR > 1 && $2 == "device" && $1 ~ /^emulator-/ { print $1 }'
  )
  if [ "${#emulators[@]}" -eq 0 ]; then
    printf 'エミュレータが無いので入れていない（手で入れる: `adb -s エミュレータ install -r %s`）\n' "$apk"
    return 0
  fi
  for name in "${emulators[@]}"; do
    if adb -s "$name" install -r "$apk" >/dev/null; then
      printf '%s\n' "$name"
    else
      printf 'エミュレータ %s への APK のインストールに失敗しました\n' "$name" >&2
      return 1
    fi
  done
}

self_test() {
  local work apk output calls
  work="$(mktemp -d "${TMPDIR:-/tmp}/verify-prep-device.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  mkdir -p "$work/bin"
  apk="$work/app-debug.apk"
  : > "$apk"

  cat > "$work/bin/adb" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = devices ]; then
  case "${ADB_SCENARIO:?}" in
    both)
      printf 'List of devices attached\nR58N12ABCDE\tdevice\nemulator-5554\tdevice\n'
      ;;
    physical)
      printf 'List of devices attached\nR58N12ABCDE\tdevice\n'
      ;;
  esac
elif [ "${1:-}" = -s ]; then
  printf '%s\n' "$*" >> "$ADB_LOG"
  if [ "${ADB_INSTALL_SCENARIO:-success}" = failure ]; then
    printf 'install failed\n' >&2
    exit 1
  fi
  printf 'Performing Streamed Install\nSuccess\n'
fi
EOF
  chmod +x "$work/bin/adb"

  echo '== Scenario: 実機とエミュレータが繋がっていれば、エミュレータにだけ入れる'
  : > "$work/calls"
  output="$(PATH="$work/bin:$PATH" ADB_SCENARIO=both ADB_LOG="$work/calls" install_on_emulators "$apk")"
  calls="$(cat "$work/calls")"
  [ "$calls" = "-s emulator-5554 install -r $apk" ]
  [ "$output" = 'emulator-5554' ]

  : > "$work/calls"
  if output="$(PATH="$work/bin:$PATH" ADB_SCENARIO=both ADB_INSTALL_SCENARIO=failure ADB_LOG="$work/calls" install_on_emulators "$apk")"; then
    exit 1
  fi
  [[ "$output" != *emulator-5554* ]]

  echo '== Scenario: 実機だけが繋がっていれば、何も入れない'
  : > "$work/calls"
  output="$(PATH="$work/bin:$PATH" ADB_SCENARIO=physical ADB_LOG="$work/calls" install_on_emulators "$apk")"
  [ ! -s "$work/calls" ]
  [[ "$output" == *'エミュレータが無いので入れていない'* ]]
  [[ "$output" == *'adb -s エミュレータ install -r'* ]]

  echo '== Scenario: 準備の台本は、端末を選ぶ手順を通して APK を入れる'
  local prep
  prep="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/verify-prep.sh"
  [ -f "$prep" ]
  ! grep -v install_on_emulators "$prep" | grep -Eq 'adb|install'
  [ "$(grep -c 'install_on_emulators' "$prep")" -eq 1 ]
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  if [ "${1:-}" = --self-test ]; then
    self_test
    echo '== OK'
    exit 0
  fi

  echo "使い方: $0 --self-test" >&2
  exit 2
fi
