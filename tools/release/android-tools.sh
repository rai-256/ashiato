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

# `apksigner verify --print-certs` の出力（標準入力）から、最初の署名者の証明書の SHA-256（小文字 64 桁）を取り出す。
# 出力の形は build-tools の版や SDK 範囲で変わる: `Signer #1 certificate SHA-256 digest: <hex>`、
# `Signer (minSdkVersion=…, maxSdkVersion=…) certificate SHA-256 digest: <hex>`、
# `V2 Signer: certificate SHA-256 digest: <hex>` のいずれも、この 1 つの関数で読む。
apk_cert_from_print_certs() {
  sed -nE 's/^(Signer (#[0-9]+|\(.*\))|V[0-9.]+ Signer:) certificate SHA-256 digest: ([0-9A-Fa-f:]+)$/\3/p' | head -n1 | tr -d ':' | tr 'A-F' 'a-f'
}

# APK の最初の署名者の証明書の SHA-256。読めなければ空で返す（判定は呼び出し側）
apk_cert_sha256() {
  local signer="$1" apk="$2"
  "$signer" verify --print-certs "$apk" 2>/dev/null | apk_cert_from_print_certs
}
