#!/usr/bin/env bash
# PostgreSQL の同梱 minor を postgresql.org の一覧と EDB の配布物で検査する。
# 17.11 の relDate=2026-08-13 は公式リリースノート
# (https://www.postgresql.org/docs/release/17.11/) の Release date と一致することを
# 2026-11-08 に確認した。versions.json の relDate は最新 minor の公開日として扱う。
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lock_file="${PGSQL_LOCK_FILE:-$root/tools/release/pgsql.lock}"
versions_url="${PG_VERSIONS_URL:-https://www.postgresql.org/versions.json}"
findings=""

die() {
  echo "error: 検査できなかった: $*" >&2
  exit 2
}

usage() {
  echo "使い方: $0 [--self-test | --findings <置き場>]" >&2
  exit 2
}

lock_value() {
  sed -n "s/^$1=//p" "$lock_file" | head -n 1
}

fetch_versions() {
  local destination="$1"
  if ! curl --fail --location --silent --show-error --max-time 20 "$versions_url" >"$destination"; then
    return 1
  fi
}

version_info() {
  python3 - "$1" "$2" <<'PY'
import json
import sys

path, major = sys.argv[1:]
try:
    with open(path, encoding="utf-8") as stream:
        rows = json.load(stream)
    row = next(row for row in rows if str(row.get("major")) == major)
    minor = str(row["latestMinor"])
    rel_date = str(row["relDate"])
    if not minor.isdigit() or len(rel_date) != 10:
        raise ValueError
except (OSError, ValueError, TypeError, KeyError, StopIteration, json.JSONDecodeError):
    raise SystemExit(1)
print(minor, rel_date)
PY
}

edb_base_from_lock() {
  # lock の version（例 17.6-1）はビルド番号まで持つ。-1 を決め打ちすると -2 の lock で外し損ね、
  # 最新の URL が壊れて absent と読まれる（review/code.md R28）。合わなければ検査できないとして止める。
  local url="$1" version="$2" suffix
  suffix="$version-windows-x64-binaries.zip"
  [[ "$url" == *"$suffix" ]] || return 1
  printf '%s' "${url%"$suffix"}"
}

edb_status() {
  local url="$1" status
  if [[ "$url" == file://* ]]; then
    [[ -f "${url#file://}" ]] && echo present || echo absent
    return 0
  fi
  status="$(curl --head --location --silent --show-error --max-time 15 --output /dev/null --write-out '%{http_code}' "$url" 2>/dev/null || echo 000)"
  case "$status" in
    200) echo present ;;
    403|404) echo absent ;;
    *) echo unknown ;;
  esac
}

write_finding() {
  # --findings の置き場は常にディレクトリ（file-issues が postgresql-minor.txt を探す。review/code.md R23）
  [[ -n "$findings" ]] || return 0
  mkdir -p "$findings"
  cat >"$findings/postgresql-minor.txt"
}

check() {
  local work bundled version major minor rel_date today age edb_base latest_url edb
  work="$(mktemp -d "${TMPDIR:-/tmp}/check-pg-minor.XXXXXX")"
  trap "rm -rf -- '$work'" EXIT
  [[ -f "$lock_file" ]] || die "pgsql.lock が無い"
  version="$(lock_value version)"
  bundled="${version%%-*}"
  major="${bundled%%.*}"
  [[ "$bundled" =~ ^[0-9]+\.[0-9]+$ ]] || die "pgsql.lock の version を読めない"
  fetch_versions "$work/versions.json" || die "postgresql.org の版の一覧"
  read -r minor rel_date < <(version_info "$work/versions.json" "$major") || die "版の一覧を読めない"
  [[ "$version" =~ ^[0-9]+\.[0-9]+-[0-9]+$ ]] || die "pgsql.lock の version を読めない"
  edb_base="${PG_EDB_URL_BASE:-}"
  if [[ -z "$edb_base" ]]; then
    edb_base="$(edb_base_from_lock "$(lock_value url)" "$version")" || die "pgsql.lock の url が version と合わない"
  fi
  # EDB の新しい minor の最初のビルドは -1。-1 が無ければ、まだ EDB に出ていない（absent）と読む。
  latest_url="${edb_base}${major}.${minor}-1-windows-x64-binaries.zip"
  edb="$(edb_status "$latest_url")"
  if [[ "$minor" == "${bundled##*.}" ]]; then
    echo "OK: PostgreSQL $bundled は最新 minor"
    return 0
  fi
  today="$(date -u +%F)"
  age="$(python3 - "$rel_date" "$today" <<'PY'
from datetime import date
import sys
print((date.fromisoformat(sys.argv[2]) - date.fromisoformat(sys.argv[1])).days)
PY
  )" || die "公開日を読めない"
  if [[ "$edb" == unknown ]]; then
    echo "PostgreSQL $bundled -> $major.$minor (EDB 不明, 公開日 $rel_date)"
    if (( age > 30 )); then
      echo "NG: 公開から 30 日を越えた"
      write_finding <<EOF
脆弱性: PostgreSQL $major の minor の遅れ
同梱: $bundled
最新: $major.$minor
EDB の zip: 不明
公開日: $rel_date
EOF
      return 1
    fi
    echo "EDB を待っている（公開から ${age} 日）"
    return 0
  fi
  echo "PostgreSQL $bundled -> $major.$minor (EDB の zip: $edb, 公開日 $rel_date)"
  if [[ "$edb" == present || "$age" -gt 30 ]]; then
    [[ "$edb" == present ]] || echo "NG: 公開から 30 日を越えた"
    write_finding <<EOF
