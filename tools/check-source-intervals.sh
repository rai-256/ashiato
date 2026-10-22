#!/usr/bin/env bash
# 登録簿の端末と PC の取得の間隔が、収集側の定数と一致することを見る（ST13 / design D3 / D9）。
# 登録簿の値は収集側の定数の写し（収集側は登録簿を読まない）。写しがずれると稼働状況の格子の見出しが嘘をつく。
# サーバの crate は collector-windows に依存しないので cargo test では比べない（spec-review R19）。DB は要らない。
set -euo pipefail
cd "$(dirname "$0")/.."

SQL=migrations/202610211000_source_acquisition.sql
RUST=crates/collector-windows/src/history/fetch.rs
KT=collector-android/app/src/main/kotlin/dev/ashiato/collector

# Kotlin の `const val NAME: Long = 60_000` / `30 * 60 * 1000L` をミリ秒の整数にして返す
kotlin_ms() {
  local expr
  expr=$(grep -hE "const val $1\b" "$KT"/*.kt | head -1 | sed -E 's/.*=[[:space:]]*//; s/_//g; s/L\b//g') || true
  [ -n "$expr" ] || { echo "  NG Kotlin の定数 $1 が見つからない" >&2; return 1; }
  [[ "$expr" =~ ^[0-9*[:space:]]+$ ]] || { echo "  NG $1 の式 '$expr' を読めない" >&2; return 1; }
  echo $((expr))
}
# 移行の SQL の VALUES の行から取得の間隔を返す
sql_interval() {
  local v
  v=$(grep -E "^[[:space:]]*\('$1'," "$SQL" | head -1 | sed -E 's/.*,[[:space:]]*([0-9]+)\)[,[:space:]]*$/\1/')
  [[ "$v" =~ ^[0-9]+$ ]] || { echo "  NG $SQL に $1 の取得の間隔が無い" >&2; return 1; }
  echo "$v"
}

bad=0
check() { # 名前 登録簿の秒 収集側の秒
  if [ "$2" = "$3" ]; then echo "  ok $1: $2 秒"; else echo "  NG $1: 登録簿 $2 秒 != 収集側 $3 秒"; bad=1; fi
}

hours=$(grep -E 'pub const HISTORY_INTERVAL' "$RUST" | sed -nE 's/.*Duration::hours\(([0-9]+)\).*/\1/p')
[ -n "$hours" ] || { echo "  NG $RUST の HISTORY_INTERVAL を読めない（Duration::hours 以外の書き方か）"; exit 1; }
check c02-browser-history "$(sql_interval c02-browser-history)" "$((hours * 3600))"
check c01-location "$(sql_interval c01-location)" "$(( $(kotlin_ms FIX_INTERVAL_MS) / 1000 ))"
check c01-app-usage "$(sql_interval c01-app-usage)" "$(( $(kotlin_ms USAGE_INTERVAL_MS) / 1000 ))"
check c01-clock "$(sql_interval c01-clock)" "$(( $(kotlin_ms CLOCK_SKEW_INTERVAL_MS) / 1000 ))"

[ "$bad" -eq 0 ] && echo "取得の間隔 OK" || { echo "取得の間隔 NG"; exit 1; }
