#!/usr/bin/env bash
# DB の版の食い違いで止まるときの、開発と確認の道具の振る舞いを確かめる（ST48 / design D13・D14）。
#
#   ./tools/check-db-version-stop.sh
#
# 見るのは: 確認バッチの run.sh が回ごとの compose の project を持つ / 別の project で立てると前の DB が残る /
# 知らない版が当たった DB では作り直さずに手順を示して止まる / STACK_RESET=1 の作り直しでは全部が当たる。
# 試験の project（st48-a-<乱数>・st48-b-<乱数>）は終わりに `down -v` で消す。この worktree の DB は終わりに起こし直す。
#
# 環境変数: SERVER_BIN  サーバの実体（既定 target/debug/ashiato-server。無ければ debug で build する。CI は smoke が作ったものを渡す）
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -f .env ]; then set -a; . ./.env; set +a; fi
. tools/ports.sh

# 呼び出し元の環境を引きずらない（HARNESS_VERIFY=1 は STACK_RESET=1 と同じに扱われる）
unset HARNESS_VERIFY STACK_RESET COMPOSE_PROJECT_NAME MIGRATE_BACKUP
work="$(mktemp -d "${TMPDIR:-/tmp}/check-db-version-stop.XXXXXX")"
bk="$work/backups"; mkdir -p "$bk"
bin="${SERVER_BIN:-target/debug/ashiato-server}"
if [ ! -x "$bin" ]; then
  echo "== サーバを build（$bin が無い）"
  cargo build -q -p ashiato-server --bin ashiato-server
  bin="target/debug/ashiato-server"
fi
id="$RANDOM$RANDOM"
A="st48-a-$id"; B="st48-b-$id"

# 始めに、この port を持っている容器（この worktree の DB）の名前を覚える。stack.sh が別の project の容器を止めるので、終わりに起こし直す
mapfile -t mine < <(docker ps --filter "publish=$ASHIATO_DB_PORT" --format '{{.Names}}')
cleanup() {
  local p
  for p in "$A" "$B"; do COMPOSE_PROJECT_NAME="$p" docker compose down -v >/dev/null 2>&1 || true; done
  for p in "${mine[@]}"; do docker start "$p" >/dev/null 2>&1 || true; done
  rm -rf "$work"
}
trap cleanup EXIT

fail=0
ok() { echo "  OK $*"; }
ng() { echo "  NG $*" >&2; fail=1; }
# 別の session で走らせる（stack.sh は失敗の後始末で自分のプロセスグループを落とすので、この台本を巻き込ませない）
stk() { local p="$1"; shift; env COMPOSE_PROJECT_NAME="$p" MIGRATE_BACKUP_DIR="$bk" SERVER_BIN="$bin" "$@" setsid -w ./tools/stack.sh up --check-only; }
dbq() { COMPOSE_PROJECT_NAME="$1" docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato -c "$2"; }
events() { dbq "$1" "SELECT count(*) FROM core.event WHERE logical_source = 'stop-check'"; }

echo "Scenario: 確認バッチの起動は、回ごとの project に立つ"
# verify-prep.sh の「project 名を決める行」と「run.sh を書く部分」だけを取り出し、2 つの違う tag で書かせる
snippet="$work/write-run.sh"
{ grep -E '^project=' tools/verify-prep.sh
  sed -n '/^cat > "\$out\/run.sh" <<EOF/,/^chmod +x "\$out\/run.sh"/p' tools/verify-prep.sh; } > "$snippet"
proj_of() {  # proj_of <tag> → run.sh の COMPOSE_PROJECT_NAME
  local tag="$1" out="$work/out-$1"; mkdir -p "$out"
  ( tag="$tag" out="$out"; . "$snippet" )
  sed -n 's/^export COMPOSE_PROJECT_NAME="\(.*\)"$/\1/p' "$out/run.sh"
}
p1="$(proj_of Batch-01)"; p2="$(proj_of batch-02)"
[ -n "$p1" ] && [ -n "$p2" ] || ng "run.sh に COMPOSE_PROJECT_NAME の行が無い"
[[ $p1 == *batch-01* && $p2 == *batch-02* ]] || ng "project 名が tag を含まない（$p1 / $p2）"
[ "$p1" != "$p2" ] || ng "違う tag で project 名が同じ（$p1）"
[ "$fail" = 0 ] && ok "tag ごとに違う project（$p1 / $p2）"