脆弱性: PostgreSQL $major の minor の遅れ
同梱: $bundled
最新: $major.$minor
EDB の zip: $edb
公開日: $rel_date
EOF
    return 1
  fi
  echo "EDB を待っている（公開から ${age} 日）"
  return 0
}

self_test() {
  local tmp lock versions edb output rc recent boundary old
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/check-pg-minor-self-test.XXXXXX")"
  trap "rm -rf -- '$tmp'" EXIT
  lock="$tmp/pgsql.lock"
  versions="$tmp/versions.json"
  edb="$tmp/edb"
  mkdir -p "$edb"
  printf 'version=17.6-1\nurl=https://get.enterprisedb.com/postgresql/postgresql-17.6-1-windows-x64-binaries.zip\n' >"$lock"
  recent="$(date -u -d '-5 days' +%F)"
  boundary="$(date -u -d '-30 days' +%F)"
  old="$(date -u -d '-31 days' +%F)"
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$recent" >"$versions"
  : >"$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  echo '== Scenario: 新しい minor が EDB にあれば落ちる'
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" 2>&1)" || rc=$?; rc=${rc:-0}
  (( rc == 1 )) && grep -q '17\.6' <<<"$output" && grep -q '17\.11' <<<"$output" || return 1
  rm -f "$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$boundary" >"$versions"
  echo '== Scenario: EDB に無く、公開から 30 日ちょうどなら待つ'
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" 2>&1)" || return 1
  grep -q 'EDB を待っている（公開から 30 日）' <<<"$output" || return 1
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$old" >"$versions"
  echo '== Scenario: EDB に無くても、公開から 30 日を越えたら落ちる'
  rc=0
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" 2>&1)" || rc=$?; rc=${rc:-0}
  (( rc == 1 )) && grep -q '30 日' <<<"$output" || return 1
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$recent" >"$versions"
  echo '== Scenario: EDB に無く、公開から 30 日以内なら待つ'
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" 2>&1)" || return 1
  grep -q 'EDB を待っている' <<<"$output" || return 1
  echo '== Scenario: EDB に届かなければ、30 日の規則だけで判定する'
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE='http://127.0.0.1:9/postgresql-' "$0" 2>&1)" || return 1
  grep -q 'EDB 不明' <<<"$output" || return 1
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$old" >"$versions"
  rc=0
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE='http://127.0.0.1:9/postgresql-' "$0" 2>&1)" && return 1
  rc=$?
  (( rc == 1 )) && grep -q 'EDB 不明' <<<"$output" || return 1
  printf '[{"major":"17","latestMinor":"6","relDate":"%s"}]\n' "$recent" >"$versions"
  echo '== Scenario: lock のビルド番号が -1 でなくても、最新の minor を EDB で探せる'
  printf 'version=17.6-2\nurl=file://%s/postgresql-17.6-2-windows-x64-binaries.zip\n' "$edb" >"$tmp/pgsql-2.lock"
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$recent" >"$versions"
  : >"$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  rc=0
  output="$(PGSQL_LOCK_FILE="$tmp/pgsql-2.lock" PG_VERSIONS_URL="file://$versions" "$0" 2>&1)" || rc=$?
  (( rc == 1 )) && grep -q 'EDB の zip: present' <<<"$output" || return 1
  rm -f "$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  echo '== Scenario: lock の url が version と合わなければ rc=2'
  printf 'version=17.6-2\nurl=file://%s/postgresql-17.6-1-windows-x64-binaries.zip\n' "$edb" >"$tmp/pgsql-bad.lock"
  rc=0
  output="$(PGSQL_LOCK_FILE="$tmp/pgsql-bad.lock" PG_VERSIONS_URL="file://$versions" "$0" 2>&1)" || rc=$?
  (( rc == 2 )) && grep -q '^error: 検査できなかった' <<<"$output" || return 1
  echo '== Scenario: 遅れの材料は、置き場のパスに . があってもディレクトリに書く'
  : >"$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  rc=0
  PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" --findings "$tmp/a.b/findings" >/dev/null 2>&1 || rc=$?
  (( rc == 1 )) && [[ -f "$tmp/a.b/findings/postgresql-minor.txt" ]] || return 1
  rm -f "$edb/postgresql-17.11-1-windows-x64-binaries.zip"
  printf '[{"major":"17","latestMinor":"6","relDate":"%s"}]\n' "$recent" >"$versions"
  echo '== Scenario: 同梱の版が最新なら通る'
  PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$versions" PG_EDB_URL_BASE="file://$edb/postgresql-" "$0" >/dev/null || return 1
  echo '== Scenario: 版の一覧を取れなければ rc=2'
  rc=0
  output="$(PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$tmp/missing.json" "$0" 2>&1)" || rc=$?
  (( rc == 2 )) && grep -q '^error: 検査できなかった' <<<"$output" || return 1
  echo 'OK: PostgreSQL minor 検査の自己テストに成功'
}

case "${1:-}" in
  --self-test) self_test ;;
  --findings) [[ $# -eq 2 ]] || usage; findings="$2"; check ;;
  "") check ;;
  *) usage ;;
esac
