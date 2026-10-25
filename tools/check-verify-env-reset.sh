#!/usr/bin/env bash
# 検証のたびにテスト用 DB を作り直す仕組み（ch06）の検査。
#
#   ./tools/check-verify-env-reset.sh --self-test  一時の repo と worktree で tools/env-init.sh の規則を確かめる（docker の daemon に触らない）。
#                                                  CI の rust job と手元で同じ結果になる
#   ./tools/check-verify-env-reset.sh              この木の DB を作り直して確かめる本体（worktree か CI=true の木だけ。ほかは rc=2）
#
# 一時の木の道具は、環境変数が結果に回り込まないよう 9 つの変数を落として呼ぶ（design D8）。本体の各段は HARNESS_VERIFY を明示して呼ぶ。
set -Eeuo pipefail
cd "$(dirname "$0")/.."

# ---- 本体 -------------------------------------------------------------------------------------
# この木の DB を作り直して確かめる（design D8 の 1.〜8.）。各段で HARNESS_VERIFY を明示する（呼び出し側の印に依らない）。
# 終わった DB は空で、役割が整っている
body() {
  local dirs gitdir common is_wt=0
  dirs=$(git rev-parse --path-format=absolute --git-dir --git-common-dir 2>/dev/null || true)
  gitdir=${dirs%%$'\n'*}; common=${dirs##*$'\n'}
  [[ -n $dirs && $gitdir != "$common" ]] && is_wt=1
  if [[ $is_wt == 0 && ${CI:-} != true ]]; then
    echo "check-verify-env-reset: worktree でも CI=true でもない木（main の checkout）では DB を作り直さない。走らせない" >&2
    return 2
  fi

  . tools/ports.sh
  tools/env-init.sh

  # trap は body が戻った後に走るので、cleanup が読む変数は local にしない
  local r c fail=0 stage=
  tmp=$(mktemp -d)
  r=$(openssl rand -hex 4)
  sp="ch06-sentinel-$r"; holder_pid=
  # sentinel（コンテナ・volume・network・compose project）は必ず消す
  cleanup() {
    [[ -z $holder_pid ]] || kill "$holder_pid" 2>/dev/null || true
    # kill が止めるのは exec の CLI だけ。DB が生きていれば pg_sleep の backend も切る
    docker compose exec -T db psql -X -qtA -U ashiato -d ashiato \
      -c "select pg_terminate_backend(pid) from pg_stat_activity where application_name='ch06-holder'" >/dev/null 2>&1 || true
    local ids
    ids=$(docker ps -aq --filter "name=$sp"); [[ -z $ids ]] || docker rm -f $ids >/dev/null 2>&1 || true
    ids=$(docker volume ls -q --filter "name=$sp"); [[ -z $ids ]] || docker volume rm $ids >/dev/null 2>&1 || true
    ids=$(docker network ls -q --filter "name=$sp"); [[ -z $ids ]] || docker network rm $ids >/dev/null 2>&1 || true
    rm -rf "$tmp"
  }
  trap cleanup EXIT
  # 守っていない docker / psql の失敗で set -e が抜けるときも、どの段かを出す
  trap 'echo "  NG [$stage]: $LINENO 行目のコマンドが失敗した" >&2' ERR

  ng() { echo "  NG [$stage]: $1" >&2; fail=1; }
  good() { echo "  ok [$stage]: $1"; }
  want() { if [[ $2 == "$3" ]]; then good "$1"; else ng "$1（$2。期待は $3）"; fi; }
  ok() { if [[ $2 == 0 ]]; then good "$1"; else ng "$1"; fi; }
  has() {  # has <説明> <文字列> <部分>
    if [[ $2 == *"$3"* ]]; then good "$1"; else ng "$1（$3 が無い）"; fi
  }
  cut0() {  # cut0 <説明> —— 出力に「切った接続: 0」の行がある（10 などには当たらない）
    local c=0; both | grep -qE '切った接続: 0([^0-9]|$)' || c=1; ok "$1" $c
  }

  q() { docker compose exec -T db psql -X -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato "$@"; }
  rows() { q -c "select count(*) from core.event" 2>/dev/null; }
  rows_after_reset() {  # 表が無ければ 0
    q -c "select coalesce(to_regclass('core.event')::text, '')" 2>/dev/null
  }
  db_cid() { docker compose ps -aq db --no-trunc; }
  db_vols() { docker inspect "$(db_cid)" --format '{{range .Mounts}}{{if eq .Type "volume"}}{{.Name}}{{"\n"}}{{end}}{{end}}'; }
  vol_exists() { docker volume ls -q | grep -qx "$1"; }
  own_net=$(docker compose config | awk '/^networks:/{n=1} n&&/^ +name:/{print $2; exit}')
  net_id() { docker network inspect "$1" --format '{{.Id}}' 2>/dev/null || true; }
  cid_of() { docker inspect "$1" --format '{{.Id}}' 2>/dev/null || true; }
  # run <印の渡し方> —— out / err / rc に残す
  rc=0
  call() { rc=0; "$@" >"$tmp/out" 2>"$tmp/err" || rc=$?; }
  marked() { call env HARNESS_VERIFY=1 tools/verify-env.sh; }
  unmarked() { call env -u HARNESS_VERIFY tools/verify-env.sh; }
  both() { cat "$tmp/out" "$tmp/err"; }
  owner_pw=$( (set -a; [[ -f .env ]] && . ./.env; printf %s "${OWNER_DB_PASSWORD:-}") )
  app_pw=$( (set -a; [[ -f .env ]] && . ./.env; printf %s "${APP_DB_PASSWORD:-}") )
  # コンテナの中の 127.0.0.1 は pg_hba で trust に当たり合言葉を見ないので、コンテナ自身の非 loopback のアドレスで繋ぐ
  as_role() {  # as_role <役割> <合言葉> <SQL>
    docker compose exec -T -e PGPASSWORD="$2" db sh -c \
      'psql -h "$(hostname -i | cut -d" " -f1)" -X -qtA -U "$0" -d ashiato -c "$1"' "$1" "$3"
  }

  stage="1. 空にする"
  marked; want "rc=0" "$rc" 0
  image=$(docker inspect "$(db_cid)" --format '{{.Config.Image}}')
  c=0; [[ -n $own_net ]] || c=1; ok "DB の network の名前が compose config から出る" $c

  stage="2. 溜まった状態を作る"
  keep_db="ch06_keep_$r"
  q -c "create schema core; create table core.event(id int); insert into core.event select generate_series(1,3)" >/dev/null
  q -c "create database $keep_db" >/dev/null
  want "3 行" "$(rows)" 3

  stage="3. 印なし"
  echo "Scenario: 印が無ければ作り直さず、溜まった行は残る"
  unmarked
  want "rc=0" "$rc" 0
  want "行は 3 件のまま" "$(rows)" 3
  if [[ $is_wt == 0 ]]; then
    echo "Scenario: CI でない main の checkout では、印つきで呼んでも行が残る"
    call env -u CI HARNESS_VERIFY=1 tools/verify-env.sh
    want "rc=0" "$rc" 0
    want "行は 3 件のまま" "$(rows)" 3
    c=0; grep -q '^keep:' "$tmp/err" || c=1; ok "標準エラーに keep: の行がある" $c
  else
    echo "SKIP 3.（CI でない main の checkout）: この木は worktree で、印があれば必ず作り直すので確かめられない（CI が通す）"
  fi

  stage="4. 別のコンテナが volume を使っている"
  echo "Scenario: DB の volume をほかのコンテナが使っていれば、何も消さずに非 0 で終わる"
  vol4=$(db_vols | head -n1)
  cid4=$(db_cid)
  docker create --name "$sp-vol" -v "$vol4:/sentinel" "$image" true >/dev/null
  marked
  c=0; [[ $rc != 0 ]] || c=1; ok "rc≠0（rc=$rc）" $c
  has "volume の名前が出る" "$(both)" "$vol4"
  has "sentinel のコンテナの名前が出る" "$(both)" "$sp-vol"
  want "DB のコンテナは同じ id" "$(db_cid)" "$cid4"
  want "行は 3 件のまま" "$(rows)" 3
  docker rm -f "$sp-vol" >/dev/null

  stage="5. 繋いでいるものがある"
  vols5=$(db_vols)
  net5=$(net_id "$own_net")
  # ほかの compose project（service db・network_mode none）と、compose の外のコンテナ・volume
  cat >"$tmp/sentinel-compose.yml" <<YML
services:
  db:
    image: $image
    network_mode: none
    command: sleep 600
    stop_grace_period: 1s
YML
  docker compose -p "$sp" -f "$tmp/sentinel-compose.yml" up -d >/dev/null 2>&1
  docker volume create "$sp-ext" >/dev/null
  docker create --name "$sp-ext" -v "$sp-ext:/sentinel" "$image" true >/dev/null
  proj_cid=$(docker ps -aq --no-trunc --filter "label=com.docker.compose.project=$sp")
  ext_cid=$(cid_of "$sp-ext")
  ext_vol_created=$(docker volume inspect "$sp-ext" --format '{{.CreatedAt}}')
  c=0; [[ -n $proj_cid ]] || c=1; ok "別の compose project のコンテナが立った" $c
  docker compose exec -T -e PGAPPNAME=ch06-holder db psql -X -q -U ashiato -d ashiato -c "select pg_sleep(600)" >/dev/null 2>&1 &
  holder_pid=$!
  for _ in $(seq 1 30); do
    [[ $(q -c "select count(*) from pg_stat_activity where application_name='ch06-holder'") == 1 ]] && break
    sleep 1
  done
  want "ch06-holder が pg_stat_activity に出た" "$(q -c "select count(*) from pg_stat_activity where application_name='ch06-holder'")" 1
  echo "Scenario: 繋いでいるものがあれば、切った数と出所を 1 行出す"
  marked
  want "rc=0" "$rc" 0
  line=$(both | grep -E '切った接続: 1([^0-9]|$)' || true)
  has "切った接続: 1 の行に ch06-holder がある" "$line" "ch06-holder"
  hrc=0; wait "$holder_pid" || hrc=$?; holder_pid=
  c=0; [[ $hrc != 0 ]] || c=1; ok "ch06-holder の psql は非 0（rc=$hrc）" $c
  echo "Scenario: 溜まった DB は、印つきの準備の後に空になる"
  want "core.event は無い（か 0 行）" "$(rows_after_reset)" ""
  want "ch06_keep_ の database は無い" "$(q -c "select count(*) from pg_database where datname like 'ch06\_keep\_%'")" 0
  echo "Scenario: 作り直しでは、DB の network を手放さない"
  want "DB の network の id は同じ" "$(net_id "$own_net")" "$net5"
  vols5_now=$(db_vols)
  for v in $vols5; do
    if vol_exists "$v"; then ng "前の volume $v が残っている"; else good "前の volume $v は無い"; fi
  done
  want "volume はちょうど 1 つ" "$(printf '%s\n' "$vols5_now" | grep -c .)" 1
  echo "Scenario: ほかの compose project と compose の外のものには触らない"
  want "別の compose project のコンテナは同じ id" "$(docker ps -aq --no-trunc --filter "label=com.docker.compose.project=$sp")" "$proj_cid"
  want "compose の外のコンテナは同じ id" "$(cid_of "$sp-ext")" "$ext_cid"
  want "compose の外の volume は同じもの" "$(docker volume inspect "$sp-ext" --format '{{.CreatedAt}}' 2>/dev/null || true)" "$ext_vol_created"
  echo "Scenario: 作り直した後も、所有者とアプリの役割で繋がる"
  want "ashiato_owner で繋がり、所有者は ashiato_owner"     "$(as_role ashiato_owner "$owner_pw" "select pg_get_userbyid(datdba) from pg_database where datname='ashiato'" 2>/dev/null || true)" ashiato_owner
  want "ashiato_app で繋がる"     "$(as_role ashiato_app "$app_pw" "select current_user" 2>/dev/null || true)" ashiato_app
  want "違う合言葉では ashiato_owner に繋がらない" "$(as_role ashiato_owner "${owner_pw}x" "select 1" 2>/dev/null || true)" ""
  want "違う合言葉では ashiato_app に繋がらない"   "$(as_role ashiato_app "${app_pw}x" "select 1" 2>/dev/null || true)" ""

  stage="6. もう一度（2 回続けて作り直せる）"
  echo "Scenario: 繋いでいるものが無ければ、切った接続は 0 と出す"
  marked
  want "rc=0" "$rc" 0
  cut0 "切った接続: 0 の行がある"
  echo "Scenario: 作り直しの後、前の DB の volume は残らない"
  for v in $vols5 $vols5_now; do
    if vol_exists "$v"; then ng "前の volume $v が残っている"; else good "前の volume $v は無い"; fi
  done
  want "volume はちょうど 1 つ" "$(db_vols | grep -c .)" 1

  stage="7. DB のコンテナが無い"
  echo "Scenario: DB のコンテナが無くても、切った接続は 0 と出して DB を立てる"
  vols7=$(db_vols)
  docker compose rm -s -f db >/dev/null
  for v in $vols7; do docker volume rm "$v" >/dev/null; done
  marked
  want "rc=0" "$rc" 0
  cut0 "切った接続: 0 の行がある"
  want "DB のコンテナが立っている" "$(docker compose ps --status running -q db | grep -c .)" 1

  if [[ -f docker-compose.subnet.yml ]]; then
    stage="8. 塞いでいる network"
    subnet=$(awk '/- subnet:/{print $3; exit}' docker-compose.subnet.yml)
    drop_db() {  # DB と volume と network を手放す。down は匿名 volume が宙に浮くので使わない（P11）
      local vs v
      vs=$(db_vols 2>/dev/null || true)
      docker compose rm -s -f db >/dev/null
      for v in $vs; do docker volume rm "$v" >/dev/null; done
      docker network rm "$own_net" >/dev/null
    }
    drop_db
    docker network create --subnet "$subnet" "$sp-net" >/dev/null
    echo "Scenario: 同じ subnet を持つ、コンテナの繋がっていない network は消して立てる"
    marked
    want "rc=0" "$rc" 0
    if docker network inspect "$sp-net" >/dev/null 2>&1; then ng "sentinel の network が残っている"; else good "sentinel の network は無い"; fi
    want "DB の network がその subnet で立っている"       "$(docker network inspect "$own_net" --format '{{range .IPAM.Config}}{{.Subnet}}{{end}}' 2>/dev/null || true)" "$subnet"

    drop_db
    docker network create --subnet "$subnet" "$sp-net" >/dev/null
    docker create --name "$sp-ctr" --network "$sp-net" "$image" true >/dev/null
    net8=$(net_id "$sp-net"); ctr8=$(cid_of "$sp-ctr")
    echo "Scenario: 同じ subnet の network にコンテナが繋がっていれば、何も消さずに非 0 で終わる"
    marked
    c=0; [[ $rc != 0 ]] || c=1; ok "rc≠0（rc=$rc）" $c
    has "network の名前が出る" "$(both)" "$sp-net"
    has "コンテナの名前が出る" "$(both)" "$sp-ctr"
    want "network は同じ id" "$(net_id "$sp-net")" "$net8"
    want "コンテナは同じ id" "$(cid_of "$sp-ctr")" "$ctr8"
    docker rm -f "$sp-ctr" >/dev/null
    marked   # DB を立て直す
    want "sentinel を消した後の準備は rc=0" "$rc" 0
  else
    echo "SKIP 8.（塞いでいる network）: subnet のファイル（docker-compose.subnet.yml）が無い木（CI）では確かめない"
  fi

  stage="終わりの DB"
  want "DB は空（core.event が無い）" "$(rows_after_reset)" ""
  want "役割が整っている（ashiato_owner で繋がる）"     "$(as_role ashiato_owner "$owner_pw" "select current_user" 2>/dev/null || true)" ashiato_owner

  if [[ $fail == 0 ]]; then echo OK; else echo "check-verify-env-reset FAILED" >&2; trap - ERR; return 1; fi   # 呼び出し行（body; exit $?）で ERR trap を鳴らさない
}

case "${1:-}" in
  --self-test) ;;
  "") body; exit $? ;;
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
mkdir -p "$tmp/demo/tools"; cp tools/ports.sh tools/verify-env.sh "$tmp/demo/tools/"
mk demo-ch01; mk demo-ch02; mk demo-st05; mk demo-verify

