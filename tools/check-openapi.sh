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
echo "API の契約 OK"
