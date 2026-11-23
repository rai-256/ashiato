#!/usr/bin/env bash
# 同梱の PostgreSQL の lock を変える PR の、更新の試験の古い側を前の lock で組むための判定と確かめ（CH26 design D7・D8）。
# 使い方:
#   pgsql-prev-lock.sh changed <base> <head>
#       base と head の分岐点から head までの差分が tools/release/pgsql.lock を変えたら `true <分岐点>`、変えていなければ `false`
#   pgsql-prev-lock.sh decide <前の lock> <今の lock>
#       中身（行末の CR を落として比べる）が違い大きい版が同じなら `swap`、同じなら `same`（rc=0）。
#       大きい版が違う・version= が読めないなら rc≠0
#   pgsql-prev-lock.sh zip-version <サーバの zip>
#       zip の THIRD-PARTY-NOTICES.txt の `== PostgreSQL` の節の `version: ` の値を出す。節か値が無ければ rc≠0
#   pgsql-prev-lock.sh check-zips <前の lock> <今の lock> <古い側の zip> <新しい側の zip>
#       古い側の版が前の lock の版、新しい側の版が今の lock の版なら `古い側 <版> → 新しい側 <版>` を出して rc=0
#   pgsql-prev-lock.sh check-workflow [<ci.yml>]
#       CI の定義の形を python3 と yaml で確かめる（Linux の chain の job だけ）
#   pgsql-prev-lock.sh --self-test
# changed・decide・zip-version・check-zips は Windows の Git Bash でも動くよう bash・git・unzip・sha256sum・awk/sed だけで書く。
set -u
set -o pipefail

readonly LOCK_PATH='tools/release/pgsql.lock'

die() { echo "pgsql-prev-lock: $*" >&2; exit 2; }

lock_version() { # <lock>: version= の値（行末の CR を落とす）。無ければ空
  sed -n 's/^version=//p' "$1" | tr -d '\r' | head -n 1
}

cmd_changed() {
  [ "$#" -eq 2 ] || die 'changed <base> <head>'
  local mb diff
  mb="$(git merge-base "$1" "$2")" || { echo "pgsql-prev-lock: $1 と $2 の分岐点を取れない" >&2; return 1; }
  diff="$(git diff --name-only "$mb" "$2" -- "$LOCK_PATH")" || return 1
  if [ -n "$diff" ]; then echo "true $mb"; else echo false; fi
}

cmd_decide() {
  [ "$#" -eq 2 ] || die 'decide <前の lock> <今の lock>'
  local pv cv pm cm
  [ -f "$1" ] && [ -f "$2" ] || { echo "pgsql-prev-lock: lock が読めない" >&2; return 1; }
  pv="$(lock_version "$1")"; cv="$(lock_version "$2")"
  for v in "$pv" "$cv"; do
    printf '%s' "$v" | grep -Eq '^[0-9]+\.[0-9]+-[0-9]+$' \
      || { echo "pgsql-prev-lock: version= が <大>.<小>-<n> の形でない: '$v'" >&2; return 1; }
  done
  pm="${pv%%.*}"; cm="${cv%%.*}"
  if [ "$pm" != "$cm" ]; then
    echo "pgsql-prev-lock: 大きい版が違う: 前の lock $pm（$pv）・今の lock $cm（$cv）。大きい版を上げる道は大きい版を上げる Story が扱う" >&2
    echo "$pm $cm"
    return 1
  fi
  if [ "$(tr -d '\r' <"$1")" = "$(tr -d '\r' <"$2")" ]; then echo same; else echo swap; fi
}

cmd_zip_version() {
  [ "$#" -eq 1 ] || die 'zip-version <サーバの zip>'
  local v
  [ -f "$1" ] || { echo "pgsql-prev-lock: zip が無い: $1" >&2; return 1; }
  v="$(unzip -p "$1" THIRD-PARTY-NOTICES.txt 2>/dev/null | tr -d '\r' \
    | awk '/^== PostgreSQL[[:space:]]*$/ {f=1; next} /^== / {f=0} f && /^version: / {sub(/^version: /, ""); print; exit}')" || true
  [ -n "$v" ] || { echo "pgsql-prev-lock: $1 の許諾の文に PostgreSQL の版が無い" >&2; return 1; }
  printf '%s\n' "$v"
}

