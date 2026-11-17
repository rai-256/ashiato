#!/usr/bin/env bash
# Windows の配布物の組み立ての鍵を 1 行で出す（CH17 design D1）。
#   windows-build-key.sh [<commit>]   既定は HEAD。鍵（sha256 の 16 進 64 桁）を標準出力へ
#   windows-build-key.sh --self-test  使い捨ての git の木で鍵の Scenario を確かめる（python3 と yaml があれば --check-workflow の自己試験も）
#   windows-build-key.sh --check-workflow [<ci.yml>]  CI の定義（既定 .github/workflows/ci.yml）の package-windows の使い回しの形を確かめる
#                                                    Linux の手元と merge gate で走らせる（Windows の runner では走らせない）
#
# 鍵 = sha256（ls-tree の行のうち除くパスを落としたもの ＋ 道具の版の行）。
# 道具の版の行は環境変数 WINDOWS_BUILD_KEY_TOOLS の文字列（CI は rustc -V・node -v・ImageOS・ImageVersion を並べて渡す）。
# 鍵を出せない（git の木でない・commit が引けない）ときは、何も出さずに 0 でない終了コードで終わる。
# Windows の Git Bash でも動くよう、sha256sum と git と bash の組み込みだけを使う。
set -euo pipefail

# 除くパス（bash の glob。`*` は `/` も越える）。理由: これらは組み立て（build.sh・package.sh など）が読まない。
# 試験の台本 3 つ（install-check.sh だけが conpty-run.ps1 を読む）と、changes.sh code が「コードでない」とするもの。
# 足すときは、組み立てがそれを読まないことを確かめる。Cargo.toml と LICENSE は外さない（design D1）。
EXCLUDE_PATHS=(
  'tools/release/install-check.sh'
  'tools/release/update-check-install.sh'
  'tools/release/conpty-run.ps1'
  'docs/*'
  'openspec/*'
  '*.md'
)

# --check-workflow の本体（python3 と yaml）。使い方: python3 - check <ci.yml> / python3 - mutate <名前> <入力> <出力>
CHECK_PY='
import json, re, sys
import yaml

HIT = "cache-hit"

def load(path):
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)

def paths_of(step):
    return [l.strip() for l in str(step.get("with", {}).get("path", "")).splitlines() if l.strip()]

def uses_of(step):
    return str(step.get("uses", ""))

def is_cache(step):
    return uses_of(step).startswith(("actions/cache@", "actions/cache/"))

def is_up(step):
    return uses_of(step).startswith("actions/upload-artifact@")

