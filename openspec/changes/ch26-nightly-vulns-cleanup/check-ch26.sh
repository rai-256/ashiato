#!/usr/bin/env bash
# CH26 の確かめの台本（勧告の片付け。Task 1〜3）。
# 使い方: check-ch26.sh lock | toml | vulns | pg-dlls | pg-lock | minor-swap | --self-test
#   lock  : Cargo.lock の組の差が「rustix 1.1.5 → 1.1.6 以上の 1.1.x・url 2.5.8 → 2.5.9 以上の 2.5.x・form_urlencoded 1.2.2 → 1.2.3 以上の 1.2.x」だけであること
#   toml  : 例外の表（osv-scanner.toml）の既にある行が残り、足した行が正典の理由の形を持つこと
#   vulns : toml に加え、全体の検査と毎晩の検査が片付けた勧告を出さないこと
#   pg-dlls : pgsql.lock の zip を落として展開し、executables から辿れる同梱の DLL が lock の dlls と一致すること（CH26_PG_CACHE に zip の置き場を渡せる）
#   pg-lock : lock の url が version の zip を指し、小さい版を上げても大きい版・executables・許諾の置き場が元と同じで、遅れの検査が通ること
#   minor-swap : 前の小さい版（postgres:17.6）で作ってサーバの移行を当てた DB の置き場を、新しい小さい版（postgres:17.12）が同じ中身で使い・戻しても使え・17.12 の退避を 17.6 が戻せること（Docker が要る）
# 比べる元の版は CH26_BASE（無ければ `git merge-base origin/main HEAD`）、読む木は CH26_ROOT（無ければ repo の根）。
set -u
set -o pipefail

readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO="$(cd "$HERE/../../.." && pwd)"
readonly ROOT="${CH26_ROOT:-$REPO}"
readonly IDS="RUSTSEC-2026-0486 RUSTSEC-2026-0459 RUSTSEC-2026-0543 RUSTSEC-2026-0592"

base_commit() {
  if [[ -n "${CH26_BASE:-}" ]]; then printf '%s\n' "$CH26_BASE"; return; fi
  git -C "$REPO" merge-base origin/main HEAD
}

# 元の版のファイルを tmp に書き出す
base_file() { # <path> <out>
  local c; c="$(base_commit)" || { echo "error: 比べる元の版を決められない" >&2; return 2; }
  git -C "$REPO" show "$c:$1" > "$2" || { echo "error: $c に $1 が無い" >&2; return 2; }
}

make_tmp() {
  local t; t="$(mktemp -d "${TMPDIR:-/tmp}/ch26.XXXXXX")" || return 2
  printf '%s\n' "$t"
}

cmd_lock() {
  echo "== Scenario: 上げた後の Cargo.lock は、上げた crate のほかの組を動かさない"
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  base_file Cargo.lock "$t/base.lock" || return 2
  python3 -I - "$t/base.lock" "$ROOT/Cargo.lock" <<'PY'
import sys, tomllib, re
def pairs(p):
    with open(p, "rb") as f:
        return {(x["name"], x["version"]) for x in tomllib.load(f)["package"]}
old, new = pairs(sys.argv[1]), pairs(sys.argv[2])
removed, added = sorted(old - new), sorted(new - old)
print("消えた組:", removed)
print("足された組:", added)
# 上げてよい組: (名前, 元の版, 新しい版の系列, 系列の中の下限)。url は RUSTSEC-2026-0592（design D11）、
# form_urlencoded は url 2.5.9 が要る版（P22）
ALLOWED = [("rustix", "1.1.5", "1.1", 6), ("url", "2.5.8", "2.5", 9), ("form_urlencoded", "1.2.2", "1.2", 3)]
def ok_pair(name, old_v, series, low):
    adds = [v for (n, v) in added if n == name]
    return (len(adds) == 1 and re.fullmatch(re.escape(series) + r"\.(\d+)", adds[0])
            and int(adds[0].rsplit(".", 1)[1]) >= low)
ok = (removed == sorted((n, v) for (n, v, _, _) in ALLOWED) and len(added) == len(ALLOWED)
      and all(ok_pair(*a) for a in ALLOWED))
if not ok:
    print("error: 違う組が rustix 1.1.5 → 1.1.6 以上・url 2.5.8 → 2.5.9 以上・form_urlencoded 1.2.2 → 1.2.3 以上の 3 つだけではない", file=sys.stderr)
    sys.exit(1)
PY
}

