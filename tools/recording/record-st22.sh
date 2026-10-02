#!/usr/bin/env bash
# ST22「記録の削除」を録画する（人間が後から動画で見るため）。1 回の実行 = 1 つの記録フォルダ。
#
#   tools/recording/record-st22.sh setup            初回: 前提の道具が揃っているかを見て、保存先を作る
#   tools/recording/record-st22.sh run [<commit>]   録画する（既定 HEAD）。<commit> のコードから起動・録画する
#
# 流れ（run）:
#   1. <commit> を一時 worktree に取り出し、録画用のシナリオと設定（tools/recording/st22/）を web/ へ重ねる
#   2. その worktree でサーバ（release）と画面（vite build）を作る
#   3. 専用の DB（一時の compose project・空いている port・乱数の合言葉）で tools/stack.sh up（偽データ SEED=normal）
#   4. 同じ worktree の web/ を Windows 側へ写し、Windows の Playwright で操作・録画（video / trace）
#   5. 結果を記録フォルダへ: テストの合否・録画の有無・再生できるか は別々に記録する
#   6. 成功でも失敗でも、今回起動したプロセス・一時の DB（volume ごと）・worktree・Windows 側の作業場所を片付ける
#
# **自動で判定しないもの**: 動画の見やすさ、人間の承認（記録には常に「未実施」と書く）。
# **触らないもの**: 既存の DB・worktree・他の compose project・cargo の既定の target（専用のキャッシュを使う）。
#
# 環境変数:
#   REC_ROOT   記録の置き場（WSL のパス）。既定 /mnt/c/dev/ashiato2-recordings（= C:\dev\ashiato2-recordings）
#   REC_CACHE  ビルドのキャッシュ。既定 ~/.cache/ashiato2-rec（消してよい。次回の build が遅くなるだけ）
set -uo pipefail

TOOL_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(git -C "$TOOL_DIR" rev-parse --show-toplevel)"
REC_ROOT="${REC_ROOT:-/mnt/c/dev/ashiato2-recordings}"
REC_CACHE="${REC_CACHE:-$HOME/.cache/ashiato2-rec}"
CMD_EXE=/mnt/c/Windows/System32/cmd.exe
STORY=st22