# 一時の木の道具を、結果に回り込む 9 つの変数を落として呼ぶ
clean() { env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u APP_DB_PASSWORD -u ASHIATO_DB_SUBNET -u ASHIATO_DB_PORT \
  -u ASHIATO_HTTP_PORT -u COMPOSE_FILE -u CI -u HARNESS_VERIFY "$@"; }
init() {  # init の前に w を決める。落ちたら場合（呼んだ行の直前の Scenario の印）を名指しし、stderr を添えて NG で終える
  clean "$@" "$tmp/$w/tools/env-init.sh" 2>"$tmp/init-err" && return
  local s
  s=$(awk -v n="${BASH_LINENO[0]}" 'NR > n { exit } sub(/^echo "Scenario[:] /, "") { sub(/"$/, ""); s = $0 } END { print s }' tools/check-verify-env-reset.sh)
  echo "  self-test NG: [$s] $w の tools/env-init.sh が失敗した: $(cat "$tmp/init-err")" >&2
  echo "self-test FAILED" >&2; exit 1
}
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
ino=$(stat -c %i "$tmp/$w/.env")
init
want "COMPOSE_FILE の行が 1 つ" "$(grep -c '^COMPOSE_FILE=' "$tmp/$w/.env")" "1"
want "ほかの行は呼ぶ前と同じ" "$(grep -v '^COMPOSE_FILE=' "$tmp/$w/.env")" "$before"

