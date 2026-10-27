#!/usr/bin/env bash
# `ashiato-server migrate` の退避（PG_DUMP）に使う、db のコンテナの中の pg_dump（db と同じ版。手元の pg_dump は db より古いと落ちる）。tools/stack.sh・tools/dev.sh の既定
# サーバが渡す引数のうち --host / --port は外し（コンテナの中から繋ぐ）、--file は手元のファイルへの書き出しに置き換える。
# 合言葉は PGPASSWORD を値なしで渡す（docker が手元の環境から引く。プロセス一覧に出さない）。
set -euo pipefail
cd "$(dirname "$0")/.."
args=() file=""
while [ $# -gt 0 ]; do
  case "$1" in
    --host|--port) shift 2 ;;
    --file) file="$2"; shift 2 ;;
    *) args+=("$1"); shift ;;
  esac
done
[ -n "$file" ] || { echo "pg_dump-docker: --file が無い" >&2; exit 2; }
docker compose exec -T -e PGPASSWORD db pg_dump --host 127.0.0.1 --port 5432 "${args[@]}" > "$file"
