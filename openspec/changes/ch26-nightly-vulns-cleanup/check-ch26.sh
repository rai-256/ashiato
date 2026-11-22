#!/usr/bin/env bash
# CH26 の確かめの台本（勧告の片付け。Task 1）。
# 使い方: check-ch26.sh lock | toml | vulns | --self-test
#   lock  : Cargo.lock の組の差が「rustix 1.1.5 → 1.1.6 以上の 1.1.x」だけであること
#   toml  : 例外の表（osv-scanner.toml）の既にある行が残り、足した行が正典の理由の形を持つこと
#   vulns : toml に加え、全体の検査と毎晩の検査が片付けた勧告を出さないこと
# 比べる元の版は CH26_BASE（無ければ `git merge-base origin/main HEAD`）、読む木は CH26_ROOT（無ければ repo の根）。
set -u
set -o pipefail

readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO="$(cd "$HERE/../../.." && pwd)"
readonly ROOT="${CH26_ROOT:-$REPO}"
readonly IDS="RUSTSEC-2026-0486 RUSTSEC-2026-0459 RUSTSEC-2026-0543"

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
ok = (removed == [("rustix", "1.1.5")] and len(added) == 1 and added[0][0] == "rustix"
      and re.fullmatch(r"1\.1\.(\d+)", added[0][1]) and int(added[0][1].split(".")[2]) >= 6)
if not ok:
    print("error: 違う組が rustix 1.1.5 → 1.1.6 以上の 1.1.x の 1 つだけではない", file=sys.stderr)
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
  # (c) 既にある行の reason を書き換える
  fresh
  sed -i '0,/reason = "届かない: 携帯の組み立ての道具専用"/s//reason = "書き換えた"/' "$t/r/osv-scanner.toml"
  expect_fail "既にある行の reason を書き換えた表" toml
  # (d) 足した行の reason を 問題ない にする
  fresh
  sed -i 's/^reason = "届かない: 保守終了の知らせで.*/reason = "問題ない"/' "$t/r/osv-scanner.toml"
  expect_fail "足した行の reason を 問題ない にした表" toml
  return $fails
}

case "${1:-}" in
  lock) cmd_lock ;;
  toml) cmd_toml ;;
  vulns) cmd_vulns ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 lock | toml | vulns | --self-test" >&2; exit 64 ;;
esac