def check(path):
    doc = load(path)
    steps = doc["jobs"]["package-windows"]["steps"]
    fails = []

    def scenario(name, fn):
        print("== Scenario: " + name)
        try:
            msgs = fn()
        except Exception as e:  # 形が想定と違う（step が無いなど）も落ちとして数える
            msgs = ["読めない: %r" % (e,)]
        if msgs:
            for m in msgs:
                print("NG: " + m)
            fails.extend(msgs)
        else:
            print("ok")

    def find(pred):
        return [i for i, s in enumerate(steps) if pred(s)]

    def one(pred, what):
        idx = find(pred)
        if len(idx) != 1:
            raise ValueError("%s が %d 個ある（1 個のはず）" % (what, len(idx)))
        return idx[0]

    def refs():
        k = one(lambda s: "WINDOWS_BUILD_KEY_TOOLS" in str(s.get("run", "")), "鍵の step")
        r = one(lambda s: uses_of(s).startswith("actions/cache/restore@"), "restore")
        return k, r

    def not_hit(rid):
        return "steps.%s.outputs.%s != \x27true\x27" % (rid, HIT)

    def s_restore():
        m = []
        k, r = refs()
        ks, rs = steps[k], steps[r]
        if r < k:
            m.append("restore が鍵の step より前にある")
        if "restore-keys" in rs.get("with", {}):
            m.append("restore が restore-keys を持つ（前方一致で引く）")
        mm = re.fullmatch(r"windows-package-\$\{\{ steps\.%s\.outputs\.(\w+) \}\}" % re.escape(ks.get("id", "?")), str(rs["with"].get("key", "")))
        if not mm:
            m.append("restore の鍵が windows-package- と鍵の step の出力だけから成らない: %r" % rs["with"].get("key"))
            return m
        c = str(rs.get("if", ""))
        if "pull_request" not in c:
            m.append("restore の if: が pull_request を条件に持たない")
        if "steps.%s.outputs.%s != \x27\x27" % (ks["id"], mm.group(1)) not in c:
            m.append("restore の if: が鍵が空でないことを条件に持たない")
        return m

    def s_skip():
        m = []
        k, r = refs()
        rid = steps[r].get("id", "?")
        ch = find(lambda s: "ch14_" in str(s.get("run", "")))
        pk = find(lambda s: "package.sh windows" in str(s.get("run", "")))
        if not ch:
            m.append("ch14_ の試験の step が無い")
        if len(pk) != 2:
            m.append("package.sh windows の step が %d 個ある（2 個のはず）" % len(pk))
        for i in ch + pk:
            if not_hit(rid) not in str(steps[i].get("if", "")):
                m.append("step %d の if: が restore の cache-hit でないことを条件に持たない" % i)
        ups = {steps[i]["with"]["name"]: steps[i] for i in find(is_up)}
        for n in ("package-windows-release", "package-windows-test"):
            if n not in ups:
                m.append("upload-artifact %s が無い" % n)
            elif "if" in ups[n]:
                m.append("upload-artifact %s が if: で飛ばされる" % n)
        return m

    def s_paths():
        m = []
        k, r = refs()
        sv = find(lambda s: uses_of(s).startswith("actions/cache/save@"))
        ups = find(is_up)
        if not ups or len(sv) != 1:
            return ["upload-artifact が無いか save が 1 個でない"]
        rp, sp_ = set(paths_of(steps[r])), set(paths_of(steps[sv[0]]))
        for i in ups:
            for p in paths_of(steps[i]):
                if p not in rp:
                    m.append("upload-artifact の path %s が restore の path に無い" % p)
                if p not in sp_:
                    m.append("upload-artifact の path %s が save の path に無い" % p)
        return m

    def s_asis():
        m = []
        k, r = refs()
        rid = steps[r].get("id", "?")
        ups = find(is_up)
        if not ups:
            return ["upload-artifact が無い"]
        for i in range(r + 1, ups[0]):
            s = steps[i]
            if is_cache(s) and "run" not in s:
                continue
            if not_hit(rid) not in str(s.get("if", "")):
                m.append("restore と upload-artifact の間の step %d が if: で飛ばされない（引いた置き場に書き得る）" % i)
        up_paths = set(p for i in ups for p in paths_of(steps[i]))
        if up_paths != set(paths_of(steps[r])):
            m.append("upload-artifact の path の集まりが restore の path と同じでない")
        return m

    def s_save():
        m = []
        k, r = refs()
        rid = steps[r].get("id", "?")
        sv = one(lambda s: uses_of(s).startswith("actions/cache/save@"), "save")
        ss = steps[sv]
        c = str(ss.get("if", ""))
        if not_hit(rid) not in c:
            m.append("save の if: が cache-hit でないことを条件に持たない")
        if not re.search(r"steps\.%s\.outputs\.\w+ != \x27\x27" % re.escape(steps[k].get("id", "?")), c):
            m.append("save の if: が鍵が空でないことを条件に持たない")
        if "success()" not in c:
            m.append("save の if: が success() を条件に持たない")
        if ss["with"].get("key") != steps[r]["with"].get("key"):
            m.append("save の鍵が restore の鍵と違う")
        if set(paths_of(ss)) != set(paths_of(steps[r])):
            m.append("save の path が restore の path と違う")
        if any(sv < i for i in find(is_up)):
            m.append("save が upload-artifact より前にある")
        return m

    def s_tools():
        m = []
        k, r = refs()
        t = json.dumps(steps[k])
        for w in ("WINDOWS_BUILD_KEY_TOOLS", "rustc -V", "node -v", "ImageOS", "ImageVersion", "github.sha", "ci.yml"):
            if w not in t:
                m.append("鍵の step が %s を持たない" % w)
        return m

    def s_rust():
        m = []
        k, r = refs()
        rid = steps[r].get("id", "?")
        i = one(lambda s: is_cache(s) and "target" in paths_of(s), "Rust の cache")
        s = steps[i]
        if not_hit(rid) not in str(s.get("if", "")):
            m.append("Rust の cache の if: が restore の cache-hit でないことを条件に持たない")
        for p in ("~/.cargo/registry", "~/.cargo/git", "target"):
            if p not in paths_of(s):
                m.append("Rust の cache の path に %s が無い" % p)
        key = str(s["with"].get("key", ""))
        if "hashFiles(\x27Cargo.lock\x27)" not in key:
            m.append("Rust の cache の鍵が hashFiles(Cargo.lock) を含まない")
        mm = re.search(r"steps\.(\w+)\.outputs\.", key)
        rv = [x for x in steps if mm and x.get("id") == mm.group(1)]
        if not rv or "rustc -V" not in str(rv[0].get("run", "")):
            m.append("Rust の cache の鍵が rustc -V の出力を含まない")
        rk = str(s["with"].get("restore-keys", "")).strip()
        if not rk or "hashFiles" in rk or not key.startswith(rk) or not mm or mm.group(0) not in rk:
            m.append("Rust の cache の restore-keys が Rust の版までの前方一致でない: %r" % rk)
        return m

    def s_install():
        m = []
        job = doc["jobs"]["install-windows"]["steps"]
        for lst in ("install-check.sh --list", "update-check-install.sh --list"):
            hit = [s for s in job if lst in str(s.get("run", ""))]
            if len(hit) != 1:
                m.append("%s の step が %d 個ある（1 個のはず）" % (lst, len(hit)))
                continue
            run = str(hit[0]["run"])
            if "comm -23" not in run or "test ! -s" not in run:
                m.append("%s の step が comm -23 と test ! -s の突き合わせを持たない" % lst)
            if "if" in hit[0]:
                m.append("%s の step が if: を持つ" % lst)
        return m

    scenario("前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く", s_restore)
    scenario("引けたときだけ組み立てを飛ばし、今と同じ名前で上げる", s_skip)
    scenario("上げる配布物は全部、保存と引きの置き場に入る", s_paths)
    scenario("使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる", s_asis)
    scenario("組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する", s_save)
    scenario("鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる", s_tools)
    scenario("ビルドするときは、Rust の依存のビルドを戻す", s_rust)
    scenario("使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる", s_install)
    return 1 if fails else 0

