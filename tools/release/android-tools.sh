# shellcheck shell=bash
# Android SDK の道具（apksigner・apkanalyzer・aapt2 など）の在処を 1 つの規則で探す（source して使う）。
# 順は PATH → $ANDROID_HOME（無ければ $ANDROID_SDK_ROOT）/cmdline-tools/latest/bin → build-tools の最新の版。
# apksigner と aapt2 は build-tools の下にだけあり、どの runner の PATH にも無い（ST52 final review R7）。
# 偽の道具を PATH の先に置く自己試験は、PATH が先なのでそのまま効く。
android_tool() {
  local name="$1" home="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}" found
  if found="$(command -v "$name" 2>/dev/null)"; then
    printf '%s\n' "$found"
    return 0
  fi
  [[ -n "$home" ]] || return 1
  if [[ -x "$home/cmdline-tools/latest/bin/$name" ]]; then
    printf '%s\n' "$home/cmdline-tools/latest/bin/$name"
    return 0
  fi
  found="$(find "$home/build-tools" -mindepth 2 -maxdepth 2 -name "$name" -type f -perm -111 2>/dev/null | sort -V | tail -n1)"
  [[ -n "$found" ]] || return 1
  printf '%s\n' "$found"
}