echo "Scenario: 別の project で立てると、前の DB は残る"
stk "$A" >/dev/null || ng "A で立てられない"
dbq "$A" "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
          VALUES ('stop-check','版で止まる確認用',21600,'none') ON CONFLICT DO NOTHING" >/dev/null
dbq "$A" "INSERT INTO core.event (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
          VALUES ('44444444-4444-4444-8444-444444444444','00000000-0000-0000-0000-000000000000','stop-check','collected',
                  '2026-09-08T02:00:00Z',540,'Asia/Tokyo',1,'stop-check-hash','{}','{}')" >/dev/null
[ "$(events "$A")" = 1 ] || ng "A に 1 件置けていない"
running() { [ -n "$(COMPOSE_PROJECT_NAME="$1" docker compose ps --status running -q db)" ]; }
COMPOSE_PROJECT_NAME="$A" setsid -w ./tools/stack.sh down >/dev/null
running "$A" && ng "stack.sh down で A が止まっていない" || ok "stack.sh down で A が止まった（容器は残る）"
if stk "$B" >/dev/null; then ok "B で立てられた"; else ng "A を止めたあと B で立てられない"; fi
COMPOSE_PROJECT_NAME="$B" setsid -w ./tools/stack.sh down >/dev/null
COMPOSE_PROJECT_NAME="$A" docker compose up -d --wait db >/dev/null
# A を動かしたまま B を立てると、stack.sh が A を止める（消さない）
if stk "$B" >/dev/null; then ok "A が動いたまま B で立てられた"; else ng "A が動いたままだと B で立てられない"; fi
running "$A" && ng "B を立てても A が動いたまま" || ok "B を立てると A は自動で止まった"
COMPOSE_PROJECT_NAME="$B" setsid -w ./tools/stack.sh down >/dev/null
COMPOSE_PROJECT_NAME="$A" docker compose up -d --wait db >/dev/null
[ "$(events "$A")" = 1 ] && ok "A を起こし直すと 1 件ある" || ng "A の 1 件が残っていない"

echo "Scenario: 版の食い違いで止まると、作り直さずに手順を示す"
dbq "$A" "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit, before_ledger)
          VALUES ('299912312359_unknown_version','x','x','x',false)" >/dev/null
rc=0; out="$(stk "$A" 2>&1)" || rc=$?
[ "$rc" != 0 ] && ok "知らない版が当たった DB で止まった（rc=$rc）" || ng "止まらなかった"
[[ $out == *STACK_RESET=1* && $out == *COMPOSE_PROJECT_NAME* ]] && ok "手順（STACK_RESET=1 と COMPOSE_PROJECT_NAME）を出した" || ng "手順が出力に無い: $out"
[ "$(events "$A")" = 1 ] && [ "$(dbq "$A" "SELECT count(*) FROM migration_ledger.applied WHERE name = '299912312359_unknown_version'")" = 1 ] \
  && ok "1 件と知らない版の行が残っている" || ng "作り直されたか、行が消えた"

echo "Scenario: 作り直した DB には、今までどおり全部が当たる"
before="$(find "$bk" -type f | sort)"
stk "$A" env STACK_RESET=1 >/dev/null && ok "STACK_RESET=1 で rc=0" || ng "STACK_RESET=1 で立てられない"
want="$(grep -vE '^[[:space:]]*(#|$)' migrations/ORDER | sed 's/[[:space:]]//g' | sort)"
got="$(dbq "$A" "SELECT name FROM migration_ledger.applied" | sort)"
[ "$got" = "$want" ] && ok "記録の名前の集合が ORDER と一致" || ng "記録の名前が ORDER と違う"
[ "$(find "$bk" -type f | sort)" = "$before" ] && ok "退避の置き場に新しいファイルが無い" || ng "作り直しで退避が増えた"

[ "$fail" = 0 ] || exit 1
echo "== OK"
