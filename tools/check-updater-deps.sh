#!/usr/bin/env bash
# 更新係と PC 収集アプリの依存境界を検査する（ST57 / Task 2）。
set -euo pipefail
cd "$(dirname "$0")/.."

db=(sqlx tokio-postgres postgres diesel sea-orm)
roots=(webpki-roots webpki-root-certs)
offsite=(reqwest ureq isahc surf attohttpc curl lettre object_store opendal)

scan_exact() {
  local name
  while read -r name _; do
    for item in "$@"; do [[ "$name" == "$item" ]] && echo "$name"; done
  done | sort -u
}

if [[ "${1:-}" == --self-test ]]; then
  fake=$'ashiato-updater v0.1.0\nsqlx v0.8.0'
  [[ -n "$(printf '%s\n' "$fake" | scan_exact "${db[@]}")" ]] || { echo 'error: DB 部品の偽入力を拾えない' >&2; exit 1; }
  if [[ -z "$(printf '%s\n' 'ashiato-updater v0.1.0' | scan_exact "${db[@]}")" ]]; then :; else echo 'error: DB 検査が偽陽性' >&2; exit 1; fi
  server_fake=$'ashiato-server v0.1.0\nashiato-updater v0.1.0'
  [[ -n "$(printf '%s\n' "$server_fake" | scan_exact ashiato-updater)" ]] || { echo 'error: サーバ依存の偽入力を拾えない' >&2; exit 1; }
  shared_fake=$'ashiato-update-check v0.1.0\nreqwest v0.12.0'
  [[ -n "$(printf '%s\n' "$shared_fake" | scan_exact "${offsite[@]}")" ]] || { echo 'error: 共有 crate の外部接続部品を拾えない' >&2; exit 1; }
  roots_fake=$'ashiato-updater v0.1.0\nashiato-collector-windows v0.1.0\nwebpki-roots v0.26.0'
  [[ -n "$(printf '%s\n' "$roots_fake" | scan_exact "${roots[@]}")" ]] || { echo 'error: 根の一覧の偽入力を拾えない' >&2; exit 1; }
  echo "== Scenario: 更新係は DB に繋ぐ部品を持たない"
  echo "== Scenario: サーバは更新係の部品に依らない"
  echo "== Scenario: 更新係と PC の収集アプリは根の一覧を同梱しない"
  exit 0
fi

updater="$(cargo tree -p ashiato-updater -e normal --prefix none)"
collector="$(cargo tree -p ashiato-collector-windows -e normal --prefix none)"
server="$(cargo tree -p ashiato-server -e normal --prefix none)"
shared="$(cargo tree -p ashiato-update-check -e normal --prefix none)"

if hits=$(printf '%s\n' "$updater" | scan_exact "${db[@]}"); [[ -n "$hits" ]]; then
  echo "error: 更新係に DB 部品がある: $hits" >&2; exit 1
fi
if grep -q '^ashiato-updater ' <<<"$server"; then
  echo 'error: サーバが更新係に依存している' >&2; exit 1
fi
if hits=$(printf '%s\n' "$shared" | scan_exact "${offsite[@]}" "${db[@]}" "${roots[@]}"); [[ -n "$hits" ]]; then
  echo "error: 共有の更新判定に外部接続または根の部品がある: $hits" >&2; exit 1
fi
if hits=$(printf '%s\n%s\n' "$updater" "$collector" | scan_exact "${roots[@]}"); [[ -n "$hits" ]]; then
  echo "error: 更新係または PC 収集アプリに根の一覧がある: $hits" >&2; exit 1
fi
echo '== Scenario: 更新係は DB に繋ぐ部品を持たない'
echo '== Scenario: サーバは更新係の部品に依らない'
echo '== Scenario: 更新係と PC の収集アプリは根の一覧を同梱しない'