cmd_check_zips() {
  [ "$#" -eq 4 ] || die 'check-zips <前の lock> <今の lock> <古い側の zip> <新しい側の zip>'
  local pv cv ov nv
  pv="$(lock_version "$1")"; cv="$(lock_version "$2")"
  [ -n "$pv" ] && [ -n "$cv" ] || { echo "pgsql-prev-lock: lock の版が読めない" >&2; return 1; }
  ov="$(cmd_zip_version "$3")" || return 1
  nv="$(cmd_zip_version "$4")" || return 1
  if [ "$ov" = "$pv" ] && [ "$nv" = "$cv" ]; then
    echo "古い側 $ov → 新しい側 $nv（$ov → $nv）"
    return 0
  fi
  echo "pgsql-prev-lock: 版が食い違う: 古い側 $ov（前の lock は $pv）・新しい側 $nv（今の lock は $cv）" >&2
  return 1
}

# --- check-workflow の本体（python3 と yaml）。python3 - check <ci.yml> / python3 - mutate <名前> <入力> <出力> ---
CHECK_PY='
import re, sys
import yaml

def load(path):
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)

def run_of(s):
    return str(s.get("run", ""))

def terms(cond):
    return [t.strip() for t in str(cond).split("&&")]

PGSQL_TRUE = "needs.changes.outputs.pgsql == \x27true\x27"
OLD_GUARD = "[ \"$name\" = old ]"
CP = "cp \"$PREV_LOCK\" tools/release/pgsql.lock"
CO = "git checkout -- tools/release/pgsql.lock"