cmd_toml() {
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  base_file osv-scanner.toml "$t/base.toml" || return 2
  python3 -I - "$t/base.toml" "$ROOT/osv-scanner.toml" <<'PY'
import sys, tomllib
def rows(p):
    with open(p, "rb") as f:
        return tomllib.load(f).get("IgnoredVulns", [])
old, new = rows(sys.argv[1]), rows(sys.argv[2])
bad = 0
print("== Scenario: 例外の表の既にある行は、中身ごと残る")
for r in old:
    key = (r.get("id"), r.get("ignoreUntil"), r.get("reason"))
    if key not in {(n.get("id"), n.get("ignoreUntil"), n.get("reason")) for n in new}:
        print("error: 元の行が無いか変わっている:", r, file=sys.stderr); bad = 1
print("== Scenario: 足した例外の行は、正典の理由の形を持つ")
allowed = {"RUSTSEC-2026-0459", "RUSTSEC-2026-0543", "RUSTSEC-2026-0502"}
oldkeys = {(r.get("id"), r.get("ignoreUntil"), r.get("reason")) for r in old}
added = [n for n in new if (n.get("id"), n.get("ignoreUntil"), n.get("reason")) not in oldkeys]
print("足した行:", [a.get("id") for a in added])
if not {"RUSTSEC-2026-0459", "RUSTSEC-2026-0543"} <= {a.get("id") for a in added}:
    print("error: 0459 と 0543 の行が足されていない", file=sys.stderr); bad = 1
for a in added:
    if a.get("id") not in allowed:
        print("error: 足してはいけない id:", a.get("id"), file=sys.stderr); bad = 1
    # 完了の判定 2: 足した行は `届かない: ` の理由を持つ（`上げる予定: ` は上げる change が無いので事実と違う。design D2・D3）
    if not str(a.get("reason", "")).startswith("届かない: "):
        print("error: reason の形が正典と違う:", a.get("id"), file=sys.stderr); bad = 1
sys.exit(bad)
PY
}

cmd_vulns() {
  cmd_toml || return $?
  echo "== Scenario: 全体の検査と毎晩の検査は、片付けた勧告を出さない"
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  local out rc id
  out="$(CHECK_VULNS_ROOT="$ROOT" "$REPO/tools/check-vulns.sh" 2>&1)"; rc=$?
  printf '%s\n' "$out"
  if [[ $rc -eq 2 ]]; then echo "error: 全体の検査が問い合わせられなかった" >&2; return 1; fi
  for id in $IDS; do
    if grep -qF "$id" <<<"$out"; then echo "error: 全体の検査が $id を出した" >&2; return 1; fi
  done
  out="$(CHECK_VULNS_ROOT="$ROOT" "$REPO/tools/check-vulns.sh" nightly "$t/f" 2>&1)"; rc=$?
  printf '%s\n' "$out"
  if [[ $rc -eq 2 ]]; then echo "error: 毎晩の検査が問い合わせられなかった" >&2; return 1; fi
  for id in $IDS; do
    if grep -qF "$id" <<<"$out"; then echo "error: 毎晩の検査が $id を出した" >&2; return 1; fi
    if [[ -d "$t/f" ]] && grep -rqF "$id" "$t/f"; then echo "error: 毎晩の材料に $id がある" >&2; return 1; fi
  done
}

lock_val() { # <lock> <key>
  awk -F= -v key="$2" '$1 == key { sub(/^[^=]*=/, ""); print; exit }' "$1"
}

