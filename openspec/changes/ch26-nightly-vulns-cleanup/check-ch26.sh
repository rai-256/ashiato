#!/usr/bin/env bash
# CH26 の確かめの台本（勧告の片付け。Task 1・2）。
# 使い方: check-ch26.sh lock | toml | vulns | pg-dlls | pg-lock | --self-test
#   lock  : Cargo.lock の組の差が「rustix 1.1.5 → 1.1.6 以上の 1.1.x・url 2.5.8 → 2.5.9 以上の 2.5.x・form_urlencoded 1.2.2 → 1.2.3 以上の 1.2.x」だけであること
#   toml  : 例外の表（osv-scanner.toml）の既にある行が残り、足した行が正典の理由の形を持つこと
#   vulns : toml に加え、全体の検査と毎晩の検査が片付けた勧告を出さないこと
#   pg-dlls : pgsql.lock の zip を落として展開し、executables から辿れる同梱の DLL が lock の dlls と一致すること（CH26_PG_CACHE に zip の置き場を渡せる）
#   pg-lock : lock の url が version の zip を指し、小さい版を上げても大きい版・executables・許諾の置き場が元と同じで、遅れの検査が通ること
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
    if not str(a.get("reason", "")).startswith(("届かない: ", "上げる予定: ")):
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
  echo "== Scenario: lock の url は lock の version の zip を指す"
  if [[ -z "$version" || "$url" != *"postgresql-$version-windows-x64-binaries.zip" ]]; then
    echo "error: url が version の zip で終わらない: version=$version url=$url" >&2; return 1
  fi
  echo "== Scenario: 小さい版を上げても、大きい版と実行ファイルと許諾の置き場は変わらない"
  base_file tools/release/pgsql.lock "$t/base.lock" || return 2
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
  # (e) pg-lock: 写しの lock の url を別の版の zip にすると落ちる・executables から 1 つ消すと落ちる
  mkdir -p "$t/r/tools/release"
  pglock() { cp "$ROOT/tools/release/pgsql.lock" "$t/r/tools/release/pgsql.lock"; }
  pglock
  CH26_ROOT="$t/r" bash "$HERE/check-ch26.sh" pg-lock >/dev/null 2>&1 || { echo "self-test NG: 写しのままの pgsql.lock が pg-lock を通らない" >&2; fails=1; }
  pglock
  sed -i 's#^url=.*#url=https://get.enterprisedb.com/postgresql/postgresql-17.6-1-windows-x64-binaries.zip#' "$t/r/tools/release/pgsql.lock"
  expect_fail "url を別の版の zip にした lock" pg-lock
  pglock
  sed -i '/^executables=/s/,pg_isready\.exe//' "$t/r/tools/release/pgsql.lock"
  expect_fail "executables から 1 つ消した lock" pg-lock
  return $fails
}

case "${1:-}" in
  lock) cmd_lock ;;
  toml) cmd_toml ;;
  vulns) cmd_vulns ;;
  pg-dlls) cmd_pg_dlls ;;
  pg-lock) cmd_pg_lock ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 lock | toml | vulns | pg-dlls | pg-lock | --self-test" >&2; exit 64 ;;
esac