def check(path):
    doc = load(path)
    fails = []

    def scenario(name, fn):
        print("== Scenario: %s" % name)
        try:
            msgs = fn()
        except Exception as e:  # 形が想定と違う（step が無いなど）も落ちとして数える
            msgs = ["読めない: %r" % (e,)]
        for m in msgs:
            print("NG: " + m)
        if not msgs:
            print("ok")
        fails.extend(msgs)

    ch = doc["jobs"]["changes"]
    pw = doc["jobs"]["package-windows"]["steps"]

    def idx(pred, what, steps=pw):
        r = [i for i, s in enumerate(steps) if pred(s)]
        if len(r) != 1:
            raise ValueError("%s が %d 個ある（1 個のはず）" % (what, len(r)))
        return r[0]

    def prev_step():
        return idx(lambda s: "decide" in run_of(s) and "pgsql-prev-lock.sh" in run_of(s), "前の lock を取る step")

    def key_step():
        return idx(lambda s: "WINDOWS_BUILD_KEY_TOOLS" in run_of(s), "鍵の step")

    def zip_step():
        return idx(lambda s: "--test-build" in run_of(s), "試験の zip を組む step")

    def s_take():
        m = []
        # changes: PR の run でだけ changed を呼び、どの出口でも pgsql を書く
        out = ch.get("outputs", {})
        for k in ("pgsql", "pgsql_from"):
            if not re.fullmatch(r"\$\{\{ steps\.\w+\.outputs\.%s \}\}" % k, str(out.get(k, ""))):
                m.append("changes の outputs に %s が無い" % k)
        fs = [s for s in ch["steps"] if "pgsql-prev-lock.sh changed" in run_of(s)]
        if len(fs) != 1:
            return m + ["changes の job に changed を呼ぶ step が %d 個ある（1 個のはず）" % len(fs)]
        r = run_of(fs[0])
        c = r.index("pgsql-prev-lock.sh changed")
        pr = re.search(r"if \[ \"\$\{?EVENT\}?\" = pull_request \]", r)
        if not pr or pr.start() > c:
            m.append("changed の呼び出しが PR の run の分岐の中にない")
        if "github.event_name" not in str(fs[0].get("env", {}).get("EVENT", "")):
            m.append("changed を呼ぶ step の EVENT が run の種類（github.event_name）を持たない")
        early = r.find("全部走らせる")
        if early != -1 and early < c:
            m.append("base が引けないときの早い出口が、pgsql を書く前にある")
        if "pgsql=false" not in r.replace(" ", "") and "pgsql_out false" not in r:
            m.append("PR でない run の pgsql=false が無い")
        if "pgsql_out true" not in r:
            m.append("台本が落ちたときの pgsql=true が無い")
        # package-windows: 前の lock を取る step
        p = prev_step()
        k = key_step()
        ps = pw[p]
        if str(ps.get("if", "")).strip() != PGSQL_TRUE:
            m.append("前の lock を取る step の if: が %s だけでない: %r" % (PGSQL_TRUE, ps.get("if")))
        if p > k:
            m.append("前の lock を取る step が鍵の step より後にある")
        pr_ = run_of(ps)
        if "git fetch" not in pr_ or "tools/release/pgsql.lock" not in pr_:
            m.append("前の lock を取る step が分岐点の lock を fetch して取り出していない")
        if "swap" not in pr_:
            m.append("前の lock を取る step が decide の swap を見ていない")
        if "pgsql_from" not in str(ps.get("env", {})):
            m.append("前の lock を取る step が pgsql_from を受け取らない")
        # 試験の zip を組む step: old を組む間だけ lock を置き換え、戻す
        z = pw[zip_step()]
        zr = run_of(z)
        lines = zr.splitlines()
        cps = [l for l in lines if CP in l]
        cos = [l for l in lines if CO in l and "Cargo" not in l]
        if len(cps) != 1 or OLD_GUARD not in cps[0]:
            m.append("前の lock への置き換えが old を組むときだけになっていない")
        if len(cos) != 1 or OLD_GUARD not in cos[0]:
            m.append("置き換えを戻す手順が old のときだけの 1 行として無い")
        pk = zr.find("package.sh windows")
        if not (zr.find(CP) != -1 and zr.find(CP) < pk < zr.find(CO)):
            m.append("置き換え → 組む → 戻す の順になっていない")
        if "steps.%s.outputs.path" % prev_id() not in str(z.get("env", {}).get("PREV_LOCK", "")):
            m.append("試験の zip を組む step が前の lock の出力を PREV_LOCK に受け取らない")
        return m

    def prev_id():
        return pw[prev_step()].get("id", "?")

    def s_check_places():
        m = []
        zi = zip_step()
        z = pw[zi]
        zr = run_of(z)
        first_up = min(i for i, s in enumerate(pw) if str(s.get("uses", "")).startswith("actions/upload-artifact@"))
        save = idx(lambda s: str(s.get("uses", "")).startswith("actions/cache/save@"), "save")
        zc = zr.rfind("check-zips")
        if zc == -1 or "package-windows-test-old" not in zr[zc:] or "package-windows-test-new" not in zr[zc:]:
            m.append("試験の zip を組む step の中に、old と new の zip への check-zips が無い")
        elif zc < zr.rfind("done"):
            m.append("組み立ての step の check-zips が loop（組み立て）の後にない")
        if zi > first_up:
            m.append("試験の zip を組む step が upload の後にある")
        if "cache-hit" not in str(z.get("if", "")):
            m.append("組み立ての step が cache-hit != true の中にない")
        after = [i for i, s in enumerate(pw) if i > save and "check-zips" in run_of(s)]
        if len(after) != 1:
            m.append("save の step の後に check-zips の step が %d 個ある（1 個のはず）" % len(after))
        else:
            a = pw[after[0]]
            c = str(a.get("if", ""))
            if "cache-hit" in c or "||" in c or terms(c) != [PGSQL_TRUE]:
                m.append("save の後の check-zips の if: が %s だけでない: %r" % (PGSQL_TRUE, c))
            ar = run_of(a)
            if "package-windows-test-old" not in ar or "package-windows-test-new" not in ar:
                m.append("save の後の check-zips が old と new の zip を渡していない")
        return m

    def s_key():
        m = []
        k = pw[key_step()]
        r = run_of(k)
        t = re.search(r"WINDOWS_BUILD_KEY_TOOLS=\"([^\"]*)\"", r)
        if not t or not t.group(1).rstrip().endswith("|prev-pgsql=$PREV_PGSQL"):
            m.append("鍵の道具の版の文字列の末尾が |prev-pgsql=$PREV_PGSQL でない")
        e = str(k.get("env", {}).get("PREV_PGSQL", ""))
        if "steps.%s.outputs.sha256" % prev_id() not in e or "none" not in e:
            m.append("鍵の step が前の lock の指紋（無ければ none）を PREV_PGSQL に受け取らない: %r" % e)
        return m

    scenario("CI の定義は、lock を変えた PR でだけ前の lock を取り、古い側だけを置き換えて組んで戻す", s_take)
    scenario("CI の定義は、試験の zip の版を、保存の前と使い回した run の両方で確かめる", s_check_places)
    scenario("鍵を出す手順は、古い側を組んだ前の lock の指紋を鍵に入れる", s_key)
    sys.exit(1 if fails else 0)

