#!/usr/bin/env bash
# 収集アプリが loopback 以外へ平文で送らないことを、生成物と build の失敗で確かめる（ST28 design D13）。
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
. ./tools/android-env.sh
cd collector-android
xml=app/build/generated/res/nsconfig/xml/network_security_config.xml

# (a) https:// の宛先に組み立てる。その宛先にも、loopback 以外の domain-config にも平文の許可が無い
./gradlew -q :app:generateNetworkSecurityConfig -Pashiato.baseUrl=https://example.invalid:1
[ -f "$xml" ] || { echo "NG 生成物が無い: $xml" >&2; exit 1; }
if grep -q 'example.invalid' "$xml"; then
  echo "NG 接続先の host に例外を出している" >&2; cat "$xml" >&2; exit 1
fi
# domain-config の中の <domain> は localhost / 127.0.0.1 だけ
others=$(grep -oE '<domain[^>]*>[^<]*</domain>' "$xml" | sed -E 's/<[^>]*>//g' | grep -vxE 'localhost|127\.0\.0\.1' || true)
[ -z "$others" ] || { echo "NG loopback 以外に平文を許している: $others" >&2; exit 1; }
grep -q '<base-config cleartextTrafficPermitted="false"' "$xml" || { echo "NG base-config が平文を禁じていない" >&2; exit 1; }
echo "Scenario: 収集アプリは接続先の宛先にも平文を許さない"

# (b) http:// で loopback 以外の宛先は組み立てを落とし、理由を出す
if out=$(./gradlew -q :app:assembleDebug -Pashiato.baseUrl=http://example.invalid:1 2>&1); then
  echo "NG 平文の接続先で組み立てが通った" >&2; exit 1
fi
echo "$out" | grep -q '暗号化されていない' || { echo "NG 理由が出ていない" >&2; echo "$out" >&2; exit 1; }
# 同じ Scenario の別の形（印は下に 1 つだけ）: [::1] は domain-config の許可に無いので、組み立ても落とす（final review R10）
if out=$(./gradlew -q :app:generateNetworkSecurityConfig '-Pashiato.baseUrl=http://[::1]:1' 2>&1); then
  echo "NG 平文の許可に無い [::1] で組み立てが通った" >&2; exit 1
fi
echo "Scenario: 平文の接続先では収集アプリを組み立てられない"
