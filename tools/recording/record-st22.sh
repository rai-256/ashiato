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
#   5. 結果を記録フォルダへ: テストの合否・録画の有無・再生できるか・片付け は別々に記録する
#   6. 成功でも失敗でも、今回起動したプロセス・一時の DB（volume ごと）・worktree・Windows 側の作業場所を片付ける
#
# **自動で判定しないもの**: 動画の見やすさ、人間の承認（記録には常に「未実施」と書く）。
# **触らないもの**: 既存の DB・worktree・他の compose project・cargo の既定の target（専用のキャッシュを使う）。
# rc: 0 = テストが通り・本命の動画が再生でき・片付けに残りが無い / 1 = それ以外 / 2 = 使い方・前提の誤り
#
# 環境変数:
#   REC_ROOT   記録の置き場（WSL のパス。/mnt/<drive>/ の下で、空白などを含まないこと）。
#              既定 /mnt/c/dev/ashiato2-recordings（= C:\dev\ashiato2-recordings）
#   REC_CACHE  ビルドのキャッシュ。既定 ~/.cache/ashiato2-rec（消してよい。次回の build が遅くなるだけ）
#   REC_HOLD_MS / REC_SLOWMO_MS  録画の停止と操作の間隔（既定 1200 / 250）。Windows の Playwright へ渡し、記録に残す
set -uo pipefail

TOOL_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(git -C "$TOOL_DIR" rev-parse --show-toplevel 2>/dev/null)" || { echo "error: git のリポジトリの中に置く" >&2; exit 2; }
REC_ROOT="${REC_ROOT:-/mnt/c/dev/ashiato2-recordings}"
REC_CACHE="${REC_CACHE:-$HOME/.cache/ashiato2-rec}"
REC_HOLD_MS="${REC_HOLD_MS:-1200}"
REC_SLOWMO_MS="${REC_SLOWMO_MS:-250}"
CMD_EXE=/mnt/c/Windows/System32/cmd.exe
POWERSHELL=/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
TASKKILL=/mnt/c/Windows/System32/taskkill.exe
STORY=st22

