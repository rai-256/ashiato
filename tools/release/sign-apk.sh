#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <apk> <official|dev> <出力> | --self-test" >&2; exit 2; }
keytool_en() { keytool -J-Duser.language=en -J-Duser.country=US "$@"; }

apksigner_bin() {
  if command -v apksigner >/dev/null 2>&1; then command -v apksigner; return; fi
  find "${ANDROID_HOME:-}"/build-tools -name apksigner -type f -print -quit 2>/dev/null | head -n1
}

apk_id() {
  local analyzer
  analyzer="$(command -v apkanalyzer || true)"
  [[ -n "$analyzer" ]] || die "apkanalyzer が無い"
  "$analyzer" manifest application-id "$1" 2>/dev/null || die "APK の識別子を読めない"
}

sign_apk() {
  local apk="$1" channel="$2" output="$3" package expected b64 store_password key_alias key_password tmp_keystore signed_tmp signer
  [[ -f "$apk" ]] || die "APK が無い"
  [[ "$channel" == official || "$channel" == dev ]] || usage
  [[ "$apk" != "$output" ]] || die "入力と出力が同じ"
  package="$(apk_id "$apk")"
  expected='dev.ashiato.collector'
  [[ "$channel" == dev ]] && expected+='.dev'
  [[ "$package" == "$expected" ]] || die "APK の識別子とチャンネルが違う"
  b64="${ASHIATO_SIGNING_KEYSTORE_B64:-}"
  store_password="${ASHIATO_SIGNING_STORE_PASSWORD:-}"
  key_alias="${ASHIATO_SIGNING_KEY_ALIAS:-}"
  key_password="${ASHIATO_SIGNING_KEY_PASSWORD:-}"
  [[ -n "$b64" && -n "$store_password" && -n "$key_alias" && -n "$key_password" ]] || die "署名の鍵または合言葉が無い"
  signer="$(apksigner_bin)"
  [[ -n "$signer" && -x "$signer" ]] || die "apksigner が無い"
  tmp_keystore="$(mktemp "${TMPDIR:-/tmp}/ashiato-signing.XXXXXX.jks")"
  signed_tmp="$(mktemp "${TMPDIR:-/tmp}/ashiato-signed.XXXXXX.apk")"
  trap "rm -f '$tmp_keystore' '$signed_tmp' '${signed_tmp}.idsig'" EXIT
  printf '%s' "$b64" | base64 --decode >"$tmp_keystore" || die "署名の鍵を復元できない"
  mkdir -p "$(dirname "$output")"
  "$signer" sign --ks "$tmp_keystore" --ks-pass "pass:$store_password" \
    --ks-key-alias "$key_alias" --key-pass "pass:$key_password" --out "$signed_tmp" "$apk" >/dev/null
  [[ -s "$signed_tmp" ]] || die "署名済み APK が無い"
  mv "$signed_tmp" "$output"
}

self_test() {
  local work="$(mktemp -d "${TMPDIR:-/tmp}/sign-apk.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  local password='self-test-password' official_b64 dev_b64 official_cert dev_cert apk_official apk_dev apk_debug
  printf '%s\n' "$password" | "$root/tools/release/make-key.sh" official "$work/official.jks"
  printf '%s\n' "$password" | "$root/tools/release/make-key.sh" dev "$work/dev.jks"
  official_b64="$(base64 -w0 "$work/official.jks")"
  dev_b64="$(base64 -w0 "$work/dev.jks")"
  local build_out="$work/build"
  mkdir -p "$build_out"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION=0.1.0 ASHIATO_APK_UNSIGNED=1 "$root/scripts/quiet-run" sign-build-official -- "$root/tools/release/build.sh" apk "$build_out"
  mv "$build_out/ashiato-collector-0.1.0.apk" "$work/official-unsigned.apk"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION=0.1.0-dev.20261101 ASHIATO_APK_UNSIGNED=1 "$root/scripts/quiet-run" sign-build-dev -- "$root/tools/release/build.sh" apk "$build_out"
  mv "$build_out/ashiato-collector-0.1.0-dev.20261101.apk" "$work/dev-unsigned.apk"
  echo "== Scenario: 正式版の APK は、正式な識別子と正式な鍵で署名されている"
  ASHIATO_SIGNING_KEYSTORE_B64="$official_b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-official ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    "$0" "$work/official-unsigned.apk" official "$work/official.apk"
  echo "== Scenario: 開発版の APK は、.dev の識別子と開発版の鍵で署名されている"
  ASHIATO_SIGNING_KEYSTORE_B64="$dev_b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    "$0" "$work/dev-unsigned.apk" dev "$work/dev.apk"
  official_cert="$("$(apksigner_bin)" verify --print-certs "$work/official.apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n1)"
  dev_cert="$("$(apksigner_bin)" verify --print-certs "$work/dev.apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n1)"
  local official_key_cert dev_key_cert
  official_key_cert="$(keytool_en -list -v -keystore "$work/official.jks" -storepass "$password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  dev_key_cert="$(keytool_en -list -v -keystore "$work/dev.jks" -storepass "$password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  official_cert="$(tr -d ':' <<<"$official_cert" | tr 'A-F' 'a-f')"
  dev_cert="$(tr -d ':' <<<"$dev_cert" | tr 'A-F' 'a-f')"
  [[ -n "$official_cert" && "$official_cert" == "$official_key_cert" ]]
  [[ -n "$dev_cert" && "$dev_cert" == "$dev_key_cert" && "$dev_cert" != "$official_key_cert" ]]
  echo "== Scenario: 識別子とチャンネルの鍵を取り違えると、署名しない"
  if ASHIATO_SIGNING_KEYSTORE_B64="$official_b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-official ASHIATO_SIGNING_KEY_PASSWORD="$password" "$0" "$work/dev-unsigned.apk" official "$work/wrong.apk" >/dev/null 2>&1; then die '開発版 APK に正式版を署名した'; fi
  [[ ! -e "$work/wrong.apk" ]]
  if ASHIATO_SIGNING_KEYSTORE_B64="$dev_b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" "$0" "$work/official-unsigned.apk" dev "$work/wrong.apk" >/dev/null 2>&1; then die '正式版 APK に開発版を署名した'; fi
  [[ ! -e "$work/wrong.apk" ]]
  echo "== Scenario: 鍵が置き場に無ければ、署名しない APK を公開しない"
  if env -u ASHIATO_SIGNING_KEYSTORE_B64 "$0" "$work/official-unsigned.apk" official "$work/missing.apk" >/dev/null 2>&1; then die '鍵無しで署名した'; fi
  [[ ! -e "$work/missing.apk" ]]
  echo "== Scenario: PR の組み立ての APK は debug の鍵で署名される"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION=0.1.0 "$root/scripts/quiet-run" sign-build-debug -- "$root/tools/release/build.sh" apk "$build_out"
  apk_debug="$build_out/ashiato-collector-0.1.0.apk"
  local apk_cert debug_cert
  apk_cert="$("$(apksigner_bin)" verify --print-certs "$apk_debug" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n1)"
  debug_cert="$(keytool_en -list -v -keystore "${HOME}/.android/debug.keystore" -storepass android 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  [[ -n "$apk_cert" && "$apk_cert" == "$debug_cert" ]] || die 'debug 鍵の APK ではない'
  echo "OK: 正式版・開発版・取り違え・鍵無し・debug 鍵を検査した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 3 ]] || usage
sign_apk "$@"
