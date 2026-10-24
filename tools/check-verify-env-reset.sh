#!/usr/bin/env bash
# 検証のたびにテスト用 DB を作り直す仕組み（ch06）の検査。
#
#   ./tools/check-verify-env-reset.sh --self-test  一時の repo と worktree で tools/env-init.sh の規則を確かめる（docker の daemon に触らない）。
#                                                  CI の rust job と手元で同じ結果になる
#   ./tools/check-verify-env-reset.sh              この木の DB を作り直して確かめる本体（worktree か CI=true の木だけ。ほかは rc=2）
#
# 一時の木の道具は、環境変数が結果に回り込まないよう 9 つの変数を落として呼ぶ（design D8）。
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1:-}" in
  --self-test) ;;
  "") echo "check-verify-env-reset: 本体はまだ無い（--self-test だけ）" >&2; exit 2 ;;
  *) echo "使い方: $0 [--self-test]" >&2; exit 2 ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
want() {  # want <説明> <実際> <期待>
  if [[ $2 == "$3" ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1（$2。期待は $3）" >&2; fail=1; fi
}
ok() {  # ok <説明> <rc>（0 なら通る）
  if [[ $2 == 0 ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1" >&2; fail=1; fi
}

git -C "$tmp" init -q demo
git -C "$tmp/demo" -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
mk() {
  git -C "$tmp/demo" worktree add -q --detach "$tmp/$1"
  mkdir -p "$tmp/$1/tools"
  cp tools/verify-env.sh tools/ports.sh tools/env-init.sh "$tmp/$1/tools/"
  cp docker-compose.yml "$tmp/$1/"
}
mkdir -p "$tmp/demo/tools"; cp tools/ports.sh "$tmp/demo/tools/"
mk demo-ch01; mk demo-ch02

# 一時の木の道具を、結果に回り込む 9 つの変数を落として呼ぶ
clean() { env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u APP_DB_PASSWORD -u ASHIATO_DB_SUBNET -u ASHIATO_DB_PORT \
  -u ASHIATO_HTTP_PORT -u COMPOSE_FILE -u CI -u HARNESS_VERIFY "$@"; }
init() { clean "$@" "$tmp/$w/tools/env-init.sh" 2>/dev/null; }   # init の前に w を決める
subnet() { clean bash -c '. "$0/tools/ports.sh" && ashiato_db_subnet' "$tmp/$1"; }
perm() { stat -c %a "$tmp/$1/.env"; }
pw='POSTGRES_PASSWORD=pg
OWNER_DB_PASSWORD=ow
APP_DB_PASSWORD=ap
BIND=127.0.0.1:1'

w=demo-ch01
echo "Scenario: 既にある .env に、合言葉を変えずに COMPOSE_FILE を足す"
printf '%s\n' "$pw" >"$tmp/$w/.env"; chmod 600 "$tmp/$w/.env"
before=$(cat "$tmp/$w/.env")
init
want "COMPOSE_FILE の行が 1 つ" "$(grep -c '^COMPOSE_FILE=' "$tmp/$w/.env")" "1"
want "ほかの行は呼ぶ前と同じ" "$(grep -v '^COMPOSE_FILE=' "$tmp/$w/.env")" "$before"

echo "Scenario: env-init が書き換えた .env の権限は、書き換える前と同じである"
want ".env の権限は 600 のまま" "$(perm $w)" "600"

echo "Scenario: 別の木から写した .env でも、その木の subnet になり、何度呼んでも同じである"
w=demo-ch02
cp -p "$tmp/demo-ch01/.env" "$tmp/$w/.env"
ASHIATO_DB_SUBNET="$(subnet demo-ch01)" init env
cp "$tmp/$w/.env" "$tmp/env1"; cp "$tmp/$w/docker-compose.subnet.yml" "$tmp/sub1"
ASHIATO_DB_SUBNET="$(subnet demo-ch01)" init env
want "subnet のファイルの subnet はその木の値" "$(sed -n 's/^ *- subnet: //p' "$tmp/$w/docker-compose.subnet.yml")" "$(subnet $w)"
cmp -s "$tmp/$w/.env" "$tmp/env1"; ok "2 回目の後の .env は 1 回目の後と同じ" $?
cmp -s "$tmp/$w/docker-compose.subnet.yml" "$tmp/sub1"; ok "2 回目の後の subnet のファイルは 1 回目の後と同じ" $?

echo "Scenario: 木に override のファイルがあれば、COMPOSE_FILE に並べる"
w=demo-ch01
: >"$tmp/$w/docker-compose.override.yml"
init
want "COMPOSE_FILE は 3 つを並べる" "$(sed -n 's/^COMPOSE_FILE=//p' "$tmp/$w/.env")" \
  "docker-compose.yml:docker-compose.subnet.yml:docker-compose.override.yml"
rm "$tmp/$w/docker-compose.override.yml"; init   # 以降の木は override 無しの形に戻す

echo "Scenario: 合言葉が環境変数で渡っていれば、.env を作らない"
git -C "$tmp/demo" worktree add -q --detach "$tmp/demo-ch03"; mkdir -p "$tmp/demo-ch03/tools"
cp tools/ports.sh tools/env-init.sh "$tmp/demo-ch03/tools/"
if clean POSTGRES_PASSWORD=a OWNER_DB_PASSWORD=b APP_DB_PASSWORD=c "$tmp/demo-ch03/tools/env-init.sh" 2>/dev/null; then r=0; else r=1; fi
ok "rc=0" "$r"
want ".env も docker-compose.subnet.yml も無い" "$(ls -A "$tmp/demo-ch03" | grep -c -e '^\.env$' -e '^docker-compose.subnet.yml$' || true)" "0"

echo "Scenario: compose project の名前を持つ .env には触らない"
w=demo-ch02
printf '%s\nCOMPOSE_PROJECT_NAME=rec\n' "$pw" >"$tmp/$w/.env"
cp "$tmp/$w/.env" "$tmp/rec-before"
init
cmp -s "$tmp/$w/.env" "$tmp/rec-before"; ok ".env は呼ぶ前と同じ" $?

echo "Scenario: env-init の後は、compose の DB の network に木の subnet が入る"
w=demo-ch01
got=$(cd "$tmp/$w" && env -i PATH="$PATH" HOME="$HOME" docker compose config --format json \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["networks"]["default"]["ipam"]["config"][0]["subnet"])')
want "default の network の subnet は木の値" "$got" "$(subnet $w)"

[[ $fail == 0 ]] && echo OK || { echo "self-test FAILED" >&2; exit 1; }