def mutate(name, src, dst):
    doc = load(src)
    pw = doc["jobs"]["package-windows"]["steps"]
    def find(pred):
        return [i for i, s in enumerate(pw) if pred(s)][0]
    p = find(lambda s: "decide" in run_of(s) and "pgsql-prev-lock.sh" in run_of(s))
    k = find(lambda s: "WINDOWS_BUILD_KEY_TOOLS" in run_of(s))
    z = find(lambda s: "--test-build" in run_of(s))
    sv = find(lambda s: str(s.get("uses", "")).startswith("actions/cache/save@"))
    after = [i for i, s in enumerate(pw) if i > sv and "check-zips" in run_of(s)][0]
    if name == "move-prev-after-key":
        s = pw.pop(p); pw.insert(k, s)           # p < k なので pop 後の k の位置に差すと鍵の後ろ
    elif name == "prev-always":
        del pw[p]["if"]
    elif name == "replace-all":
        pw[z]["run"] = pw[z]["run"].replace(OLD_GUARD, "[ -n \"$name\" ]")
    elif name == "no-restore":
        pw[z]["run"] = "\n".join(l for l in pw[z]["run"].splitlines() if not (CO in l and "Cargo" not in l)) + "\n"
    elif name == "no-check-before-save":
        pw[z]["run"] = pw[z]["run"].replace("check-zips", "version-skip")
    elif name == "check-only-on-miss":
        pw[after]["if"] = PGSQL_TRUE + " && steps.reuse.outputs.cache-hit != \x27true\x27"
    elif name == "no-prev-key":
        pw[k]["run"] = pw[k]["run"].replace("|prev-pgsql=$PREV_PGSQL", "")
    else:
        raise SystemExit("unknown mutation " + name)
    if yaml.safe_dump(doc) == yaml.safe_dump(load(src)):
        raise SystemExit("mutation %s changed nothing" % name)
    with open(dst, "w", encoding="utf-8") as f:
        yaml.safe_dump(doc, f, sort_keys=False, width=100000, allow_unicode=True)

if sys.argv[1] == "check":
    check(sys.argv[2])
elif sys.argv[1] == "mutate":
    mutate(sys.argv[2], sys.argv[3], sys.argv[4])
'

default_ci() { printf '%s\n' "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.github/workflows/ci.yml"; }

cmd_check_workflow() {
  local ci="${1:-$(default_ci)}"
  [ -f "$ci" ] || { echo "pgsql-prev-lock: $ci が無い" >&2; return 2; }
  python3 -c 'import yaml' 2>/dev/null || { echo "pgsql-prev-lock: python3 と yaml が要る" >&2; return 2; }
  python3 -I -c "$CHECK_PY" check "$ci"
}

# --- 自己試験 ---
mkzip() { # <out zip> <THIRD-PARTY-NOTICES.txt の中身>: 中身は引数 2 をそのまま書く
  local d; d="$(mktemp -d "$work/zip.XXXXXX")"
  if [ -n "${2+x}" ]; then printf '%s' "$2" >"$d/THIRD-PARTY-NOTICES.txt"; else echo x >"$d/other.txt"; fi
  (cd "$d" && zip -q -r "$1" .)
}

notices() { # <版>: build.sh の windows_notices と同じ形の許諾の文
  printf 'THIRD-PARTY NOTICES (ashiato-server for Windows)\n\n== PostgreSQL\nversion: %s\nlocation: pgsql\\\n\nterms\n\n== PostgreSQL bundled DLLs\nicuuc67.dll\n' "$1"
}