echo "Scenario: env-init が書き換えた .env の権限は、書き換える前と同じである"
want ".env の権限は 600 のまま" "$(perm $w)" "600"
want ".env の inode は呼ぶ前と同じ（置き換えていない）" "$(stat -c %i "$tmp/$w/.env")" "$ino"

echo "Scenario: 別の木から写した .env でも、その木の subnet になり、何度呼んでも同じである"
w=demo-ch02
cp -p "$tmp/demo-ch01/.env" "$tmp/$w/.env"
c=0; [[ $(subnet demo-ch01) != "$(subnet $w)" ]] || c=1; ok "2 つの木の subnet は違う" $c
# clean の env -u の後ろへ渡す（古い値が環境にあっても、subnet は port から決まる）
init ASHIATO_DB_SUBNET="$(subnet demo-ch01)"
cp "$tmp/$w/.env" "$tmp/env1"; cp "$tmp/$w/docker-compose.subnet.yml" "$tmp/sub1"
init ASHIATO_DB_SUBNET="$(subnet demo-ch01)"
want "subnet のファイルの subnet はその木の値" "$(sed -n 's/^ *- subnet: //p' "$tmp/$w/docker-compose.subnet.yml")" "$(subnet $w)"
c=0; cmp -s "$tmp/$w/.env" "$tmp/env1" || c=1; ok "2 回目の後の .env は 1 回目の後と同じ" $c
c=0; cmp -s "$tmp/$w/docker-compose.subnet.yml" "$tmp/sub1" || c=1; ok "2 回目の後の subnet のファイルは 1 回目の後と同じ" $c

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
c=0; cmp -s "$tmp/$w/.env" "$tmp/rec-before" || c=1; ok ".env は呼ぶ前と同じ" $c

