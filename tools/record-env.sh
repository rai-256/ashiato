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

# 合成の書庫を 2 冊置く（ST12 の録画用。tools/smoke.sh の書庫の段と同じ手順）:
#   1 冊目 rec-youtube.zip（YouTube の視聴履歴）—— 形を本人の代わりに確認して取り込ませる
#   2 冊目 rec-pending.zip（マイアクティビティの検索）—— 形の確認待ちのまま残す
# 本人の作業（tools/archive-shape.sh で形を見て印を置く）はターミナルの操作で、画面からは押せないので、ここで済ませる。
# 標準出力の最後の 1 行が要約（ready の INFO_archive）。
archive_prepare() {
  local api="$1" uid token admin shape status i
  uid="$ASHIATO_ARCHIVE_USER_ID"; token="$(sed -n 's/^API_TOKEN=//p' .env)"
  admin="$(sed -n 's/^ *POSTGRES_USER: *//p' docker-compose.yml | head -1)"
  q() { docker compose exec -T db psql -qtA -U "$admin" -d ashiato -c "$1" 2>/dev/null; }
  st() { curl -sf -H "authorization: Bearer $token" "http://127.0.0.1:$api/archives/status?user_id=$uid"; }
  zipdir() { python3 -c 'import sys,zipfile,pathlib as P
src,dst=P.Path(sys.argv[1]),sys.argv[2]
with zipfile.ZipFile(dst+".part","w") as z:
    [z.write(f,f.relative_to(src)) for f in sorted(src.rglob("*")) if f.is_file()]
P.Path(dst+".part").rename(dst)' "$1" "$2"; }
  local a="$ARCHIVE_DIR/src/a" b="$ARCHIVE_DIR/src/b"
  mkdir -p "$a/Takeout/YouTube" "$b/Takeout/My Activity/Search"
  printf '%s' '[{"header":"YouTube","title":"録画の確認の動画 を視聴しました","titleUrl":"https://www.youtube.com/watch?v=rec1","time":"2026-09-20T03:00:00Z"},{"header":"YouTube","title":"録画の確認の動画 2 を視聴しました","titleUrl":"https://www.youtube.com/watch?v=rec2","time":"2026-09-21T12:30:00Z"}]' \
    >"$a/Takeout/YouTube/watch-history.json"
  printf '%s' '[{"header":"検索","title":"録画の確認 を検索しました","time":"2026-09-21T03:00:00Z","products":["Search"]}]' \
    >"$b/Takeout/My Activity/Search/MyActivity.json"
  zipdir "$a" "$ASHIATO_INBOX_DIR/rec-youtube.zip" || { echo "1 冊目を作れない"; return 1; }
  for i in $(seq 1 60); do
    shape="$(q "SELECT shape_hash FROM core.archive_pending_shape WHERE user_id = '$uid'::uuid LIMIT 1")"
    [ -n "$shape" ] && break; sleep 1
  done
  [ -n "$shape" ] || { echo "1 冊目が形の確認待ちにならない（60 秒）"; return 1; }
  DATABASE_URL="$(sed -n 's/^DATABASE_URL=//p' .env)" ASHIATO_ARCHIVE_USER_ID="$uid" tools/archive-shape.sh --confirm "$shape" >/dev/null 2>&1 \
    || { echo "tools/archive-shape.sh --confirm が落ちた"; return 1; }
  for i in $(seq 1 60); do
    status="$(st)"
    printf '%s' "$status" | python3 -c 'import json,sys; d=json.load(sys.stdin); l=d.get("latest_archive") or {}; sys.exit(0 if l.get("file_name")=="rec-youtube.zip" and l.get("outcome") not in ("pending_shape",None) else 1)' 2>/dev/null && break
    status=""; sleep 1
  done
  [ -n "$status" ] || { echo "1 冊目が取り込まれない（60 秒）"; return 1; }
  zipdir "$b" "$ASHIATO_INBOX_DIR/rec-pending.zip" || { echo "2 冊目を作れない"; return 1; }
  for i in $(seq 1 60); do
    st | python3 -c 'import json,sys; d=json.load(sys.stdin); l=d.get("latest_archive") or {}; sys.exit(0 if d.get("pending_shape") and l.get("file_name")=="rec-pending.zip" and l.get("outcome")=="pending_shape" else 1)' 2>/dev/null && break
    sleep 1
  done
  st | python3 -c 'import json,sys; d=json.load(sys.stdin); l=d.get("latest_archive") or {}; sys.exit(0 if d.get("pending_shape") and l.get("file_name")=="rec-pending.zip" and l.get("outcome")=="pending_shape" else 1)' 2>/dev/null \
    || { echo "2 冊目が形の確認待ちにならない（60 秒）"; return 1; }
  echo "rec-youtube.zip を取り込み済み・rec-pending.zip が形の確認待ち"
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
    # ST28 より前の木（ST28 を含まない Story のブランチ）: DB の合言葉はその木の docker-compose.yml の固定値で、
    # 画面のログインも無い（実測 2026-10-03: ST05 の木に上の形を渡すと DB の認証で落ちた）。DB は loopback の専用 project。
    # 値はその木の docker-compose.yml から読む（この台本に字面で書かない。ST28 の tools/check-db-secret.sh）
    local old_user old_pw
    old_user="$(sed -n 's/^ *POSTGRES_USER: *//p' docker-compose.yml | head -1)"
    old_pw="$(sed -n 's/^ *POSTGRES_PASSWORD: *//p' docker-compose.yml | head -1)"
    [ -n "$old_user" ] && [ -n "$old_pw" ] || { echo "error: docker-compose.yml から DB の利用者と合言葉を読めない" >&2; exit 1; }
    cat >.env <<EOF || exit 1
COMPOSE_PROJECT_NAME=$PROJECT
DATABASE_URL=postgres://${old_user}:${old_pw}@127.0.0.1:$db/ashiato
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
  # worktree ごとの port の規則（ST05 の tools/ports.sh）を持つ木では、規則が名前から port を決めて
  # .env の URL と BIND の port を差し替える。録画の worktree の名前は -st<NN> で終わらないので規則は 55432 / 18787
  # —— **手元の開発用の DB とサーバ**になる。ここで選んだ port を規則より先に渡す（ports.sh は既にある値を使う）
  export ASHIATO_DB_PORT="$db" ASHIATO_HTTP_PORT="$api"
  # 書庫の取り込み器（ST12 の入った木だけ）。置き場は**必ずこの実行の一時の場所**にする ——
  # 渡さないと既定で本人のダウンロードフォルダを見に行く（archive/config.rs）
  ARCHIVE_DIR=""
  if [ -f crates/server/src/archive/config.rs ]; then
    ARCHIVE_DIR="$STATE/archive"
    mkdir -p "$ARCHIVE_DIR/inbox" "$ARCHIVE_DIR/downloads" "$ARCHIVE_DIR/copies" "$ARCHIVE_DIR/src" || exit 1
    # 利用者は**既定の利用者**（全部 0）。画面は名乗らずに `/archives/status` を読み、サーバは既定の利用者で答える
    # （seed と tools/smoke.sh の書庫も同じ。実測 2026-10-05: .env の利用者で動かすと、箱は取り込み器を一度も見なかった）
    export ASHIATO_ARCHIVE_USER_ID="00000000-0000-0000-0000-000000000000"
    export ASHIATO_INBOX_DIR="$ARCHIVE_DIR/inbox" ASHIATO_DOWNLOADS_DIR="$ARCHIVE_DIR/downloads"
    export ASHIATO_ARCHIVE_COPY_DIR="$ARCHIVE_DIR/copies" ASHIATO_ARCHIVE_SCAN_SEC=1
  fi

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
  local archive_info="無し（この木に書庫の取り込み器が無い）"
  if [ -n "$ARCHIVE_DIR" ]; then
    # 落ちても他のシナリオは撮る（書庫のシナリオのアサーションが落ちる）
    archive_info="$(archive_prepare "$api")" || echo "warn: 書庫の用意が落ちた: $archive_info" >&2
  fi

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
    echo "INFO_archive=$archive_info"
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