cmd_pg_dlls() {
  echo "== Scenario: lock の DLL の一覧は、zip の実行ファイルから辿れる同梱の DLL と一致する"
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  local lock="$ROOT/tools/release/pgsql.lock" z
  z="$(PGSQL_LOCK_FILE="$lock" "$REPO/tools/release/pgsql.sh" fetch "${CH26_PG_CACHE:-$t/cache}")" || return 1
  unzip -q "$z" -d "$t/x" || return 1
  python3 -I - "$t/x" "$(lock_val "$lock" executables)" "$(lock_val "$lock" dlls)" <<'PY'
import os, re, subprocess, sys
root, executables, dlls = sys.argv[1], sys.argv[2], sys.argv[3]
base = os.path.join(root, "pgsql") if os.path.isdir(os.path.join(root, "pgsql")) else root
bindir = os.path.join(base, "bin")
have = {n.lower(): n for n in os.listdir(bindir) if n.lower().endswith(".dll")}
VC = re.compile(r"^(vcruntime140.*|msvcp140.*)\.dll$", re.I)
def imports(path):
    out = subprocess.run(["x86_64-w64-mingw32-objdump", "-p", path], check=True,
                         capture_output=True, text=True).stdout
    return [m.group(1) for m in re.finditer(r"^\s*DLL Name: (\S+)", out, re.M)]
seen, todo = set(), []
for exe in executables.split(","):
    todo += imports(os.path.join(bindir, exe))
while todo:
    n = todo.pop().lower()
    if n in seen or VC.match(n) or n not in have:  # Windows の DLL は zip の bin に無いので数えない
        continue
    seen.add(n)
    todo += imports(os.path.join(bindir, have[n]))
want = {d.lower() for d in dlls.split(",") if d}
print("辿った DLL:", sorted(seen))
print("lock の dlls:", sorted(want))
if seen != want:
    print("error: lock に足りない:", sorted(seen - want), "/ zip から辿れない:", sorted(want - seen), file=sys.stderr)
    sys.exit(1)
PY
}

cmd_pg_lock() {
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  local lock="$ROOT/tools/release/pgsql.lock" version url
  version="$(lock_val "$lock" version)"; url="$(lock_val "$lock" url)"
  base_file tools/release/pgsql.lock "$t/base.lock" || return 2
  echo "== Scenario: lock の url は lock の version の zip を指す"
  # 末尾だけでなく、配布元と道も元の lock の url と同じこと（元の url の版の所だけを今の版に替えたもの。task-2 F1）
  local bv bu
  bv="$(lock_val "$t/base.lock" version)"; bu="$(lock_val "$t/base.lock" url)"
  if [[ -z "$version" || "$url" != *"postgresql-$version-windows-x64-binaries.zip" || -z "$bv" || "$url" != "${bu//"$bv"/"$version"}" ]]; then
    echo "error: url が元の url の版だけを替えた version の zip でない: version=$version url=$url（元: $bu）" >&2; return 1
  fi
  echo "== Scenario: 小さい版を上げても、大きい版と実行ファイルと許諾の置き場は変わらない"
  local k
  if [[ "${version%%.*}" != "$(lock_val "$t/base.lock" version | sed 's/\..*//')" ]]; then
    echo "error: 大きい版が元と違う" >&2; return 1
  fi
  for k in executables license license_destination; do
    if [[ "$(lock_val "$lock" $k)" != "$(lock_val "$t/base.lock" $k)" ]]; then
      echo "error: $k が元と違う" >&2; return 1
    fi
  done
  echo "== Scenario: 同梱の版が最新の minor なら、遅れの検査は通る"
  local bundled="${version%%-*}" major minor
  major="${bundled%%.*}"; minor="${bundled#*.}"
  mkdir -p "$t/edb"
  printf '[{"major":"%s","latestMinor":"%s","relDate":"%s"}]\n' "$major" "$minor" "$(date -u +%F)" > "$t/versions.json"
  : > "$t/edb/postgresql-$version-windows-x64-binaries.zip"
  PGSQL_LOCK_FILE="$lock" PG_VERSIONS_URL="file://$t/versions.json" PG_EDB_URL_BASE="file://$t/edb/postgresql-" \
    "$REPO/tools/check-pg-minor.sh"
}

# ---- minor-swap（Task 3。design D5 の 1）----
readonly PG_OLD="postgres:17.6"
readonly PG_NEW="postgres:17.12"
# initdb の引数はインストーラと同じ。crates/installer/src/db.rs の initdb_args から写した（-U ashiato、
# --locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums。-A と --pwfile は
# 像の入口が POSTGRES_PASSWORD から付ける）
readonly INITDB_ARGS="--locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums"
readonly SW_ADMIN="ashiato"
readonly SW_DB="ashiato"

SW_CONTAINERS=()
SW_VOLUMES=()
sw_cleanup() {
  local c v
  for c in "${SW_CONTAINERS[@]:-}"; do [[ -n "$c" ]] && docker rm -f "$c" >/dev/null 2>&1; done
  for v in "${SW_VOLUMES[@]:-}"; do [[ -n "$v" ]] && docker volume rm -f "$v" >/dev/null 2>&1; done
  [[ -n "${SW_TMP:-}" ]] && rm -rf -- "$SW_TMP"
  return 0
}

