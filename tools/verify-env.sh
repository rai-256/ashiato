#!/usr/bin/env bash
# 検証の準備。ハーネス（scripts/verify-run）が**検証コマンドの前に毎回**呼ぶ。
# **印（HARNESS_VERIFY=1）が無ければ何度呼んでも同じ**（起動済みなら何もしない）。**印があれば毎回空から**（worktree か CI=true のとき。ch06 design D2）。
# 作り直すかの判定は `--plan`（`reset` か `keep: <理由>` の 1 行。docker にも .env にも触らない）。
# いまやることは 2 つ: この worktree のテスト用 DB を起動して接続できるまで待ち、DB の役割（ST28）を整える。
#   - ポートは tools/ports.sh の worktree ごとの規則（Story を並行して走らせても取り合わない）
#   - 起動するのはこの worktree の compose の DB だけ。他の Story のコンテナには触らない
#   - 準備できなければ非 0 で終わる（ハーネスは BLOCKED_INFRA として記録し、AI の実装ループに流さない）
# 実測 2026-09-30 ST05 10.4: ポートを分けた後、`cargo test --workspace` の DB を起動する係がいなかった
# （それまでは別の Story の DB が 55432 に残っていて、そこに繋がって通っていた）。
set -euo pipefail
cd "$(dirname "$0")/.."

# 作り直すかの判定（design D2）。worktree かどうかの見分け方は tools/ports.sh と同じ。git の木でなければ worktree でない木と読む
plan() {
  local dirs gitdir common
  [[ ${HARNESS_VERIFY:-} == 1 ]] || { echo "keep: 印が無い"; return; }
  dirs=$(git rev-parse --path-format=absolute --git-dir --git-common-dir 2>/dev/null || true)
  gitdir=${dirs%%$'\n'*}
  common=${dirs##*$'\n'}
  if [[ -n $dirs && $gitdir != "$common" ]]; then echo reset
  elif [[ ${CI:-} == true ]]; then echo reset
  else echo "keep: main の checkout（開発用の起動の DB）は作り直さない"
  fi
}
case "${1:-}" in
  --plan) plan; exit 0 ;;
  "") ;;
  *) echo "使い方: $0 [--plan]" >&2; exit 2 ;;
esac
decision=$(plan)
[[ $decision != keep:* || ${HARNESS_VERIFY:-} != 1 ]] || echo "$decision" >&2

. tools/ports.sh
tools/env-init.sh     # `.env` が無い worktree（下流で新しく作ったもの）でも DB を起動できるように

# 切った接続の 1 行（design D3）。動いている DB の client backend（この問い合わせ自身を除く）を出所ごとに数える。問えなければ 0
cut_line() {
  local rows n list
  if ! rows=$(docker compose exec -T db psql -X -At -U ashiato -d ashiato -F ' ' -c \
    "select coalesce(nullif(application_name,''),'-')||'@'||coalesce(host(client_addr),'local'), count(*)
       from pg_stat_activity where backend_type='client backend' and pid<>pg_backend_pid()
      group by 1 order by 2 desc, 1" 2>/dev/null); then
    echo "verify-env: DB を作り直す（切った接続: 0（DB は止まっていた））" >&2; return
  fi
  n=$(awk '{s+=$NF} END{print s+0}' <<<"$rows")
  list=$(awk 'NF{ printf "%s%s%s", (c++ ? ", " : ""), $1, ($2>1 ? " ×" $2 : "") }' <<<"$rows" | awk -F', ' '{ for (i=1;i<=NF && i<=10;i++) printf "%s%s", (i>1?", ":""), $i; if (NF>10) printf ", …" }')
  if [[ $n == 0 ]]; then echo "verify-env: DB を作り直す（切った接続: 0）" >&2
  else echo "verify-env: DB を作り直す（切った接続: $n。$list）" >&2; fi
}

# 作り直し（design D1・D4）。volume を名前で控え、他のコンテナが使っていなければ、コンテナと volume を消す。`-v` は使わない
reset_db() {
  local cid vols v users others
  cid=$(docker compose ps -aq db --no-trunc)
  if [[ -z $cid ]]; then echo "verify-env: DB を作り直す（切った接続: 0（DB のコンテナが無い））" >&2; return; fi
  vols=$(docker inspect "$cid" --format '{{range .Mounts}}{{if eq .Type "volume"}}{{.Name}}{{"\n"}}{{end}}{{end}}')
  for v in $vols; do
    users=$(docker ps -aq --no-trunc --filter "volume=$v")
    others=$(grep -vx "$cid" <<<"$users" || true)
    if [[ -n $others ]]; then
      echo "verify-env: volume $v を使う別のコンテナがある。何も消さずに止める: $(docker inspect --format '{{.Name}}' $others | tr '\n' ' ')" >&2
      exit 1
    fi
  done
  cut_line
  docker compose rm -s -f db >/dev/null
  for v in $vols; do docker volume rm "$v" >/dev/null || { echo "verify-env: volume $v を消せない" >&2; exit 1; }; done
}

# 塞いでいる network の確かめ（design D5）。subnet のファイルがあり自分の network が無いとき、同じ subnet の別の network を探す。
# コンテナが繋がっていなければその network だけを消し、繋がっていれば何も消さずに止める
clear_blocking_network() {
  local subnet own n others
  [[ -f docker-compose.subnet.yml ]] || return 0
  subnet=$(awk '/- subnet:/{print $3; exit}' docker-compose.subnet.yml)
  own=$(docker compose config | awk '/^networks:/{n=1} n&&/^ +name:/{print $2; exit}')
  [[ -n $subnet && -n $own ]] || return 0
  ! docker network inspect "$own" >/dev/null 2>&1 || return 0
  for n in $(docker network ls --format '{{.Name}}'); do
    [[ $n != "$own" ]] || continue
    docker network inspect "$n" --format '{{range .IPAM.Config}}{{.Subnet}}{{"\n"}}{{end}}' | grep -qx "$subnet" || continue
    others=$(docker ps -aq --filter "network=$n")
    if [[ -n $others ]]; then
      echo "verify-env: network $n が subnet $subnet を塞いでいる。繋がっているコンテナ: $(docker inspect --format '{{.Name}}' $others | tr '\n' ' ')。何も消さずに止める" >&2
      exit 1
    fi
    docker network rm "$n" >/dev/null
  done
}

[[ $decision != reset ]] || reset_db
clear_blocking_network
docker compose up -d --wait db >/dev/null
# --wait は healthcheck を待つ。テストが繋ぐのと同じ口（127.0.0.1:<ポート>）で受け付けるまで確かめる
for _ in $(seq 1 30); do
  if docker compose exec -T db pg_isready -q -h 127.0.0.1 -U ashiato -d ashiato \
     && (exec 3<>"/dev/tcp/127.0.0.1/${ASHIATO_DB_PORT}") 2>/dev/null; then
    # 役割（ashiato_owner / ashiato_app）を整える。テストは所有者で繋ぐ（ST28 / design D4）。何度呼んでも同じ。
    # **毎回やる** —— tools/smoke.sh は後片付けで DB を volume ごと消すので、次の検証の DB には役割が無い
    # （実測 2026-10-07 ST21 11.3: `password authentication failed for user "ashiato_owner"` で 371 本が落ちた）
    tools/db-roles.sh >/dev/null || { echo "DB の役割を整えられない（tools/db-roles.sh。.env の合言葉を見る）" >&2; exit 1; }
    exit 0
  fi
  sleep 1
done
echo "テスト用 DB（127.0.0.1:${ASHIATO_DB_PORT}）に接続できない" >&2
exit 1
