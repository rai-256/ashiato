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
# --- 生存信号（ST02 / FR-78）
# **grep では守れない。** 同じ語が docstring や別の行にも出るので空振りする
# （実測: `capturable` を `capturableX` に改名しても緑のままだった）。
# 欄名の一致は Kotlin 側の `HeartbeatContractTest` が**実際の直列化**で確かめ、
# ここはその試験が持つ一覧と `docs/openapi.json` を突き合わせる（両方を同時に書き換えないと通らない）。
hbtest=collector-android/app/src/test/kotlin/dev/ashiato/collector/HeartbeatContractTest.kt
kt_fields=$(sed -n '/CONTRACT-FIELDS-BEGIN/,/CONTRACT-FIELDS-END/p' "$hbtest" \
  | grep -oE '"[a-z_]+"' | tr -d '"' | sort)
api_fields=$(jq -r '.components.schemas.HeartbeatRequest.properties | keys[]' docs/openapi.json | sort)
if [ "$kt_fields" != "$api_fields" ]; then
  echo "  NG 生存信号の欄が契約とずれている"
  diff <(echo "$api_fields") <(echo "$kt_fields") | sed 's/^/     /'
  bad=1
else
  echo "生存信号の欄名 OK（$(echo "$api_fields" | wc -l) 欄。実際の直列化は HeartbeatContractTest が見る）"
fi
[ "$bad" -eq 0 ] || { echo "NG: 契約とKotlin側の欄名がずれている"; exit 1; }
echo "収集側の欄名 OK（要求 $(jq -r '.components.schemas.IngestRequest.properties|keys|length' docs/openapi.json) / 応答 $(jq -r '.components.schemas.IngestResult.properties|keys|length' docs/openapi.json)）"

echo "API の契約 OK"