def mutate(name, src, dst):
    doc = load(src)
    steps = doc["jobs"]["package-windows"]["steps"]
    ix = lambda pred: [i for i, s in enumerate(steps) if pred(s)]
    r = ix(lambda s: uses_of(s).startswith("actions/cache/restore@"))[0]
    sv = ix(lambda s: uses_of(s).startswith("actions/cache/save@"))[0]
    ups = ix(is_up)
    strip_key = lambda c: re.sub(r"\s*&&\s*steps\.\w+\.outputs\.\w+ != \x27\x27", "", c)
    if name == "a":
        steps[r]["with"]["restore-keys"] = "windows-package-"
    elif name == "b":
        del steps[ix(lambda s: "package.sh windows" in str(s.get("run", "")))[0]]["if"]
    elif name == "c":
        job = doc["jobs"]["install-windows"]["steps"]
        job.remove([s for s in job if "install-check.sh --list" in str(s.get("run", ""))][0])
    elif name == "d":
        steps[r]["if"] = steps[r]["if"].replace("github.event_name == \x27pull_request\x27 && ", "")
    elif name == "e":
        steps[sv]["if"] = steps[sv]["if"].replace(" && success()", "")
    elif name == "f":
        steps[r]["if"] = strip_key(steps[r]["if"])
        steps[sv]["if"] = strip_key(steps[sv]["if"])
    elif name == "g":
        k = ix(lambda s: "WINDOWS_BUILD_KEY_TOOLS" in str(s.get("run", "")))[0]
        steps[k]["run"] = steps[k]["run"].replace("ImageVersion", "ImageVer")
    elif name == "h":
        del steps[ix(lambda s: is_cache(s) and "target" in paths_of(s))[0]]
    elif name == "i":
        steps.insert(ups[-1] + 1, {"uses": steps[ups[-1]]["uses"], "with": {"name": "extra", "path": "${{ runner.temp }}/extra"}})
    elif name == "j":
        steps.insert(r + 1, {"shell": "bash", "run": "echo x > \"$RUNNER_TEMP/package-windows/x\""})
    elif name == "k":
        other = "${{ runner.temp }}/package-windows-other"
        steps[ups[-1]]["with"]["path"] = other
        steps[sv]["with"]["path"] = str(steps[sv]["with"]["path"]).rstrip("\n") + "\n" + other + "\n"
    else:
        raise SystemExit("unknown mutation " + name)
    with open(dst, "w", encoding="utf-8") as f:
        yaml.safe_dump(doc, f, allow_unicode=True, sort_keys=False)

