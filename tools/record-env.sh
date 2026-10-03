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
#   ANDROID_APK= / ANDROID_TEST_APK=   REC_PLATFORMS に android があるとき。録画用のサーバ向けに作った APK
#   ANDROID_REVERSE=18787:<API の port>   端末の中の 127.0.0.1:18787（APK の接続先）を録画用のサーバへ
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
  if grep -q '^APP_DB_PASSWORD=' .env.example 2>/dev/null; then
    # ST28 以降の木: DB の役割が 3 つ（管理者・所有者・アプリ）と画面の合言葉
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
  else
    # ST28 より前の木（ST28 を含まない Story のブランチ）: DB の合言葉は docker-compose.yml の固定値で、
    # 画面のログインも無い（実測 2026-10-03: ST05 の木に上の形を渡すと DB の認証で落ちた）。DB は loopback の専用 project
    cat >.env <<EOF || exit 1
COMPOSE_PROJECT_NAME=$PROJECT
DATABASE_URL=postgres://ashiato:ashiato@127.0.0.1:$db/ashiato
BIND=127.0.0.1:$api
API_TOKEN=$(r_)
ASHIATO_USER_ID=$(cat /proc/sys/kernel/random/uuid)
ALLOWED_HOSTS=.example.ts.net
EOF
  fi
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
  local android_ok=""
  if [[ " ${REC_PLATFORMS:-} " == *" android "* ]]; then
    # 録画用のサーバへ送る APK（接続先は端末の中の 127.0.0.1:18787。record-run が adb reverse で API の port へ渡す。
    # 平文 HTTP を許すのは loopback だけ —— app/build.gradle.kts の検査）。トークンと利用者はこの実行の .env の値
    echo "== Android（APK とテスト APK。録画用のサーバ向け）"
    (
      set +u
      # shellcheck disable=SC1091
      . ./tools/android-env.sh >/dev/null 2>&1
      set -u
      # トークンはコマンド行（ps に見える）に出さず、Gradle のプロジェクト属性の環境変数で渡す。
      # daemon は使わない（片付けがこの環境のプロセスグループを止めるとき、他のビルドが使う daemon を巻き込まない）
      env "ORG_GRADLE_PROJECT_ashiato.apiToken=$(sed -n 's/^API_TOKEN=//p' .env)" \
          "ORG_GRADLE_PROJECT_ashiato.userId=$(sed -n 's/^ASHIATO_USER_ID=//p' .env)" \
        ./collector-android/gradlew -q --no-daemon -p collector-android :app:assembleDebug :app:assembleDebugAndroidTest \
          -Pashiato.baseUrl=http://127.0.0.1:18787
    ) && android_ok=1
    # APK が作れなくても画面のシナリオは撮る（Android のシナリオだけが「撮れなかった」になる）
    [ -n "$android_ok" ] || echo "warn: APK のビルドが落ちた。Android のシナリオは撮らない" >&2
  fi

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
    # 画面のログインが無い木（ST28 より前）では渡さない
    if grep -q '^WEB_PASSWORD=' .env; then echo "PASS_WEB_PASSWORD=$(sed -n 's/^WEB_PASSWORD=//p' .env)"; fi
    # 録画の間（各工程の後の停止 / 操作の間隔）。録画用の spec と設定が読む。変えるなら呼ぶ側で REC_HOLD_MS / REC_SLOWMO_MS
    echo "PASS_REC_HOLD_MS=${REC_HOLD_MS:-1200}"; echo "INFO_rec_hold_ms=${REC_HOLD_MS:-1200}"
    echo "PASS_REC_SLOWMO_MS=${REC_SLOWMO_MS:-250}"; echo "INFO_rec_slowmo_ms=${REC_SLOWMO_MS:-250}"
    echo "INFO_server_sha256=$(sha256sum .rec-bin/ashiato-server | cut -d' ' -f1)"
    echo "INFO_web_dist_sha256=$( (cd web/dist && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1)"
    echo "INFO_compose_project=$PROJECT"
    echo "INFO_ports=DB $db / API $api / 画面 $web"
    echo "INFO_seed=normal（STACK_RESET=1 で作り直した直後）"
    if [[ " ${REC_PLATFORMS:-} " == *" android "* ]] && [ -z "$android_ok" ]; then
      echo "ANDROID_UNAVAILABLE=APK のビルドが落ちた（logs/env-up.log）"
    elif [[ " ${REC_PLATFORMS:-} " == *" android "* ]]; then
      local apk="$WT/collector-android/app/build/outputs/apk/debug/app-debug.apk"
      local tapk="$WT/collector-android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
      echo "ANDROID_APK=$apk"; echo "ANDROID_TEST_APK=$tapk"; echo "ANDROID_REVERSE=18787:$api"
      echo "INFO_apk_sha256=$(sha256sum "$apk" | cut -d' ' -f1)"
      echo "INFO_test_apk_sha256=$(sha256sum "$tapk" | cut -d' ' -f1)"
    fi
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