say() { printf '== %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 2; }

# REC_ROOT は Windows のコマンド行（cmd.exe）にそのまま載る。空白や記号があると cmd が別のパスとして読むので、
# 安全な形だけを許す（独立レビュー C1）。WSL 側のパス（/home/…）は Windows から UNC になり `cd /d` できない
check_rec_root() {
  [[ "$REC_ROOT" =~ ^/mnt/[a-z]/[A-Za-z0-9._/-]+$ ]] && [[ "$REC_ROOT" != *..* ]] \
    || die "REC_ROOT は /mnt/<drive>/ の下で、英数字・. _ - / だけのパスにする（いま: $REC_ROOT）"
}

# ---------------------------------------------------------------- setup
setup() {
  check_rec_root
  local miss=0 c winnode
  for c in git docker cargo node npm python3 openssl ss flock setsid timeout wslpath "$CMD_EXE" "$POWERSHELL" "$TASKKILL"; do
    if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then echo "  ok  $c"; else echo "  NG  $c が無い"; miss=1; fi
  done
  if docker compose version >/dev/null 2>&1; then echo "  ok  docker compose"; else echo "  NG  docker compose が無い"; miss=1; fi
  winnode="$(cd /mnt/c && "$CMD_EXE" /c "node -v" 2>/dev/null </dev/null | tr -d '\r')"
  if [[ "$winnode" == v* ]]; then echo "  ok  Windows の node $winnode"; else echo "  NG  Windows に node が無い（https://nodejs.org から入れる）"; miss=1; fi
  mkdir -p "$REC_ROOT/$STORY" "$REC_CACHE" || miss=1
  echo "  記録の置き場: $REC_ROOT/$STORY  （Windows: $(wslpath -w "$REC_ROOT/$STORY" 2>/dev/null)）"
  echo "  キャッシュ:   $REC_CACHE"
  echo "  Playwright のブラウザは run の中で入れる（WSL・Windows とも。入っていれば何もしない）"
  if [ "$miss" = 0 ]; then echo "setup: OK"; else echo "setup: 足りないものがある"; exit 1; fi
}

# ---------------------------------------------------------------- run
# 長い工程は裏で走らせて wait で待つ。wait の間なら Ctrl-C / SIGTERM の trap がすぐ効く。
# 子は setsid で自分のセッション（= プロセスグループ）を持つので、片付けがグループごと止められる。
# **set -m は使わない** —— 使うと前景のコマンドが端末の前景を取り、Ctrl-C がスクリプトに届かない（独立レビュー I2）。
# 子には端末の stdin（SIGTTIN で止まる）とロックの fd 9（子が残ると次の実行が止まる）を渡さない
bg() { setsid "$@" </dev/null 9>&- & CHILD=$!; wait "$CHILD"; local rc=$?; CHILD=""; return "$rc"; }

# 待ち受けの無い番号を $1 から上へ探す。WSL 側に加えて、$2 に渡した Windows 側の待ち受けの一覧も避ける
#（Windows 側に同じ番号の待ち受けがあると、Windows の Playwright はそちらへ繋がる。独立レビュー I8）
free_port() {
  local p=$1 win=" ${2:-} "
  while [ -n "$(ss -ltnH "sport = :$p" 2>/dev/null)" ] || [[ "$win" == *" $p "* ]]; do p=$((p + 1)); done
  echo "$p"
}

fail() { FAILED_STEP="$1"; exit 1; }

run() {
  check_rec_root
  local ref="${1:-HEAD}"
  [ -n "$SHA_FULL" ] || die "コミットが見つからない: $ref"
  # 同時に 2 本走らせない（キャッシュの target を共有するため）。記録フォルダより先に取る
  mkdir -p "$REC_CACHE" || die "キャッシュを作れない: $REC_CACHE"
  exec 9>"$REC_CACHE/lock"
  flock -n 9 || die "別の録画が走っている（$REC_CACHE/lock）"

  RUN_ID="$(date +%Y%m%d-%H%M%S)-${SHA_FULL:0:7}"
  OUT="$REC_ROOT/$STORY/$RUN_ID"
  WT="$REC_CACHE/wt-$RUN_ID"
  WINWORK="$REC_ROOT/_work/$RUN_ID"
  PROJECT="ashiato2rec$(echo "$RUN_ID" | tr -dc '0-9a-f')"
  STACK_PID=""; CHILD=""; FAILED_STEP=""; PW_RC=""; PW_CMD=""
  T_START="$(date -Iseconds)"
  INVOCATION="tools/recording/record-st22.sh run $ref"
  mkdir -p "$OUT/logs" "$OUT/overlay" || die "記録フォルダを作れない: $OUT"
  trap finish EXIT
  trap 'FAILED_STEP="中断（SIGINT）"; exit 130' INT
  trap 'FAILED_STEP="中断（SIGTERM）"; exit 143' TERM

  say "記録フォルダ: $OUT"
  say "対象: $SHA_FULL"
  # 道具の版は**始めに**取る（実行中に編集・commit しても、走ったものを記録する。独立レビュー Minor）
  {
    echo "tool_commit=$(git -C "$REPO" rev-parse HEAD)"
    git -C "$REPO" status --porcelain --untracked-files=all -- tools/recording | sed 's/^/dirty=/'
  } >"$OUT/tool-status.txt"

  # ---- 1. 一時 worktree（対象コミットそのもの）+ 録画用ファイルを重ねる
  git -C "$REPO" worktree add --detach "$WT" "$SHA_FULL" >"$OUT/logs/worktree.log" 2>&1 || fail "worktree の作成（logs/worktree.log）"
  [ -f "$WT/web/e2e/day-erase.spec.ts" ] && [ -f "$WT/web/e2e/global-setup.ts" ] \
    || fail "対象コミットに web/e2e/day-erase.spec.ts か global-setup.ts が無い（ST22 より前のコミット？）"
  { mkdir -p "$WT/web/e2e-recording" \
      && cp "$TOOL_DIR/st22/playwright.recording.config.ts" "$WT/web/playwright.recording.config.ts" \
      && cp "$TOOL_DIR/st22/st22-erase-reload.rec.ts" "$WT/web/e2e-recording/st22-erase-reload.rec.ts" \
      && cp "$TOOL_DIR/st22/"* "$OUT/overlay/"; } || fail "録画用ファイルを重ねる"

  # ---- 2. 専用の DB と合言葉（乱数）。port は WSL・Windows の両方で空いている番号
  local winports
  winports="$(cd /mnt/c && "$POWERSHELL" -NoProfile -Command \
    "(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue).LocalPort | Sort-Object -Unique" </dev/null 2>/dev/null | tr -d '\r' | tr '\n' ' ')"
  DB_PORT="$(free_port 55440)"; API_PORT="$(free_port 18810 "$winports")"; WEB_PORT="$(free_port 5210 "$winports")"
  r() { openssl rand -hex 24; }
  local pg ow ap old_umask; pg="$(r)"; ow="$(r)"; ap="$(r)"
  old_umask="$(umask)"; umask 077
  cat >"$WT/.env" <<EOF || { umask "$old_umask"; fail "専用の .env を書く"; }
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
  umask "$old_umask"
  cat >"$WT/docker-compose.override.yml" <<EOF || fail "DB の port の上書きを書く"
services:
  db:
    ports: !override
      - "127.0.0.1:$DB_PORT:5432"
EOF
  export COMPOSE_PROJECT_NAME="$PROJECT"

  # ---- 3. ビルド（対象コミットのコードから）
  say "ビルド（サーバ release・画面）"
  export CARGO_TARGET_DIR="$REC_CACHE/target"
  # shellcheck disable=SC2016  # $1 は内側の bash が展開する（cd 先を引数で渡す）
  bg bash -c 'cd "$1" && cargo build -q --release -p ashiato-server --bin ashiato-server' _ "$WT" >"$OUT/logs/build.log" 2>&1 \
    || fail "サーバのビルド（logs/build.log）"
  { mkdir -p "$WT/.rec-bin" && cp "$CARGO_TARGET_DIR/release/ashiato-server" "$WT/.rec-bin/ashiato-server"; } \
    || fail "ビルドしたサーバを写す"
  # shellcheck disable=SC2016
  bg bash -c 'cd "$1" && npm ci --no-audit --no-fund && npm run build' _ "$WT/web" >>"$OUT/logs/build.log" 2>&1 \
    || fail "画面のビルド（logs/build.log）"
  [ -f "$WT/web/dist/index.html" ] || fail "画面のビルドに index.html が無い（logs/build.log）"
  # 再生できるかの確かめに WSL 側の Chromium を使う（入っていれば何もしない）
  # shellcheck disable=SC2016
  bg bash -c 'cd "$1" && npx playwright install chromium' _ "$WT/web" >>"$OUT/logs/build.log" 2>&1 \
    || fail "WSL 側の playwright install（logs/build.log）"
  {
    echo "server_sha256=$(sha256sum "$WT/.rec-bin/ashiato-server" | cut -d' ' -f1)"
    echo "web_dist_sha256=$( (cd "$WT/web/dist" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1)"
    echo "playwright_wsl=$(cd "$WT/web" && npx playwright --version 2>/dev/null)"
  } >"$OUT/build-info.txt"

  # ---- 4. 起動（WSL）。専用 DB を作り直し、偽データを入れる
  say "起動: DB $DB_PORT / API $API_PORT / 画面 $WEB_PORT（compose project $PROJECT）"
  # setsid で自分のグループを持たせる（サーバ・vite preview も同じグループ）。exec でつなぐので STACK_PID = グループの番号
  ( cd "$WT" && SERVER_BIN="$WT/.rec-bin/ashiato-server" WEB_DIST="$WT/web/dist" WEB_PORT="$WEB_PORT" \
      STACK_RESET=1 SEED=normal exec setsid ./tools/stack.sh up ) </dev/null >"$OUT/logs/stack.log" 2>&1 9>&- &
  STACK_PID=$!
  local i
  for i in $(seq 1 300); do
    grep -q '^画面:' "$OUT/logs/stack.log" && break
    kill -0 "$STACK_PID" 2>/dev/null || fail "起動（logs/stack.log）"
    sleep 1
  done
  grep -q '^画面:' "$OUT/logs/stack.log" || fail "起動が ${i} 秒で終わらない（logs/stack.log）"

  # ---- 5. Windows 側へ同じ web/ を写して録画
  say "Windows 側の準備（同じ worktree の web/ を写して npm ci）"
  { mkdir -p "$WINWORK/web" \
      && (cd "$WT/web" && tar --exclude=./node_modules --exclude=./dist --exclude=./test-results --exclude=./playwright-report -cf - .) \
         | (cd "$WINWORK/web" && tar -xf -); } || fail "Windows 側へ写す"
  local wwork pwbin
  wwork="$(wslpath -w "$WINWORK/web")"
  # Windows 側のコマンド行には必ず作業場所のパス（…\_work\<RUN_ID>\…）を含める。片付けがそれで探して止める
  #（WSL 側のプロセスを止めても、Windows 側の npm / node は残る。実測 2026-10-02）。
  # パスは check_rec_root で空白・記号を含まないことを確かめてあるので、引用符なしで cmd に渡せる
  pwbin="$wwork\\node_modules\\.bin\\playwright.cmd"
  cd /mnt/c || fail "cd /mnt/c"   # cmd.exe を UNC の cwd で起こさない
  bg "$CMD_EXE" /c "cd /d $wwork && npm ci --prefix $wwork --no-audit --no-fund && $pwbin install chromium" \
    >"$OUT/logs/windows-setup.log" 2>&1 || fail "Windows 側の npm ci / playwright install（logs/windows-setup.log）"
  cd "$REPO" || true
  echo "npm_windows=$(cd /mnt/c && "$CMD_EXE" /c "npm -v" </dev/null 2>/dev/null | tr -d '\r')" >>"$OUT/build-info.txt"

  say "録画（Windows の Playwright → WSL の画面 http://127.0.0.1:$WEB_PORT）"
  mkdir -p "$OUT/playwright" || fail "記録フォルダに playwright/ を作る"
  PW_CMD="cd /d $wwork && $pwbin test -c $wwork\\playwright.recording.config.ts"
  # 合言葉はコマンド行に出さない（WSLENV で環境変数として渡す）。利用者の WSLENV は残して足す
  # bg は前景の関数で呼ぶ（サブシェルに入れると CHILD が親に戻らず、中断のときに片付けが子を止められない）
  cd /mnt/c || fail "cd /mnt/c"
  WEB_PASSWORD="$(grep '^WEB_PASSWORD=' "$WT/.env" | cut -d= -f2)" \
  REC_BASE_URL="http://127.0.0.1:$WEB_PORT" \
  REC_OUT="$(wslpath -w "$OUT/playwright")\\results" \
  REC_REPORT="$(wslpath -w "$OUT/playwright")\\report" \
  REC_HOLD_MS="$REC_HOLD_MS" REC_SLOWMO_MS="$REC_SLOWMO_MS" \
  WSLENV="${WSLENV:+$WSLENV:}WEB_PASSWORD:REC_BASE_URL:REC_OUT:REC_REPORT:REC_HOLD_MS:REC_SLOWMO_MS" \
    bg "$CMD_EXE" /c "$PW_CMD" >"$OUT/logs/playwright.log" 2>&1
  PW_RC=$?
  cd "$REPO" || true

  # ---- 6. 録画の有無と再生できるか（テストの合否とは別に記録する。見やすさは判定しない）
  say "録画を確かめる（ファイルがあるか・再生できるか）"
  local v res
  : >"$OUT/video-check.jsonl"
  while IFS= read -r -d '' v; do
    res="$(timeout 90 node "$TOOL_DIR/check-video.mjs" "$WT/web/node_modules" "$v" </dev/null 2>/dev/null)"
    [[ "$res" == \{* ]] || res='{"exists":true,"playable":false,"error":"確かめが返らない（時間切れか異常終了）"}'
    printf '{"file":"%s","check":%s}\n' "${v#"$OUT"/}" "$res" >>"$OUT/video-check.jsonl"
  done < <(find "$OUT/playwright/results" -name 'video.webm' -print0 2>/dev/null | LC_ALL=C sort -z)
  local main
  main="$(find "$OUT/playwright/results" -path '*/e2e-recording-st22-erase-r*' -name video.webm 2>/dev/null | head -1)"
  if [ -n "$main" ]; then cp "$main" "$OUT/ST22-erase-reload.webm" || fail "本命の動画を写す"; fi
  main="$(find "$OUT/playwright/results" -path '*/e2e-recording-st22-erase-r*' -name trace.zip 2>/dev/null | head -1)"
  if [ -n "$main" ]; then cp "$main" "$OUT/ST22-erase-reload.trace.zip" || fail "本命の trace を写す"; fi
  exit 0
}

# ---------------------------------------------------------------- 終了時（成功でも失敗でも）
# 片付けは**この実行の名前を持つものだけ**に当てる。残ったものは LEFT に貯めて記録と rc に出す（独立レビュー I7）
cleanup() {
  LEFT=""
  {
    echo "== 片付け $(date -Iseconds)"
    if [ -n "${CHILD:-}" ] && kill -0 -- "-$CHILD" 2>/dev/null; then
      kill -TERM -- "-$CHILD" 2>/dev/null; sleep 1; kill -KILL -- "-$CHILD" 2>/dev/null
      echo "途中の工程: 止めた（pgid $CHILD）"
    fi
    # Windows 側: コマンド行にこの実行の作業場所（\_work\<RUN_ID>\）を含む cmd.exe / node.exe だけを木ごと止める。
    # RUN_ID だけで探すと、記録フォルダのファイルを開いたアプリ（Code.exe など）まで当たる（独立レビュー I1）
    local wp pid
    wp="$(cd /mnt/c && "$POWERSHELL" -NoProfile -Command \
      "Get-CimInstance Win32_Process | Where-Object { (\$_.Name -eq 'cmd.exe' -or \$_.Name -eq 'node.exe') -and \$_.CommandLine -like '*\\_work\\$RUN_ID\\*' } | ForEach-Object { \$_.ProcessId }" \
      </dev/null 2>/dev/null | tr -d '\r' | tr '\n' ' ')"
    for pid in $wp; do (cd /mnt/c && "$TASKKILL" /T /F /PID "$pid" </dev/null >/dev/null 2>&1); done
    if [ -n "${wp// /}" ]; then echo "Windows のプロセス: 止めた（$wp）"; else echo "Windows のプロセス: 残っていない"; fi
    if [ -n "${STACK_PID:-}" ] && kill -0 -- "-$STACK_PID" 2>/dev/null; then
      kill -TERM -- "-$STACK_PID" 2>/dev/null
      for _ in $(seq 1 20); do kill -0 -- "-$STACK_PID" 2>/dev/null || break; sleep 0.5; done
      kill -0 -- "-$STACK_PID" 2>/dev/null && kill -KILL -- "-$STACK_PID" 2>/dev/null
      echo "stack: 止めた（pgid $STACK_PID）"
    fi
    if [ -d "$WT" ]; then
      # **この実行の compose project だけ**を volume ごと消す（他の project には触らない）
      (cd "$WT" && docker compose -p "$PROJECT" down -v --remove-orphans </dev/null) 2>&1 | tail -3
      git -C "$REPO" worktree remove --force "$WT" && echo "worktree: 消した $WT"
      # prune はしない —— リポジトリ全体の古い worktree の管理情報まで消す（独立レビュー Minor）
    fi
    if [ -d "$WINWORK" ]; then
      rm -rf -- "${WINWORK:?}"
      if [ -d "$WINWORK" ]; then echo "Windows の作業場所: 消せなかった $WINWORK"; else echo "Windows の作業場所: 消した"; fi
    fi
    rmdir "$REC_ROOT/_work" 2>/dev/null || true
  } >>"$OUT/logs/cleanup.log" 2>&1
  # 残りの確かめ（記録と rc に出す）
  local p
  for p in ${DB_PORT:-} ${API_PORT:-} ${WEB_PORT:-}; do
    [ -n "$(ss -ltnH "sport = :$p" 2>/dev/null)" ] && LEFT="$LEFT port:$p"
  done
  docker ps -a --filter "label=com.docker.compose.project=$PROJECT" --format '{{.Names}}' | grep -q . && LEFT="$LEFT container"
  docker volume ls -q --filter "label=com.docker.compose.project=$PROJECT" | grep -q . && LEFT="$LEFT volume"
  [ -d "$WT" ] && LEFT="$LEFT worktree"
  git -C "$REPO" worktree list --porcelain | grep -qxF "worktree $WT" && LEFT="$LEFT worktree-registration"
  [ -d "$WINWORK" ] && LEFT="$LEFT windows-work"
  [ -n "$(cd /mnt/c && "$POWERSHELL" -NoProfile -Command \
    "Get-CimInstance Win32_Process | Where-Object { \$_.CommandLine -like '*\\_work\\$RUN_ID\\*' -and \$_.Name -ne 'powershell.exe' } | ForEach-Object { \$_.ProcessId }" \
    </dev/null 2>/dev/null | tr -d '\r\n ')" ] && LEFT="$LEFT windows-process"
  if [ -z "$LEFT" ]; then echo "ok" >"$OUT/cleanup-status.txt"; else echo "leftover:$LEFT" >"$OUT/cleanup-status.txt"; fi
  echo "残り: ${LEFT:-なし}" >>"$OUT/logs/cleanup.log"
}

finish() {
  local rc=$?
  trap - EXIT
  trap '' INT TERM   # 片付けの最中の 2 回目の Ctrl-C で片付けが途切れないように
  [ -n "${OUT:-}" ] && [ -d "$OUT" ] || exit "$rc"
  [ "$rc" != 0 ] && [ -z "${FAILED_STEP:-}" ] && FAILED_STEP="途中で止まった（rc=$rc）"
  cleanup
  # 記録（失敗・中断も書く）。summarize が落ちても一覧には 1 行残す
  python3 "$TOOL_DIR/summarize.py" \
    --out "$OUT" --run-id "$RUN_ID" --sha "$SHA_FULL" --started "$T_START" --invocation "$INVOCATION" \
    --failed-step "${FAILED_STEP:-}" --pw-cmd "${PW_CMD:-}" --pw-rc "${PW_RC:-}" \
    --ports "${DB_PORT:-}/${API_PORT:-}/${WEB_PORT:-}" --project "$PROJECT" \
    --hold-ms "$REC_HOLD_MS" --slowmo-ms "$REC_SLOWMO_MS"
  local s=$?
  if [ "$s" -gt 1 ]; then
    printf '%s\t%s\t%s\n' "$RUN_ID" "${SHA_FULL:0:12}" "summarize が落ちた（rc=$s）。logs/ を見る" >>"$REC_ROOT/$STORY/runs-errors.tsv"
    s=1
  fi
  # 中断・工程の失敗は、記録の中身に関わらず成功にしない（独立レビュー I3）
  [ -n "${FAILED_STEP:-}" ] && s=1
  echo
  echo "記録: $OUT"
  echo "      （Windows: $(wslpath -w "$OUT")）"
  exit "$s"
}

case "${1:-}" in
  setup) setup ;;
  run) shift
       SHA_FULL="$(git -C "$REPO" rev-parse --verify --quiet "${1:-HEAD}^{commit}")" || die "コミットが見つからない: ${1:-HEAD}"
       run "$@" ;;
  *) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