if sys.argv[1] == "check":
    sys.exit(check(sys.argv[2]))
mutate(sys.argv[2], sys.argv[3], sys.argv[4])
'

key() {
  local commit="${1:-HEAD}" root line path pat skip
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "windows-build-key: git の木でない" >&2; return 2; }
  git -C "$root" rev-parse --verify --quiet "${commit}^{commit}" >/dev/null \
    || { echo "windows-build-key: commit を引けない: $commit" >&2; return 2; }
  {
    printf 'tools:%s\n' "${WINDOWS_BUILD_KEY_TOOLS:-}"
    git -C "$root" ls-tree -r --full-tree "$commit" | while IFS= read -r line; do
      path="${line#*$'\t'}"
      skip=0
      for pat in "${EXCLUDE_PATHS[@]}"; do
        # shellcheck disable=SC2053
        if [[ "$path" == $pat ]]; then skip=1; break; fi
      done
      [ "$skip" = 1 ] || printf '%s\n' "$line"
    done
  } | sha256sum | cut -d' ' -f1
}

default_ci() {
  echo "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.github/workflows/ci.yml"
}

check_workflow() {
  local ci="${1:-$(default_ci)}"
  [ -f "$ci" ] || { echo "windows-build-key: $ci が無い" >&2; return 2; }
  python3 -c 'import yaml' 2>/dev/null || { echo "windows-build-key: python3 と yaml が要る" >&2; return 2; }
  python3 -I -c "$CHECK_PY" check "$ci"
}

# --check-workflow が落ちることの確かめ（python3 と yaml があるときだけ。無ければ飛ばしたと出す）
self_test_workflow() {
  local ci="$1" work="$2" m rc
  if ! python3 -c 'import yaml' 2>/dev/null; then
    echo "-- --check-workflow の自己試験は python3 か yaml が無いので飛ばした"
    return 0
  fi
  echo "-- --check-workflow の自己試験"
  check_workflow "$ci" >/dev/null || { check_workflow "$ci"; echo "NG: 今の ci.yml が --check-workflow を通らない"; return 1; }
  for m in a b c d e f g h i j k; do
    python3 -I -c "$CHECK_PY" mutate "$m" "$ci" "$work/mut-$m.yml" || { echo "NG: 変えた版 ($m) を作れない"; return 1; }
    rc=0
    check_workflow "$work/mut-$m.yml" >/dev/null 2>&1 || rc=$?
    [ "$rc" -ne 0 ] || { echo "NG: 変えた版 ($m) でも --check-workflow が通った"; return 1; }
  done
  echo "ok: 変えた版 11 通りは全部落ちる"
}