echo "Scenario: env-init の後は、compose の DB の network に木の subnet が入る"
w=demo-ch01
got=$(cd "$tmp/$w" && env -i PATH="$PATH" HOME="$HOME" docker compose config --format json \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["networks"]["default"]["ipam"]["config"][0]["subnet"])')
want "default の network の subnet は木の値" "$got" "$(subnet $w)"

plan() { clean "$@" "$tmp/$w/tools/verify-env.sh" --plan; }   # plan の前に w を決める

echo "Scenario: CI でない main の checkout では、印があっても作り直さない"
w=demo
got=$(plan HARNESS_VERIFY=1)
want "worktree でない木は keep:" "${got%%:*}" "keep"
c=0; [[ $got == *main* ]] || c=1; ok "理由に main を挙げる" $c

echo "Scenario: CI の main の checkout と、worktree では、印があれば作り直す"
w=demo; want "worktree でない木の CI=true" "$(plan CI=true HARNESS_VERIFY=1)" "reset"
for w in demo-st05 demo-verify demo-ch01; do want "$w の worktree" "$(plan HARNESS_VERIFY=1)" "reset"; done

echo "Scenario: 印が無ければ、どの木でも作り直さない判定になる"
w=demo; want "worktree でない木（印なし）" "$(plan | cut -d: -f1)" "keep"
want "worktree でない木（CI=true・印なし）" "$(plan CI=true | cut -d: -f1)" "keep"
for w in demo-st05 demo-verify demo-ch01; do want "$w の worktree（印なし）" "$(plan | cut -d: -f1)" "keep"; done

[[ $fail == 0 ]] && echo OK || { echo "self-test FAILED" >&2; exit 1; }
