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
# 出力の署名者名は build-tools の版や SDK 範囲で変わる。署名者名を列挙せず、最初の
# `certificate SHA-256 digest` 行だけを読む（最初の行が最初の署名者の証明書）。
apk_cert_from_print_certs() {
  sed -nE 's/^.* certificate SHA-256 digest: ([0-9A-Fa-f:]+)$/\1/p' | head -n1 | tr -d ':' | tr 'A-F' 'a-f'
}

# APK の最初の署名者の証明書の SHA-256。読めなければ空で返す（判定は呼び出し側）
apk_cert_sha256() {
  local signer="$1" apk="$2"
  # apksigner の失敗で pipefail の呼び出し側が理由を出さずに止まらないよう、失敗は空で返す（2026-11-02 CI で無言の rc=1）
  { "$signer" verify --print-certs "$apk" 2>/dev/null || true; } | apk_cert_from_print_certs
}

# 読めなかったときに出す手掛かり（apksigner の在処・版・出力の末尾）。証明書の指紋は秘密ではない
apk_cert_diag() {
  local signer="$1" apk="$2"
  printf 'apksigner=%s 版=%s\n' "$signer" "$("$signer" --version 2>&1 || true)"
  "$signer" verify --print-certs "$apk" 2>&1 | tail -n 8 || true
}
