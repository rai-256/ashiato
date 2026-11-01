#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <official|dev> <keystore> | verify <keystore> | --self-test" >&2; exit 2; }

verify_key() {
  local keystore="$1" password end end_epoch minimum_epoch
  [[ -f "$keystore" ]] || die "鍵の置き場が無い"
  IFS= read -r password || die "合言葉を標準入力から読めない"
  [[ -n "$password" ]] || die "合言葉が空"
  local details
  details="$(keytool -list -v -keystore "$keystore" -storepass "$password" 2>/dev/null)" || die "鍵を読めない"
  grep -Eq 'Subject Public Key Algorithm: 4096-bit RSA key' <<<"$details" || die "鍵は RSA 4096 bit ではない"
  end="$(sed -n 's/^Valid from: .* until: //p' <<<"$details" | head -n1)"
  [[ -n "$end" ]] || die "鍵の有効期限を読めない"
  end_epoch="$(date -d "$end" +%s 2>/dev/null)" || die "鍵の有効期限を読めない"
  minimum_epoch="$(date -d '+25 years' +%s)"
  (( end_epoch >= minimum_epoch )) || die "鍵の有効期限が 25 年未満"
}

make_key() {
  local kind="$1" keystore="$2" password alias
  [[ "$kind" == official || "$kind" == dev ]] || usage
  [[ -n "$keystore" ]] || usage
  [[ ! -e "$keystore" ]] || die "鍵の置き場が既にある"
  IFS= read -r password || die "合言葉を標準入力から読めない"
  [[ -n "$password" ]] || die "合言葉が空"
  alias="ashiato-$kind"
  mkdir -p "$(dirname "$keystore")"
  keytool -genkeypair -noprompt -keystore "$keystore" -storetype JKS \
    -storepass "$password" -keypass "$password" -alias "$alias" \
    -keyalg RSA -keysize 4096 -validity 10000 \
    -dname "CN=ashiato $kind, OU=release, O=ashiato, C=JP" >/dev/null 2>&1
  printf '%s\n' "$password" | verify_key "$keystore"
}

self_test() {
  local work="$(mktemp -d "${TMPDIR:-/tmp}/make-key.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  local password='self-test-password'
  echo "== Scenario: 鍵を作る台本は、決めた種類・長さ・有効期限の鍵だけを作る"
  printf '%s\n' "$password" | "$0" official "$work/official.jks"
  printf '%s\n' "$password" | "$0" verify "$work/official.jks"
  local details
  details="$(keytool -list -v -keystore "$work/official.jks" -storepass "$password" 2>/dev/null)"
  grep -Eq 'Subject Public Key Algorithm: 4096-bit RSA key' <<<"$details"
  echo "== Scenario: 鍵を作る台本は、決めた長さに満たない鍵を確かめで落とす"
  keytool -genkeypair -noprompt -keystore "$work/short.jks" -storepass "$password" -keypass "$password" \
    -alias short -keyalg RSA -keysize 2048 -validity 10000 \
    -dname 'CN=short' >/dev/null 2>&1
  if printf '%s\n' "$password" | "$0" verify "$work/short.jks" >/dev/null 2>&1; then
    die "RSA 2048 bit の鍵を受け入れた"
  fi
  echo "OK: 鍵の種類・長さ・有効期限を検査した"
}

case "${1:-}" in
  --self-test) self_test ;;
  verify) [[ $# == 2 ]] || usage; verify_key "$2" ;;
  official|dev) [[ $# == 2 ]] || usage; make_key "$1" "$2" ;;
  *) usage ;;
esac
