#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lock_file="${PGSQL_LOCK_FILE:-$root/tools/release/pgsql.lock}"
# docker compose の COMPOSE_FILE（`.env` が `a.yml:b.yml` の並びで置く）は読まない。専用の名前で差し替える（review R57）
compose_file="${PG_MAJOR_COMPOSE_FILE:-$root/docker-compose.yml}"
die() { echo "NG: $*" >&2; exit 1; }

major_from_lock() {
  sed -n 's/^version=\([0-9][0-9]*\)\..*/\1/p' "$lock_file" | head -n 1
}

major_from_compose() {
  sed -n 's/.*image: postgres:\([0-9][0-9]*\).*/\1/p' "$compose_file" | head -n 1
}

check() {
  local bundled development
  [[ -f "$lock_file" ]] || die "pgsql.lock が無い"
  [[ -f "$compose_file" ]] || die "docker-compose.yml が無い"
  bundled="$(major_from_lock)"
  development="$(major_from_compose)"
  [[ -n "$bundled" && -n "$development" ]] || die 'PostgreSQL のメジャー版を読めない'
  [[ "$bundled" == "$development" ]] || die "PostgreSQL のメジャー版が不一致"
  echo "OK: PostgreSQL のメジャー版が一致"
}

self_test() {
  local work bad_lock bad_compose
  work="$(mktemp -d "${TMPDIR:-/tmp}/check-pg-major.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  bad_lock="$work/pgsql.lock"
  bad_compose="$work/docker-compose.yml"
  sed 's/^version=.*/version=16.9-1/' "$lock_file" >"$bad_lock"
  cp "$compose_file" "$bad_compose"
  if PGSQL_LOCK_FILE="$bad_lock" PG_MAJOR_COMPOSE_FILE="$bad_compose" "$0"; then
    die 'メジャー版の不一致を受け入れた'
  fi
  # `.env` を読んだ端末（COMPOSE_FILE が並び）でも、既定の docker-compose.yml を読む
  COMPOSE_FILE="$root/docker-compose.yml:$root/docker-compose.subnet.yml" "$0" >/dev/null ||
    die 'COMPOSE_FILE の並びで docker-compose.yml を読めなかった'
  echo '== Scenario: 配る DB と開発の DB は同じメジャー版'
  echo 'OK: PostgreSQL メジャー版検査の自己テストに成功'
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
[[ $# == 0 ]] || die "使い方: $0 [--self-test]"
check