self_test() {
  local script work t keyfile base k alt
  script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  work="$(mktemp -d "${TMPDIR:-/tmp}/wbk.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  t="$work/tree"
  mkdir -p "$t"
  export WINDOWS_BUILD_KEY_TOOLS="rustc 1.90.0|node v24.0.0|ImageOS=win25|ImageVersion=20260101.1"
  local git_t=(git -C "$t" -c user.name=t -c user.email=t@example.invalid)
  "${git_t[@]}" init -q
  local f
  for f in tools/release/install-check.sh tools/release/update-check-install.sh tools/release/conpty-run.ps1 \
           docs/a.txt docs/y.md openspec/z.txt README.md \
           crates/server/src/lib.rs migrations/ORDER web/src/App.tsx tools/release/build.sh tools/release/pgsql.lock \
           tools/licenses/a.txt LICENSE Cargo.toml collector-android/app.kt Cargo.lock .github/workflows/ci.yml; do
    mkdir -p "$t/$(dirname "$f")"
    echo "base $f" >"$t/$f"
  done
  "${git_t[@]}" add -A
  "${git_t[@]}" commit -q -m base
  base="$("${git_t[@]}" rev-parse HEAD)"

  # 変えた木の鍵を出す。$1: 台本  残り: "パス=内容" の列。base の上に commit して、base へ戻る
  key_with() {
    local s="$1" kv p out
    shift
    "${git_t[@]}" checkout -q --detach "$base"
    for kv in "$@"; do
      p="${kv%%=*}"
      mkdir -p "$t/$(dirname "$p")"
      printf '%s\n' "${kv#*=}" >"$t/$p"
    done
    "${git_t[@]}" add -A
    "${git_t[@]}" commit -q -m variant
    out="$(cd "$t" && "$s" HEAD)"
    "${git_t[@]}" checkout -q --detach "$base"
    "${git_t[@]}" clean -qfd
    printf '%s' "$out"
  }
  local base_key
  base_key="$(cd "$t" && "$script" "$base")"
  [[ "$base_key" =~ ^[0-9a-f]{64}$ ]] || { echo "NG: 鍵が sha256 の形でない: $base_key"; return 1; }

  echo "== Scenario: 試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる"
  k="$(key_with "$script" \
    'tools/release/install-check.sh=changed' 'tools/release/update-check-install.sh=changed' \
    'tools/release/conpty-run.ps1=changed' 'docs/x.md=new doc')"
  [ "$k" = "$base_key" ] || { echo "NG: 台本と文書の違いで鍵が変わった"; return 1; }
  # 除くことが効いている確かめ: 除くパスの列挙から install-check.sh を外した台本では、同じ変更で鍵が変わる
  alt="$work/windows-build-key-noexclude.sh"
  grep -v "^  'tools/release/install-check.sh'$" "$script" >"$alt"
  chmod +x "$alt"
  k="$(key_with "$alt" 'tools/release/install-check.sh=changed')"
  [ "$k" != "$(cd "$t" && "$alt" "$base")" ] || { echo "NG: 除くパスを外しても鍵が変わらない（除くことが効いていない）"; return 1; }
  echo "ok"

  echo "== Scenario: 組み立てに入るファイルが 1 つでも違えば、鍵が変わる"
  for f in crates/server/src/lib.rs migrations/ORDER web/src/App.tsx tools/release/build.sh tools/release/pgsql.lock \
           tools/licenses/a.txt LICENSE Cargo.toml collector-android/app.kt Cargo.lock .github/workflows/ci.yml; do
    k="$(key_with "$script" "$f=changed")"
    [ "$k" != "$base_key" ] || { echo "NG: $f を変えても鍵が変わらない"; return 1; }
  done
  # repo の根でない cwd から走らせても同じ鍵になる
  k="$(mkdir -p "$t/crates" && cd "$t/crates" && "$script" "$base")"
  [ "$k" = "$base_key" ] || { echo "NG: cwd が違うと鍵が変わる"; return 1; }
  echo "ok"

  echo "== Scenario: 名指ししていないファイルが増えれば、鍵が変わる"
  k="$(key_with "$script" 'tools/release/new-script.sh=new')"
  [ "$k" != "$base_key" ] || { echo "NG: 新しい台本を足しても鍵が変わらない"; return 1; }
  echo "ok"

  echo "== Scenario: 道具の版が違えば、鍵が変わる"
  k="$(cd "$t" && WINDOWS_BUILD_KEY_TOOLS="rustc 1.91.0|node v24.0.0|ImageOS=win25|ImageVersion=20260101.1" "$script" "$base")"
  [ "$k" != "$base_key" ] || { echo "NG: Rust の版を変えても鍵が変わらない"; return 1; }
  echo "ok"

  echo "== Scenario: 鍵を出せない木では、鍵を出さずに落ちる"
  local plain="$work/plain" out rc=0
  mkdir -p "$plain"
  out="$(cd "$plain" && GIT_CEILING_DIRECTORIES="$work" "$script" HEAD 2>/dev/null)" || rc=$?
  [ "$rc" -ne 0 ] || { echo "NG: git の木でないのに rc=0"; return 1; }
  [ -z "$out" ] || { echo "NG: git の木でないのに出力がある: $out"; return 1; }
  rc=0
  out="$(cd "$t" && "$script" no-such-commit 2>/dev/null)" || rc=$?
  [ "$rc" -ne 0 ] && [ -z "$out" ] || { echo "NG: 引けない commit で鍵を出した"; return 1; }
  echo "ok"

  self_test_workflow "$(default_ci)" "$work"
}

case "${1:-}" in
  --self-test) self_test ;;
  --check-workflow) shift; check_workflow "${1:-}" ;;
  *) key "${1:-HEAD}" ;;
esac