# 置き場（名前に乱数を含む volume）で <像> を起こし、空いている port を docker に選ばせる
sw_start() { # <container> <volume> <image>
  # 合言葉は argv（ps に見える）に置かず、docker の環境から -e NAME で渡す（task-3 F1）
  POSTGRES_PASSWORD="$SW_ADMIN_PW" docker run -d --name "$1" -v "$2:/var/lib/postgresql/data" -p 127.0.0.1::5432 \
    -e POSTGRES_USER="$SW_ADMIN" -e POSTGRES_DB="$SW_DB" -e POSTGRES_PASSWORD \
    -e POSTGRES_INITDB_ARGS="$INITDB_ARGS" "$3" >/dev/null || return 1
  local i
  for i in $(seq 1 60); do
    # 入口の一時サーバ（TCP を聞かない）ではなく、本番の起動を待つ: TCP 越しに入れること
    if PGPASSWORD="$SW_ADMIN_PW" docker exec -e PGPASSWORD "$1" psql -X -q -h 127.0.0.1 -U "$SW_ADMIN" -d "$SW_DB" -Atc 'select 1' >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "error: $3 が起きなかった" >&2; docker logs "$1" 2>&1 | tail -20 >&2; return 1
}
sw_port() { docker port "$1" 5432/tcp | head -1 | sed 's/.*://'; }
sw_stop() { docker stop -t 120 "$1" >/dev/null && docker rm "$1" >/dev/null; }

# tools/db-roles.sh と同じ SQL（tools/db-roles.sql）を、同じ環境変数の渡し方（stack.sh の db-roles.sh）で当てる
sw_roles() { # <container>
  ADMIN_PW="$SW_ADMIN_PW" OWNER_PW="$SW_OWNER_PW" APP_PW="$SW_APP_PW" docker exec -i -e ADMIN_PW -e OWNER_PW -e APP_PW "$1" \
    psql -X -q -v ON_ERROR_STOP=1 -v "admin=$SW_ADMIN" -v "dbname=$SW_DB" -U "$SW_ADMIN" -d "$SW_DB" \
    < "$REPO/tools/db-roles.sql" >/dev/null
}

# stack.sh と同じ形: 所有者の接続（DATABASE_OWNER_URL）で `ashiato-server migrate`。使い捨てなので退避しない
sw_migrate() { # <container>
  DATABASE_OWNER_URL="postgres://ashiato_owner:$SW_OWNER_PW@127.0.0.1:$(sw_port "$1")/$SW_DB" \
    MIGRATE_BACKUP=off "$SW_SERVER" migrate
}

sw_sql() { # <container> <sql>
  PGPASSWORD="$SW_ADMIN_PW" docker exec -e PGPASSWORD "$1" psql -X -q -h 127.0.0.1 -U "$SW_ADMIN" -d "$SW_DB" -Atc "$2"
}

# 「中身」: 17.12 の像の中の pg_dump（平文）から、版の行と毎回変わる鍵の行を除いたもの
sw_content() { # <container> <out>
  PGPASSWORD="$SW_ADMIN_PW" docker run --rm --network "container:$1" -e PGPASSWORD "$PG_NEW" \
    pg_dump --no-password -h 127.0.0.1 -U "$SW_ADMIN" -d "$SW_DB" \
    | grep -v -e '^-- Dumped from' -e '^-- Dumped by' -e '^\\restrict' -e '^\\unrestrict' > "$2" || return 1
  [[ -s "$2" ]] || { echo "error: pg_dump の出力が空" >&2; return 1; }
}

sw_same() { # <a> <b> <what>
  if ! diff -u "$1" "$2" >"$SW_TMP/diff.txt"; then
    echo "error: 中身が違う（$3）:" >&2; head -40 "$SW_TMP/diff.txt" >&2; return 1
  fi
  echo "中身は同じ（$3。$(wc -l < "$1") 行）"
}