mklock() { # <out> <版> [改行 lf|crlf]
  local nl=$'\n'; [ "${3:-lf}" = crlf ] && nl=$'\r\n'
  printf '# lock%surl=https://example.invalid/postgresql-%s-windows-x64-binaries.zip%sversion=%s%ssha256=%s%s' \
    "$nl" "$2" "$nl" "$2" "$nl" "$(printf '%s' "$2" | sha256sum | cut -d' ' -f1)" "$nl" >"$1"
}

self_test() {
  local script rc
  script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  work="$(mktemp -d "${TMPDIR:-/tmp}/pgprev.XXXXXX")"
  trap "rm -rf $(printf '%q' "$work")" EXIT
  local g="$work/repo" out
  command -v zip >/dev/null 2>&1 || { echo "NG: 自己試験は zip を使う"; return 1; }

  echo "== Scenario: PR の差分が lock を変えたら、変えたと判定して分岐点を示す"
  mkdir -p "$g/tools/release"
  git -C "$g" init -q -b main || return 1
  git -C "$g" config user.email t@example.invalid; git -C "$g" config user.name t
  git -C "$g" config commit.gpgsign false
  mklock "$g/$LOCK_PATH" 17.6-1; echo a >"$g/other"
  git -C "$g" add -A; git -C "$g" commit -q -m base
  git -C "$g" checkout -q -b pr
  local fork; fork="$(git -C "$g" rev-parse HEAD)"
  mklock "$g/$LOCK_PATH" 17.12-1
  git -C "$g" commit -q -am bump
  out="$(cd "$g" && "$script" changed main pr)" || { echo "NG: changed が落ちた"; return 1; }
  [ "$out" = "true $fork" ] || { echo "NG: '$out'（true $fork のはず）"; return 1; }
  echo "ok: $out"

  echo "== Scenario: base の側だけが lock を変えた古い枝は、変えたと判定しない"
  git -C "$g" checkout -q main
  git -C "$g" checkout -q -b old-branch "$fork"
  echo b >"$g/other"; git -C "$g" commit -q -am "head の側は lock を変えない"
  git -C "$g" checkout -q main
  mklock "$g/$LOCK_PATH" 17.12-1; git -C "$g" commit -q -am "base の側だけが lock を変える"
  # base の先端と head の 2 点では lock が違う（これを「変えた」と読まないこと）
  git -C "$g" diff --quiet main old-branch -- "$LOCK_PATH" && { echo "NG: 前提（2 点では lock が違う）が崩れている"; return 1; }
  out="$(cd "$g" && "$script" changed main old-branch)" || { echo "NG: changed が落ちた"; return 1; }
  [ "$out" = false ] || { echo "NG: '$out'（false のはず）"; return 1; }
  echo "ok: $out"

  mklock "$work/p176.lock" 17.6-1; mklock "$work/c1712.lock" 17.12-1
  echo "== Scenario: 中身が違い大きい版が同じ前の lock なら、古い側を前の lock で組むと判定する"
  out="$("$script" decide "$work/p176.lock" "$work/c1712.lock")" || { echo "NG: decide が落ちた"; return 1; }
  [ "$out" = swap ] || { echo "NG: '$out'（swap のはず）"; return 1; }
  echo "ok: $out"

  echo "== Scenario: 改行だけが違う 2 つの lock は、同じ中身と判定する"
  mklock "$work/lf.lock" 17.12-1 lf; mklock "$work/crlf.lock" 17.12-1 crlf
  cmp -s "$work/lf.lock" "$work/crlf.lock" && { echo "NG: 前提（バイト列が違う）が崩れている"; return 1; }
  out="$("$script" decide "$work/lf.lock" "$work/crlf.lock")" || { echo "NG: decide が落ちた"; return 1; }
  [ "$out" = same ] || { echo "NG: '$out'（same のはず）"; return 1; }
  echo "ok: $out"

  echo "== Scenario: 前の lock と大きい版が違えば、古い側を組まずに落ちる"
  mklock "$work/p164.lock" 16.4-1
  rc=0; out="$("$script" decide "$work/p164.lock" "$work/c1712.lock" 2>&1)" || rc=$?
  [ "$rc" -ne 0 ] && printf '%s' "$out" | grep -q 16 && printf '%s' "$out" | grep -q 17 \
    || { echo "NG: rc=$rc out='$out'"; return 1; }
  echo "ok: rc=$rc"

  echo "== Scenario: 組んだ zip の同梱の版を、許諾の文から読む"
  mkzip "$work/s176.zip" "$(notices 17.6-1)"
  out="$("$script" zip-version "$work/s176.zip")" || { echo "NG: zip-version が落ちた"; return 1; }
  [ "$out" = 17.6-1 ] || { echo "NG: '$out'"; return 1; }
  echo "ok: $out"

  echo "== Scenario: 許諾の文に PostgreSQL の版が無い zip では、版を読めずに落ちる"
  mkzip "$work/none.zip" $'THIRD-PARTY NOTICES\n\n== Something else\nversion: 1.0\n'
  rc=0; "$script" zip-version "$work/none.zip" >/dev/null 2>&1 || rc=$?
  [ "$rc" -ne 0 ] || { echo "NG: 落ちなかった"; return 1; }
  echo "ok: rc=$rc"

  echo "== Scenario: 古い側と新しい側の zip が前と今の lock の版なら、確かめは通って版の組を出す"
  mkzip "$work/s1712.zip" "$(notices 17.12-1)"
  out="$("$script" check-zips "$work/p176.lock" "$work/c1712.lock" "$work/s176.zip" "$work/s1712.zip")" || { echo "NG: check-zips が落ちた"; return 1; }
  printf '%s' "$out" | grep -q '17.6-1 → 17.12-1' || { echo "NG: '$out'"; return 1; }
  echo "ok: $out"

  echo "== Scenario: 古い側の zip が前の lock の版でなければ、確かめは落ちる"
  rc=0; "$script" check-zips "$work/p176.lock" "$work/c1712.lock" "$work/s1712.zip" "$work/s1712.zip" >/dev/null 2>&1 || rc=$?
  [ "$rc" -ne 0 ] || { echo "NG: 落ちなかった"; return 1; }
  echo "ok: rc=$rc"

  # 形の確かめ（python3 と yaml があるときだけ。無ければ飛ばしたと出す）
  local ci m; ci="$(default_ci)"
  if ! python3 -c 'import yaml' 2>/dev/null; then
    echo "-- check-workflow の自己試験は python3 か yaml が無いので飛ばした"
    echo "pgsql-prev-lock: 自己試験 ok（check-workflow は飛ばした）"
    return 0
  fi
  cmd_check_workflow "$ci" || { echo "NG: 今の ci.yml が check-workflow を通らない"; return 1; }
  echo "== Scenario: CI の定義の形が 1 つでも崩れると、形の確かめは落ちる"
  for m in move-prev-after-key prev-always replace-all no-restore no-check-before-save check-only-on-miss no-prev-key; do
    python3 -I -c "$CHECK_PY" mutate "$m" "$ci" "$work/mut-$m.yml" || { echo "NG: 崩した写し ($m) を作れない"; return 1; }
    rc=0; cmd_check_workflow "$work/mut-$m.yml" >/dev/null 2>&1 || rc=$?
    [ "$rc" -ne 0 ] || { echo "NG: 崩した写し ($m) でも check-workflow が通った"; return 1; }
  done
  echo "ok: 崩した写し 7 通りは全部落ちる"
  echo "pgsql-prev-lock: 自己試験 ok"
}

main() {
  local sub="${1:-}"
  [ "$#" -gt 0 ] && shift
  case "$sub" in
    changed) cmd_changed "$@" ;;
    decide) cmd_decide "$@" ;;
    zip-version) cmd_zip_version "$@" ;;
    check-zips) cmd_check_zips "$@" ;;
    check-workflow) cmd_check_workflow "$@" ;;
    --self-test) self_test ;;
    *) sed -n '2,17p' "${BASH_SOURCE[0]}" >&2; return 2 ;;
  esac
}

main "$@"
