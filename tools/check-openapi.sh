#!/usr/bin/env bash
# API の契約はコードから生成する（製造準備 A-1: 手書きしない）。
# コミットされた docs/openapi.json とコードがずれたら落とす。
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -q -p ashiato-server --bin openapi > /tmp/openapi.gen.json
if ! diff -u docs/openapi.json /tmp/openapi.gen.json; then
  echo "NG: API の契約がコードとずれている。cargo run -p ashiato-server --bin openapi > docs/openapi.json で更新する"
  exit 1
fi

# --- 収集側（Kotlin）が同じ欄名を読んでいるか（review R14）
# **これが無いと、サーバが `accepted` を改名した日に収集側は例外も出さずに全件 false と読む**
# （`ignoreUnknownKeys = true` + 既定値つき）。未送信は永久に取り除かれず、全部緑のまま。
kt=collector-android/app/src/main/kotlin/dev/ashiato/collector/IngestRequest.kt
bad=0
for field in $(jq -r '.components.schemas.IngestResult.properties | keys[]' docs/openapi.json); do
  grep -q "\b$field\b" "$kt" || { echo "  NG 応答の欄 $field を収集側が読んでいない（$kt）"; bad=1; }
done
# 送る側も同じ —— 欄名がずれると、同じ 1 件が別物として入る
for field in $(jq -r '.components.schemas.IngestRequest.properties | keys[]' docs/openapi.json); do
  grep -q "\b$field\b" "$kt" || { echo "  NG 要求の欄 $field を収集側が送っていない（$kt）"; bad=1; }
done
[ "$bad" -eq 0 ] || { echo "NG: 契約とKotlin側の欄名がずれている"; exit 1; }
echo "収集側の欄名 OK（要求 $(jq -r '.components.schemas.IngestRequest.properties|keys|length' docs/openapi.json) / 応答 $(jq -r '.components.schemas.IngestResult.properties|keys|length' docs/openapi.json)）"

echo "API の契約 OK"