# 退避の戻し（pg_restore を通る道）の比べだけが使う揃え（design D5、本人の決定 Q-impl-2）。
# 平文の CHECK 制約の行（` CHECK (` を含む行）で、同じ AND の連なりの中の AND の括弧の入れ子を平らにする:
# `((a AND b) AND c)` → `(a AND b AND c)`。AND と OR が混ざる括弧・CHECK の外の行は揃えない。Docker を使わない。
sw_flatten_checks() { # <in> <out>
  python3 -I - "$1" "$2" <<'PY'
import sys

def whole(t):  # t 全体が 1 組の括弧で包まれているか
    if not (t.startswith("(") and t.endswith(")")):
        return False
    depth = 0
    for i, ch in enumerate(t):
        depth += (ch == "(") - (ch == ")")
        if depth == 0 and i < len(t) - 1:
            return False
    return depth == 0

def top_split(t, word):  # 括弧の外の ` word ` で分ける（引用符の中は見ない: 制約の式には出ない）
    parts, depth, cur, i = [], 0, "", 0
    while i < len(t):
        if t[i] == "(":
            depth += 1
        elif t[i] == ")":
            depth -= 1
        if depth == 0 and t.startswith(word, i):
            parts.append(cur); cur = ""; i += len(word); continue
        cur += t[i]; i += 1
    parts.append(cur)
    return parts

def conjuncts(t):
    """t が AND だけの連なり（括弧の入れ子は何重でも）ならその項の一覧、そうでなければ None"""
    while whole(t):
        inner = t[1:-1]
        if len(top_split(inner, " OR ")) > 1:
            return None
        parts = top_split(inner, " AND ")
        if len(parts) > 1:
            out = []
            for part in parts:
                sub = conjuncts(part)
                out += sub if sub is not None else [part]
            return out
        t = inner
    return None

def flat(line):
    key = " CHECK "
    i = line.find(key + "(")
    if i < 0:
        return line
    head, rest = line[: i + len(key)], line[i + len(key):]
    end = rest.rfind(")") + 1  # CHECK の式の閉じ括弧（後ろは NOT VALID; など）
    expr, tail = rest[:end], rest[end:]
    c = conjuncts(expr)
    return head + ("(" + " AND ".join(c) + ")" if c else expr) + tail

with open(sys.argv[1]) as f, open(sys.argv[2], "w") as g:
    for line in f:
        g.write(flat(line.rstrip("\n")) + "\n")
PY
}

sw_same_flat() { # <a> <b> <what>   退避の戻しの比べ（括弧の入れ子を揃えてから）
  sw_flatten_checks "$1" "$SW_TMP/flat-a.sql" && sw_flatten_checks "$2" "$SW_TMP/flat-b.sql" || return 1
  sw_same "$SW_TMP/flat-a.sql" "$SW_TMP/flat-b.sql" "$3（CHECK の括弧の入れ子を平らにして）"
}

