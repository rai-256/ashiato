#!/usr/bin/env bash
# 録画の専用環境（ハーネスの `scripts/record-run` の差し込み口。宣言は tools/recording/recording.json）。
#
#   tools/record-env.sh up   <worktree> <state>   <worktree> のコードをビルドし、専用の DB で縦串を立てて**前景で待つ**
#   tools/record-env.sh down <worktree> <state>   up が作ったもの（DB のコンテナ・volume・網）を片付け、残りがあれば rc=1
#
# up は用意ができたら <state>/ready に次を書く（record-run が読む）:
#   BASE_URL=…                 画面
#   PASS_<名前>=…              Playwright へ渡す値（合言葉。record-run はコマンド行に出さずに渡す）
#   INFO_<名前>=…              記録に残す値（ビルドの sha256・compose project・port）
#
# 専用にするもの: compose project（<state> の名前から。ashiato2rec<数字>）・port（空いている番号。Windows 側で
# 待ち受けのある番号 REC_AVOID_PORTS も避ける）・合言葉（毎回乱数）・偽データ（STACK_RESET=1 SEED=normal）。
# cargo の出力は REC_CACHE/target（リポジトリの target/ は使わない）。**既存の DB・compose project には触らない。**
set -uo pipefail

cmd="${1:-}"; WT="${2:-}"; STATE="${3:-}"
[ -n "$cmd" ] && [ -d "$WT" ] && [ -d "$STATE" ] || { sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2; }
PROJECT="ashiato2rec$(basename "$STATE" | sed 's/^state-//' | tr -dc '0-9a-f')"
[ "$PROJECT" != ashiato2rec ] || { echo "error: <state> の名前から compose project を作れない: $STATE" >&2; exit 2; }

free_port() { # $1 から上へ、WSL 側でも REC_AVOID_PORTS（Windows 側）でも待ち受けの無い番号
  local p=$1 avoid=" ${REC_AVOID_PORTS:-} "
  while [ -n "$(ss -ltnH "sport = :$p" 2>/dev/null)" ] || [[ "$avoid" == *" $p "* ]]; do p=$((p + 1)); done
  echo "$p"
}

up() {
  cd "$WT" || exit 1
  local db api web pg ow ap old_umask
  db="$(free_port 55440)"; api="$(free_port 18810)"; web="$(free_port 5210)"
  echo "$PROJECT" >"$STATE/project"; echo "$db $api $web" >"$STATE/ports"
  r_() { openssl rand -hex 24; }
  pg="$(r_)"; ow="$(r_)"; ap="$(r_)"
  old_umask="$(umask)"; umask 077
  cat >.env <<EOF || exit 1
COMPOSE_PROJECT_NAME=$PROJECT
POSTGRES_PASSWORD=$pg
OWNER_DB_PASSWORD=$ow
APP_DB_PASSWORD=$ap
DATABASE_URL=postgres://ashiato_app:$ap@127.0.0.1:$db/ashiato
DATABASE_OWNER_URL=postgres://ashiato_owner:$ow@127.0.0.1:$db/ashiato
BIND=127.0.0.1:$api
API_TOKEN=$(r_)
WEB_PASSWORD=$(r_)
ASHIATO_USER_ID=$(cat /proc/sys/kernel/random/uuid)
ALLOWED_HOSTS=.example.ts.net
EOF
  umask "$old_umask"
  cat >docker-compose.override.yml <<EOF || exit 1
services:
  db:
    ports: !override
      - "127.0.0.1:$db:5432"
EOF
  export COMPOSE_PROJECT_NAME="$PROJECT"

  echo "== ビルド（サーバ release・画面）"
  export CARGO_TARGET_DIR="${REC_CACHE:-$HOME/.cache/harness2-rec}/ashiato2-target"
  cargo build -q --release -p ashiato-server --bin ashiato-server || exit 1
  mkdir -p .rec-bin && cp "$CARGO_TARGET_DIR/release/ashiato-server" .rec-bin/ashiato-server || exit 1
  (cd web && npm ci --no-audit --no-fund && npm run build) || exit 1
  [ -f web/dist/index.html ] || { echo "error: 画面のビルドに index.html が無い" >&2; exit 1; }

  echo "== 起動: DB $db / API $api / 画面 $web（compose project $PROJECT）"
  SERVER_BIN="$WT/.rec-bin/ashiato-server" WEB_DIST="$WT/web/dist" WEB_PORT="$web" STACK_RESET=1 SEED=normal \
    ./tools/stack.sh up &
  local stack=$! i
  for i in $(seq 1 300); do
    # stack.sh の「画面:」の行は、画面の待ち受けと網の外への口の検査の後に出る
    [ -n "$(ss -ltnH "sport = :$web" 2>/dev/null)" ] && curl -sf "http://127.0.0.1:$api/healthz" >/dev/null && break
    kill -0 "$stack" 2>/dev/null || { echo "error: tools/stack.sh up が止まった" >&2; exit 1; }
    sleep 1
  done
  curl -sf -o /dev/null "http://127.0.0.1:$web/" || { echo "error: 画面が ${i} 秒で立たない" >&2; exit 1; }
  sleep 2   # stack.sh の網の外への口の検査（check-exposure）を待つ。落ちていれば stack.sh が止まる
  kill -0 "$stack" 2>/dev/null || { echo "error: tools/stack.sh up が止まった（網の外への口の検査？）" >&2; exit 1; }

  local tmp="$STATE/ready.tmp"
  {
    echo "BASE_URL=http://127.0.0.1:$web"
    echo "PASS_WEB_PASSWORD=$(sed -n 's/^WEB_PASSWORD=//p' .env)"
    echo "INFO_server_sha256=$(sha256sum .rec-bin/ashiato-server | cut -d' ' -f1)"
    echo "INFO_web_dist_sha256=$( (cd web/dist && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1)"
    echo "INFO_compose_project=$PROJECT"
    echo "INFO_ports=DB $db / API $api / 画面 $web"
    echo "INFO_seed=normal（STACK_RESET=1 で作り直した直後）"
  } >"$tmp" && mv "$tmp" "$STATE/ready"
  echo "== 用意ができた: http://127.0.0.1:$web"
  wait "$stack"
}

down() {
  local p; p="$(cat "$STATE/project" 2>/dev/null || echo "$PROJECT")"
  [ "$p" = "$PROJECT" ] || { echo "error: <state> の compose project が名前と食い違う（$p）" >&2; exit 2; }
  # **この実行の compose project だけ**を volume ごと消す（他の project には触らない）
  if [ -f "$WT/docker-compose.yml" ]; then
    (cd "$WT" && docker compose -p "$PROJECT" down -v --remove-orphans) 2>&1 | tail -3
  fi
  local left=""
  docker ps -aq --filter "label=com.docker.compose.project=$PROJECT" | grep -q . && left="$left container"
  docker volume ls -q --filter "label=com.docker.compose.project=$PROJECT" | grep -q . && left="$left volume"
  docker network ls -q --filter "label=com.docker.compose.project=$PROJECT" | grep -q . && left="$left network"
  local port
  local ports=""; ports="$(cat "$STATE/ports" 2>/dev/null)"
  for port in $ports; do
    [ -n "$(ss -ltnH "sport = :$port" 2>/dev/null)" ] && left="$left port:$port"
  done
  if [ -n "$left" ]; then echo "残り:$left（compose project $PROJECT）"; exit 1; fi
  echo "残りなし（compose project $PROJECT）"
}

case "$cmd" in
  up) up ;;
  down) down ;;
  *) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2 ;;
esac
