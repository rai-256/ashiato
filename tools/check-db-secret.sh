#!/usr/bin/env bash
# 配布物に DB の合言葉の固定値が無いことを確かめる（ST28 / design D19）。
#
#   tools/check-db-secret.sh                      # 全部（(a) 追跡ファイル (b) 雛形の拒否）
#   tools/check-db-secret.sh --only compose,testdb # (a) の見る場所を絞る。(b) は走らせない
#
# (a) DB を立てる設定・試験・台本に、`POSTGRES_PASSWORD: <字面>` と `postgres://<役割>:<字面>@` が無い。
#     `${` で始まるもの（環境からの参照）は通す。`.env.example` は見ない（雛形は change-me-）。
# (b) 雛形の値だけの環境ファイルでは tools/db-roles.sh が失敗し、DB の役割の一覧が変わらない。
#     DB が立っていなければ自分で立てる（docker compose up -d --wait db。tools/smoke.sh が落としていることがある）。
#
# 場所の名前: compose / testdb / tools / workflows
set -euo pipefail
cd "$(dirname "$0")/.."

only=""
if [ "${1:-}" = "--only" ]; then only="${2:?--only に場所の名前を渡す}"; fi

files_of() {
  case "$1" in
    compose)   git ls-files 'docker-compose.yml' ;;
    testdb)    git ls-files 'crates/*.rs' ;;
    tools)     git ls-files 'tools/*.sh' | grep -vxF 'tools/check-db-secret.sh' || true ;;
    workflows) git ls-files '.github/workflows/*.yml' ;;
    *) echo "error: 場所の名前が違う: $1" >&2; exit 2 ;;
  esac
}
places=(compose testdb tools workflows)
[ -z "$only" ] || IFS=',' read -r -a places <<<"$only"

# 字面の値 = `$` で始まらず、空でもない。（YAML の `KEY: 値` / URL の `://役割:値@`）
lit_env='(^|[[:space:],])POSTGRES_PASSWORD[:=][[:space:]]*["'"'"']?[^$"'"'"'[:space:]#]'
lit_url='postgres(ql)?://[^:/@[:space:]]+:[^$@[:space:]][^@[:space:]]*@'

found=0
for place in "${places[@]}"; do
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    if hits="$(grep -nE "$lit_env|$lit_url" "$f")"; then
      # 行番号と場所だけ出す（値は出さない）
      echo "$hits" | cut -d: -f1 | sed "s|^|NG $f:|"
      found=1
    fi
  done < <(files_of "$place")
done
if [ "$found" -ne 0 ]; then echo "error: DB の合言葉が字面で書かれている" >&2; exit 1; fi
# Scenario: 配布物に DB の合言葉の固定値が無い
echo "配布物に DB の合言葉の固定値が無い（${places[*]}）"

[ -z "$only" ] || exit 0

roles() { docker compose exec -T db psql -X -qtA -U "${POSTGRES_USER:-ashiato}" -d "${POSTGRES_DB:-ashiato}" \
  -c "SELECT rolname, rolsuper, rolcreatedb, rolcreaterole, rolcanlogin FROM pg_roles ORDER BY 1"; }
# 直前に tools/smoke.sh が DB を落としていることがある（末尾の down -v）。立っていなければ立てる
docker compose up -d --wait db >/dev/null 2>&1 || true
before="$(roles)" || { echo "error: DB に問えない（docker compose up -d --wait db を先に）" >&2; exit 1; }
if ENV_FILE=tools/fixtures/db-roles/placeholder.env tools/db-roles.sh >/tmp/db-roles-placeholder.log 2>&1; then
  echo "error: 雛形の合言葉で役割を作る手順が成功した" >&2; exit 1
fi
grep -q 'POSTGRES_PASSWORD' /tmp/db-roles-placeholder.log \
  || { echo "error: 雛形のままの項目の名前が出ていない" >&2; exit 1; }
[ "$(roles)" = "$before" ] || { echo "error: 失敗したのに DB の役割の一覧が変わった" >&2; exit 1; }
# Scenario: 雛形の合言葉のままでは役割を作らない
echo "雛形の合言葉のままでは役割を作らない（exit 1・役割の一覧は不変）"