say() { printf '== %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 2; }

# ---------------------------------------------------------------- setup
setup() {
  local miss=0
  for c in git docker cargo node npm python3 openssl ss flock wslpath "$CMD_EXE"; do
    if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then echo "  ok  $c"; else echo "  NG  $c が無い"; miss=1; fi
  done
  docker compose version >/dev/null 2>&1 && echo "  ok  docker compose" || { echo "  NG  docker compose が無い"; miss=1; }
  local winnode
  winnode="$(cd /mnt/c && "$CMD_EXE" /c "node -v" 2>/dev/null | tr -d '\r')"
  if [[ "$winnode" == v* ]]; then echo "  ok  Windows の node $winnode"; else echo "  NG  Windows に node が無い（https://nodejs.org から入れる）"; miss=1; fi
  mkdir -p "$REC_ROOT/$STORY" "$REC_CACHE" || miss=1
  echo "  記録の置き場: $REC_ROOT/$STORY  （Windows: $(wslpath -w "$REC_ROOT/$STORY" 2>/dev/null)）"
  echo "  キャッシュ:   $REC_CACHE"
  echo "  Playwright のブラウザは run の中で入れる（WSL・Windows とも。入っていれば何もしない）"
  [ "$miss" = 0 ] && echo "setup: OK" || { echo "setup: 足りないものがある"; exit 1; }
}

# ---------------------------------------------------------------- run
# 長い工程は裏で走らせて wait で待つ。wait の間なら Ctrl-C / SIGTERM の trap がすぐ効き、
# 片付けが子をプロセスグループごと止める（run の頭で set -m にするので、裏の工程は自分のグループを持つ）
bg() { "$@" & CHILD=$!; wait "$CHILD"; local rc=$?; CHILD=""; return "$rc"; }

free_port() { # $1 から上へ、待ち受けの無い番号を探す
  local p=$1
  while [ -n "$(ss -ltnH "sport = :$p" 2>/dev/null)" ]; do p=$((p + 1)); done
  echo "$p"
}

run() {
  local ref="${1:-HEAD}"
  local sha; sha="$(git -C "$REPO" rev-parse --verify "$ref^{commit}" 2>/dev/null)" || die "コミットが見つからない: $ref"
  local short=${sha:0:7}
  RUN_ID="$(date +%Y%m%d-%H%M%S)-$short"
  OUT="$REC_ROOT/$STORY/$RUN_ID"
  mkdir -p "$OUT/logs" "$OUT/overlay" "$REC_CACHE" || die "記録フォルダを作れない: $OUT"
  # 同時に 2 本走らせない（キャッシュの target を共有するため）
  exec 9>"$REC_CACHE/lock"
  flock -n 9 || die "別の録画が走っている（$REC_CACHE/lock）"

  WT="$REC_CACHE/wt-$RUN_ID"
  WINWORK="$REC_ROOT/_work/$RUN_ID"
  PROJECT="ashiato2rec$(echo "$RUN_ID" | tr -dc '0-9a-f')"
  STACK_PID=""; CHILD=""
  set -m
  T_START="$(date -Iseconds)"
  INVOCATION="tools/recording/record-st22.sh run $ref"
  FAILED_STEP=""
  trap finish EXIT
  trap 'FAILED_STEP="中断（シグナル）"; exit 130' INT TERM

  say "記録フォルダ: $OUT"
  say "対象: $sha"

  # ---- 1. 一時 worktree（対象コミットそのもの）+ 録画用ファイルを重ねる
  git -C "$REPO" worktree add --detach "$WT" "$sha" >"$OUT/logs/worktree.log" 2>&1 || { FAILED_STEP="worktree の作成"; exit 1; }
  mkdir -p "$WT/web/e2e-recording"
  cp "$TOOL_DIR/st22/playwright.recording.config.ts" "$WT/web/playwright.recording.config.ts"
  cp "$TOOL_DIR/st22/st22-erase-reload.rec.ts" "$WT/web/e2e-recording/st22-erase-reload.rec.ts"
  cp "$TOOL_DIR/st22/"* "$OUT/overlay/"
  [ -f "$WT/web/e2e/day-erase.spec.ts" ] || echo "warn: 対象コミットに web/e2e/day-erase.spec.ts が無い（ST22 より前のコミット？）" | tee -a "$OUT/logs/worktree.log"

  # ---- 2. 専用の DB と合言葉（乱数）。port は空いている番号
  DB_PORT="$(free_port 55440)"; API_PORT="$(free_port 18810)"; WEB_PORT="$(free_port 5210)"
  r() { openssl rand -hex 24; }
  local pg ow ap; pg="$(r)"; ow="$(r)"; ap="$(r)"
  umask 077
  cat >"$WT/.env" <<EOF
COMPOSE_PROJECT_NAME=$PROJECT
POSTGRES_PASSWORD=$pg
OWNER_DB_PASSWORD=$ow
APP_DB_PASSWORD=$ap
DATABASE_URL=postgres://ashiato_app:$ap@127.0.0.1:$DB_PORT/ashiato
DATABASE_OWNER_URL=postgres://ashiato_owner:$ow@127.0.0.1:$DB_PORT/ashiato
BIND=127.0.0.1:$API_PORT
API_TOKEN=$(r)
WEB_PASSWORD=$(r)
ASHIATO_USER_ID=$(cat /proc/sys/kernel/random/uuid)
ALLOWED_HOSTS=.example.ts.net
EOF
  umask 022
  cat >"$WT/docker-compose.override.yml" <<EOF
services:
  db:
    ports: !override
      - "127.0.0.1:$DB_PORT:5432"
EOF
  export COMPOSE_PROJECT_NAME="$PROJECT"

  # ---- 3. ビルド（対象コミットのコードから）
  say "ビルド（サーバ release・画面）"
  export CARGO_TARGET_DIR="$REC_CACHE/target"
  bg bash -c 'cd "$1" && cargo build -q --release -p ashiato-server --bin ashiato-server' _ "$WT" >"$OUT/logs/build.log" 2>&1 \
    || { FAILED_STEP="サーバのビルド（logs/build.log）"; exit 1; }
  mkdir -p "$WT/.rec-bin" && cp "$CARGO_TARGET_DIR/release/ashiato-server" "$WT/.rec-bin/ashiato-server"
  bg bash -c 'cd "$1" && npm ci --no-audit --no-fund && npm run build' _ "$WT/web" >>"$OUT/logs/build.log" 2>&1 \
    || { FAILED_STEP="画面のビルド（logs/build.log）"; exit 1; }
  # 再生できるかの確かめに WSL 側の Chromium を使う（入っていれば何もしない）
  bg bash -c 'cd "$1" && npx playwright install chromium' _ "$WT/web" >>"$OUT/logs/build.log" 2>&1 \
    || { FAILED_STEP="WSL 側の playwright install（logs/build.log）"; exit 1; }
  SERVER_SHA="$(sha256sum "$WT/.rec-bin/ashiato-server" | cut -c1-16)"
  WEB_SHA="$( (cd "$WT/web/dist" && find . -type f | sort | xargs sha256sum) | sha256sum | cut -c1-16)"

  # ---- 4. 起動（WSL）。専用 DB を作り直し、偽データを入れる
  say "起動: DB $DB_PORT / API $API_PORT / 画面 $WEB_PORT（compose project $PROJECT）"
  ( cd "$WT" && SERVER_BIN="$WT/.rec-bin/ashiato-server" WEB_DIST="$WT/web/dist" WEB_PORT="$WEB_PORT" \
      STACK_RESET=1 SEED=normal setsid ./tools/stack.sh up ) >"$OUT/logs/stack.log" 2>&1 &
  STACK_PID=$!
  local i
  for i in $(seq 1 300); do
    grep -q '^画面:' "$OUT/logs/stack.log" && break
    kill -0 "$STACK_PID" 2>/dev/null || { FAILED_STEP="起動（logs/stack.log）"; exit 1; }
    sleep 1
  done
  grep -q '^画面:' "$OUT/logs/stack.log" || { FAILED_STEP="起動が 300 秒で終わらない（logs/stack.log）"; exit 1; }

  # ---- 5. Windows 側へ同じ web/ を写して録画
  say "Windows 側の準備（同じ worktree の web/ を写して npm ci）"
  mkdir -p "$WINWORK"
  ( cd "$WT/web" && tar --exclude=node_modules --exclude=dist --exclude=test-results --exclude=playwright-report -cf - . ) \
    | ( cd "$WINWORK" && mkdir -p web && cd web && tar -xf - ) || { FAILED_STEP="Windows 側へ写す"; exit 1; }
  local wwork; wwork="$(wslpath -w "$WINWORK/web")"
  # Windows 側のコマンド行には必ず作業場所のパス（= RUN_ID）を含める。中断したとき、片付けがそれで探して止める
  #（WSL 側のプロセスを止めても、Windows 側の npm / node は残る。実測 2026-10-02）
  local pwbin="$wwork\node_modules\.bin\playwright.cmd"
  cd /mnt/c && bg "$CMD_EXE" /c "cd /d $wwork && npm ci --prefix $wwork --no-audit --no-fund && $pwbin install chromium" \
    >"$OUT/logs/windows-setup.log" 2>&1 || { FAILED_STEP="Windows 側の npm ci / playwright install（logs/windows-setup.log）"; exit 1; }

  say "録画（Windows の Playwright → WSL の画面 http://127.0.0.1:$WEB_PORT）"
  mkdir -p "$OUT/playwright"
  # 合言葉はコマンド行に出さない（WSLENV で環境変数として渡す）
  PW_CMD="cd /d $wwork && $pwbin test -c $wwork\playwright.recording.config.ts"
  WEB_PASSWORD="$(grep '^WEB_PASSWORD=' "$WT/.env" | cut -d= -f2)" \
  REC_BASE_URL="http://127.0.0.1:$WEB_PORT" \
  REC_OUT="$(wslpath -w "$OUT/playwright")\\results" \
  REC_REPORT="$(wslpath -w "$OUT/playwright")\\report" \
  WSLENV="WEB_PASSWORD:REC_BASE_URL:REC_OUT:REC_REPORT" \
    bg "$CMD_EXE" /c "$PW_CMD" >"$OUT/logs/playwright.log" 2>&1
  PW_RC=$?
  cd "$REPO" || true

  # ---- 6. 録画の有無と再生できるか（テストの合否とは別に記録する。見やすさは判定しない）
  say "録画を確かめる（ファイルがあるか・再生できるか）"
  local v
  : >"$OUT/video-check.jsonl"
  while IFS= read -r -d '' v; do
    printf '{"file":"%s","check":%s}\n' "${v#"$OUT"/}" \
      "$(node "$TOOL_DIR/check-video.mjs" "$WT/web/node_modules" "$v")" >>"$OUT/video-check.jsonl"
  done < <(find "$OUT/playwright/results" -name 'video.webm' -print0 2>/dev/null | sort -z)
  local main; main="$(find "$OUT/playwright/results" -path '*/e2e-recording-st22-erase-r*' -name video.webm 2>/dev/null | head -1)"
  [ -n "$main" ] && cp "$main" "$OUT/ST22-erase-reload.webm"
  main="$(find "$OUT/playwright/results" -path '*/e2e-recording-st22-erase-r*' -name trace.zip 2>/dev/null | head -1)"
  [ -n "$main" ] && cp "$main" "$OUT/ST22-erase-reload.trace.zip"
  exit 0
}

# ---------------------------------------------------------------- 終了時（成功でも失敗でも）
cleanup() {
  {
    echo "== 片付け $(date -Iseconds)"
    if [ -n "${CHILD:-}" ] && kill -0 "$CHILD" 2>/dev/null; then
      kill -TERM -- "-$CHILD" 2>/dev/null; sleep 1; kill -KILL -- "-$CHILD" 2>/dev/null
      echo "途中の工程: 止めた（pgid $CHILD）"
    fi
    if [ -n "${RUN_ID:-}" ]; then
      # Windows 側で、コマンド行にこの実行の RUN_ID を含むプロセスだけを木ごと止める（他には触らない）
      local wp
      wp="$(cd /mnt/c && /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe -NoProfile -Command \
        "Get-CimInstance Win32_Process | Where-Object { \$_.ProcessId -ne \$PID -and \$_.CommandLine -like '*$RUN_ID*' -and \$_.Name -ne 'powershell.exe' } | ForEach-Object { \$_.ProcessId }" 2>/dev/null | tr -d '\r' | tr '\n' ' ')"
      for pid in $wp; do (cd /mnt/c && /mnt/c/Windows/System32/taskkill.exe /T /F /PID "$pid" >/dev/null 2>&1); done
      [ -n "${wp// /}" ] && echo "Windows のプロセス: 止めた（$wp）" || echo "Windows のプロセス: 残っていない"
    fi
    if [ -n "${STACK_PID:-}" ] && kill -0 "$STACK_PID" 2>/dev/null; then
      # setsid で立てたので、プロセスグループごと止める（サーバ・vite preview も同じグループ）
      local pg; pg="$(ps -o pgid= -p "$STACK_PID" 2>/dev/null | tr -d ' ')"
      [ -n "$pg" ] && kill -TERM -- "-$pg" 2>/dev/null
      for _ in $(seq 1 20); do kill -0 "$STACK_PID" 2>/dev/null || break; sleep 0.5; done
      kill -0 "$STACK_PID" 2>/dev/null && [ -n "$pg" ] && kill -KILL -- "-$pg" 2>/dev/null
      echo "stack: 止めた（pgid $pg）"
    fi
    if [ -n "${PROJECT:-}" ] && [ -d "${WT:-/nonexistent}" ]; then
      # **この実行の compose project だけ**を volume ごと消す（他の project には触らない）
      (cd "$WT" && docker compose -p "$PROJECT" down -v --remove-orphans) 2>&1 | tail -3
    fi
    if [ -n "${WT:-}" ] && [ -d "$WT" ]; then
      git -C "$REPO" worktree remove --force "$WT" && echo "worktree: 消した $WT"
    fi
    git -C "$REPO" worktree prune
    if [ -n "${WINWORK:-}" ] && [ -d "$WINWORK" ]; then
      (cd /mnt/c && "$CMD_EXE" /c "rmdir /s /q $(wslpath -w "$WINWORK")") >/dev/null 2>&1
      rm -rf "$WINWORK" 2>/dev/null
      [ -d "$WINWORK" ] && echo "Windows の作業場所: 消せなかった $WINWORK" || echo "Windows の作業場所: 消した"
    fi
    rmdir "$REC_ROOT/_work" 2>/dev/null || true
    local left=""
    for p in ${DB_PORT:-} ${API_PORT:-} ${WEB_PORT:-}; do [ -n "$(ss -ltnH "sport = :$p" 2>/dev/null)" ] && left="$left $p"; done
    [ -z "$left" ] && echo "port: 解放を確かめた（${DB_PORT:-} ${API_PORT:-} ${WEB_PORT:-}）" || echo "port: まだ使われている:$left"
    docker ps -a --filter "label=com.docker.compose.project=${PROJECT:-none}" --format '{{.Names}}' | grep -q . \
      && echo "container: 残っている（project ${PROJECT:-}）" || echo "container: 残っていない（project ${PROJECT:-}）"
  } >>"$OUT/logs/cleanup.log" 2>&1
}

finish() {
  local rc=$?
  trap - EXIT INT TERM
  [ -n "${OUT:-}" ] || exit "$rc"
  cleanup
  python3 "$TOOL_DIR/summarize.py" \
    --out "$OUT" --run-id "$RUN_ID" --sha "$sha_full" --repo "$REPO" --tool-dir "$TOOL_DIR" \
    --started "$T_START" --invocation "$INVOCATION" --failed-step "${FAILED_STEP:-}" \
    --pw-cmd "${PW_CMD:-}" --server-sha "${SERVER_SHA:-}" --web-sha "${WEB_SHA:-}" \
    --ports "${DB_PORT:-}/${API_PORT:-}/${WEB_PORT:-}" --project "${PROJECT:-}" --pw-rc "${PW_RC:-}"
  local s=$?
  echo
  echo "記録: $OUT"
  echo "      （Windows: $(wslpath -w "$OUT")）"
  exit "$s"
}

case "${1:-}" in
  setup) setup ;;
  run) shift
       sha_full="$(git -C "$REPO" rev-parse --verify "${1:-HEAD}^{commit}" 2>/dev/null)" || die "コミットが見つからない: ${1:-HEAD}"
       run "$@" ;;
  *) sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