cmd_minor_swap() {
  command -v docker >/dev/null || { echo "error: docker が無い" >&2; return 2; }
  SW_TMP="$(make_tmp)" || return 2
  trap sw_cleanup EXIT
  local rnd; rnd="$(od -An -N6 -tx1 /dev/urandom | tr -d ' \n')"
  SW_ADMIN_PW="$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
  SW_OWNER_PW="$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
  SW_APP_PW="$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
  local vol="ch26-swap-$rnd" c_old="ch26-swap-old-$rnd" c_new="ch26-swap-new-$rnd" c_back="ch26-swap-back-$rnd"
  local vol2="ch26-swap2-$rnd" c_rest="ch26-swap-rest-$rnd"
  SW_VOLUMES=("$vol" "$vol2"); SW_CONTAINERS=("$c_old" "$c_new" "$c_back" "$c_rest")
  docker volume create "$vol" >/dev/null && docker volume create "$vol2" >/dev/null || return 2
  docker image inspect "$PG_OLD" >/dev/null 2>&1 || docker pull -q "$PG_OLD" >/dev/null || return 2
  docker image inspect "$PG_NEW" >/dev/null 2>&1 || docker pull -q "$PG_NEW" >/dev/null || return 2

  local td="${CARGO_TARGET_DIR:-target}"; [[ "$td" = /* ]] || td="$REPO/$td"   # cargo の置き場に従う（task-3 F2）
  SW_SERVER="$td/debug/ashiato-server"
  (cd "$REPO" && cargo build -q -p ashiato-server --bin ashiato-server) || { echo "error: ashiato-server を build できない" >&2; return 2; }
  [[ -x "$SW_SERVER" ]] || { echo "error: $SW_SERVER が無い" >&2; return 2; }

  local t="$SW_TMP"
  # 17.6 で置き場を作り、役割 → 移行 → 行（core.source は NOT NULL の列が logical_source・display_name・
  # expected_gap_sec の 3 つで、残りは既定値か移行の trigger が埋める単純な表）
  sw_start "$c_old" "$vol" "$PG_OLD" || return 1
  sw_roles "$c_old" || return 1
  sw_migrate "$c_old" || { echo "error: 17.6 で移行を当てられない" >&2; return 1; }
  sw_sql "$c_old" "INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES ('ch26-swap-a', '17.6 で入れた行', 60)" || return 1
  local n0; n0="$(sw_sql "$c_old" 'SELECT count(*) FROM migration_ledger.applied')"
  [[ "${n0:-0}" -gt 0 ]] || { echo "error: 当てた移行が 0" >&2; return 1; }
  sw_content "$c_old" "$t/old.sql" || return 1
  sw_stop "$c_old" || return 1

  echo "== Scenario: 前の小さい版で移行を当てた置き場を、新しい小さい版が同じ中身で使う"
  sw_start "$c_new" "$vol" "$PG_NEW" || return 1
  sw_content "$c_new" "$t/new.sql" || return 1
  sw_same "$t/old.sql" "$t/new.sql" "17.6 で起きていたときと 17.12 で起こしたとき" || return 1
  [[ "$(sw_sql "$c_new" "SELECT count(*) FROM core.source WHERE logical_source='ch26-swap-a'")" = 1 ]] \
    || { echo "error: 17.6 で入れた行が 17.12 で読めない" >&2; return 1; }

  echo "== Scenario: 新しい小さい版で移行をもう一度当てても、足す移行は無い"
  sw_migrate "$c_new" || { echo "error: 17.12 で移行をもう一度当てたら rc≠0" >&2; return 1; }
  local n1; n1="$(sw_sql "$c_new" 'SELECT count(*) FROM migration_ledger.applied')"
  [[ "$n1" = "$n0" ]] || { echo "error: 当てた移行の数が変わった: $n0 → $n1" >&2; return 1; }
  echo "当てた移行の数は $n1 のまま"

  echo "== Scenario: 新しい小さい版で足した記録を、前の小さい版に戻しても読める"
  sw_sql "$c_new" "INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES ('ch26-swap-b', '17.12 で足した行', 60)" || return 1
  sw_content "$c_new" "$t/new2.sql" || return 1
  # 退避（Scenario 4）は、行を足した後の 17.12 で起きている DB から、サーバの移行の前の退避と同じ形式（--format=custom）で取る
  PGPASSWORD="$SW_ADMIN_PW" docker run --rm --network "container:$c_new" -e PGPASSWORD "$PG_NEW" \
    pg_dump --no-password --format=custom -h 127.0.0.1 -U "$SW_ADMIN" -d "$SW_DB" > "$t/backup.dump" || { echo "error: 17.12 の pg_dump が失敗" >&2; return 1; }
  [[ -s "$t/backup.dump" ]] || { echo "error: 退避が空" >&2; return 1; }
  sw_stop "$c_new" || return 1
  sw_start "$c_back" "$vol" "$PG_OLD" || return 1
  sw_content "$c_back" "$t/back.sql" || return 1
  sw_same "$t/new2.sql" "$t/back.sql" "17.12 で行を足した後と 17.6 へ戻したとき" || return 1
  [[ "$(sw_sql "$c_back" "SELECT count(*) FROM core.source WHERE logical_source IN ('ch26-swap-a','ch26-swap-b')")" = 2 ]] \
    || { echo "error: 足した行が 17.6 で読めない" >&2; return 1; }
  sw_stop "$c_back" || return 1

  echo "== Scenario: 新しい小さい版で取った退避を、前の小さい版が戻せる"
  sw_start "$c_rest" "$vol2" "$PG_OLD" || return 1
  sw_roles "$c_rest" || return 1   # 戻す先に所有者と実行時の役割がある（インストーラの戻しの道と同じ）
  PGPASSWORD="$SW_ADMIN_PW" docker run --rm -i --network "container:$c_rest" -e PGPASSWORD "$PG_OLD" \
    pg_restore --no-password --exit-on-error -h 127.0.0.1 -U "$SW_ADMIN" -d "$SW_DB" < "$t/backup.dump" \
    || { echo "error: 17.6 の pg_restore が rc≠0" >&2; return 1; }
  echo "pg_restore は rc=0"
  sw_content "$c_rest" "$t/restored.sql" || return 1
  sw_same_flat "$t/new2.sql" "$t/restored.sql" "17.12 の退避を取ったときと 17.6 に戻したとき" || return 1
}

self_test() {
  local t; t="$(make_tmp)" || return 2
  trap "rm -rf -- '$t'" RETURN
  local fails=0
  expect_fail() { # <名前> <サブコマンド>
    if CH26_ROOT="$t/r" bash "$HERE/check-ch26.sh" "$2" >/dev/null 2>&1; then
      echo "self-test NG: $1 が落ちなかった" >&2; fails=1
    else
      echo "self-test ok: $1 は落ちる"
    fi
  }
  fresh() { rm -rf "$t/r"; mkdir -p "$t/r"; cp "$ROOT/Cargo.lock" "$ROOT/osv-scanner.toml" "$t/r/"; }
  fresh
  CH26_ROOT="$t/r" bash "$HERE/check-ch26.sh" lock >/dev/null 2>&1 || { echo "self-test NG: 写しのままの lock が通らない" >&2; fails=1; }
  CH26_ROOT="$t/r" bash "$HERE/check-ch26.sh" toml >/dev/null 2>&1 || { echo "self-test NG: 写しのままの toml が通らない" >&2; fails=1; }
  # (a) rustix 1.1.x を 1.1.5 に戻す
  fresh
  python3 -I - "$t/r/Cargo.lock" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
s2 = re.sub(r'(name = "rustix"\nversion = ")1\.1\.\d+(")', r'\g<1>1.1.5\2', s)
assert s != s2
open(p, "w").write(s2)
PY
  expect_fail "rustix を 1.1.5 に戻した lock" lock
  # (b) 別の項目の version を 1 つ書き換える
  fresh
  python3 -I - "$t/r/Cargo.lock" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
s2 = re.sub(r'(name = "serde"\nversion = ")(\d+\.\d+\.)(\d+)(")', lambda m: m[1] + m[2] + str(int(m[3]) + 1) + m[4], s, count=1)
assert s != s2
open(p, "w").write(s2)
PY
  expect_fail "別の項目の version を書き換えた lock" lock
  # (b2) url 2.5.x を 2.5.8 に戻す
  fresh
  python3 -I - "$t/r/Cargo.lock" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
s2 = re.sub(r'(name = "url"\nversion = ")2\.5\.\d+(")', r'\g<1>2.5.8\2', s)
assert s != s2
open(p, "w").write(s2)
PY
  expect_fail "url を 2.5.8 に戻した lock" lock
  # (c) 既にある行の reason を書き換える
  fresh
  sed -i '0,/reason = "届かない: 携帯の組み立ての道具専用"/s//reason = "書き換えた"/' "$t/r/osv-scanner.toml"
  expect_fail "既にある行の reason を書き換えた表" toml
  # (d) 足した行の reason を 問題ない にする
  fresh
  sed -i 's/^reason = "届かない: 保守終了の知らせで.*/reason = "問題ない"/' "$t/r/osv-scanner.toml"
  expect_fail "足した行の reason を 問題ない にした表" toml
  # (d2) 足した行の reason を 上げる予定: にしても落ちる（完了の判定 2 は 届かない: を求める）
  fresh
  sed -i 's/^reason = "届かない: 保守終了の知らせで/reason = "上げる予定: 保守終了の知らせで/' "$t/r/osv-scanner.toml"
  grep -q '^reason = "上げる予定: 保守終了の知らせで' "$t/r/osv-scanner.toml" || { echo "self-test NG: 上げる予定 の写しを作れない" >&2; fails=1; }
  expect_fail "足した行の reason を 上げる予定: にした表" toml
  # (e) pg-lock: 写しの lock の url を別の版の zip にすると落ちる・executables から 1 つ消すと落ちる
  mkdir -p "$t/r/tools/release"
  pglock() { cp "$ROOT/tools/release/pgsql.lock" "$t/r/tools/release/pgsql.lock"; }
  pglock
  CH26_ROOT="$t/r" bash "$HERE/check-ch26.sh" pg-lock >/dev/null 2>&1 || { echo "self-test NG: 写しのままの pgsql.lock が pg-lock を通らない" >&2; fails=1; }
  pglock
  sed -i 's#^url=.*#url=https://get.enterprisedb.com/postgresql/postgresql-17.6-1-windows-x64-binaries.zip#' "$t/r/tools/release/pgsql.lock"
  expect_fail "url を別の版の zip にした lock" pg-lock
  pglock
  sed -i 's#^url=https://get\.enterprisedb\.com/#url=https://example.invalid/#' "$t/r/tools/release/pgsql.lock"
  grep -q '^url=https://example.invalid/' "$t/r/tools/release/pgsql.lock" || { echo "self-test NG: 配布元を替えた写しを作れない" >&2; fails=1; }
  expect_fail "url の配布元を替えた lock（末尾は version の zip のまま）" pg-lock
  pglock
  sed -i '/^executables=/s/,pg_isready\.exe//' "$t/r/tools/release/pgsql.lock"
  expect_fail "executables から 1 つ消した lock" pg-lock
  # 退避の戻しの比べ（Docker を使わない）
  echo "== Scenario: 退避の戻しの比べは、括弧の入れ子のほかの違いを同じとみなさない"
  SW_TMP="$t"
  local A="    ADD CONSTRAINT event_end_values CHECK ((((end_tz_offset_min >= '-1439'::integer) AND (end_tz_offset_min <= 1439)) AND (end_tz_id <> ''::text))) NOT VALID;"
  local B="    ADD CONSTRAINT event_end_values CHECK (((end_tz_offset_min >= '-1439'::integer) AND (end_tz_offset_min <= 1439) AND (end_tz_id <> ''::text))) NOT VALID;"
  flat_eq() { printf '%s\nCREATE TABLE x ();\n' "$1" > "$t/fa"; printf '%s\nCREATE TABLE x ();\n' "$2" > "$t/fb"; sw_same_flat "$t/fa" "$t/fb" self >/dev/null 2>&1; }
  flat_eq "$A" "$B" || { echo "self-test NG: 括弧の入れ子だけの違いを同じと判定しない" >&2; fails=1; }
  flat_eq "$A" "${B/<= 1439/<= 1440}" && { echo "self-test NG: 式の値の違いを同じと判定した" >&2; fails=1; }
  flat_eq "$A" "${B/>= \'-1439\'/> \'-1439\'}" && { echo "self-test NG: 演算子の違いを同じと判定した" >&2; fails=1; }
  flat_eq "ALTER x CHECK (((a > 1) AND (b > 2)) OR (c > 3));" "ALTER x CHECK (((a > 1) AND ((b > 2) OR (c > 3))));" \
    && { echo "self-test NG: AND と OR の組み替えを同じと判定した" >&2; fails=1; }
  printf '%s\nCREATE TABLE x ();\n' "$A" > "$t/fa"; printf '%s\nCREATE TABLE y ();\n' "$B" > "$t/fb"
  sw_same_flat "$t/fa" "$t/fb" self >/dev/null 2>&1 && { echo "self-test NG: CHECK の外の行の違いを同じと判定した" >&2; fails=1; }
  # 本人の決定 Q-impl-2 の片側: 置き場の入れ替え（Scenario 1〜3）の比べは揃えない（review R2）。
  # minor-swap の比べの呼び出しが、順に sw_same・sw_same・sw_same_flat（退避の戻しだけ）であることと、
  # 揃えない比べが括弧の入れ子だけの違いも違うと判定することを見る
  local calls
  calls="$(declare -f cmd_minor_swap | grep -oE '\bsw_same(_flat)?\b' | tr '\n' ' ')"
  [[ "$calls" == "sw_same sw_same sw_same_flat " ]] \
    || { echo "self-test NG: minor-swap の比べが 入れ替え 2 つは揃えない・退避の戻しだけ揃える になっていない: $calls" >&2; fails=1; }
  printf '%s\nCREATE TABLE x ();\n' "$A" > "$t/fa"; printf '%s\nCREATE TABLE x ();\n' "$B" > "$t/fb"
  sw_same "$t/fa" "$t/fb" self >/dev/null 2>&1 && { echo "self-test NG: 揃えない比べが括弧の入れ子だけの違いを同じと判定した" >&2; fails=1; }
  [[ $fails -eq 0 ]] && echo "self-test ok: 括弧の入れ子だけを同じとみなす（退避の戻しだけ。置き場の入れ替えは揃えない）"
  return $fails
}

case "${1:-}" in
  lock) cmd_lock ;;
  toml) cmd_toml ;;
  vulns) cmd_vulns ;;
  pg-dlls) cmd_pg_dlls ;;
  pg-lock) cmd_pg_lock ;;
  minor-swap) cmd_minor_swap ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 lock | toml | vulns | pg-dlls | pg-lock | minor-swap | --self-test" >&2; exit 64 ;;
esac
