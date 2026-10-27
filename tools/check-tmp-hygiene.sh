#!/usr/bin/env bash
# 試験の一時フォルダ・一時ファイルを**手で作る書き方**を止める（CH03。design D3〜D5）。
#
#   tools/check-tmp-hygiene.sh              # 追跡ファイル全部を見る（CI の chain job・pre-commit）
#   tools/check-tmp-hygiene.sh --self-test  # 違反を差し込んだ断片で落ち、正しい書き方で通ること
#
# **なぜ要るか**: 試験が `std::env::temp_dir()` や `Files.createTempDirectory` で作った一時フォルダは、試験が
# 落ちる・panic する・殺されると消されずに残る。CH03 の実測（2026-10-23）で、消し忘れが 16G の `/tmp` を満杯にし、
# 並走する Story の検証が「空きなし」で落ちた。**消えることが言語の仕組みで決まる書き方**だけを許し、手で作る書き方を止める。
#
#   範囲: `git ls-files` の `crates/**/*.rs`（製品のコードも含む。`#[cfg(test)]` は読まない）と
#         `collector-android/app/src/test/**/*.kt`。携帯の `app/src/main` と `app/src/androidTest` は見ない。
#   Rust   : `temp_dir()`・`"/tmp`・`.keep()`・`.into_path()`・`.persist(` を落とす。
#            代わりに `tempfile::TempDir`（`tempfile::Builder::new().prefix("…").tempdir()`）を試験の最後まで持つ。
#   Kotlin : `createTempDirectory`・`createTempFile`・`createTempDir`・`java.io.tmpdir`・`"/tmp` と、
#            同じ行か 1 行上に `@Rule` / `@get:Rule` の無い `TemporaryFolder(` を落とす。
#            代わりに `@get:Rule val tmp = TemporaryFolder()` と `tmp.newFolder()`。
#
# どうしても要る箇所（一時フォルダを読むだけ、など）は、同じ行か 1 行上に `// tmp-ok: <理由>` を書く。
# 理由の無い印と、2 行以上離れた印は数えない。コメントの中の文字列も拾う（誤検知は印で逃がす）。
#
# rc: 0 = 違反なし / 1 = 違反あり（ファイルと行番号を挙げる）/ 2 = 知らない引数（使い方を出す）
set -euo pipefail
cd "$(dirname "$0")/.."

PYSRC=$(cat <<'PY'
import os, re, subprocess, sys

MARK = re.compile(r"//\s*tmp-ok:\s*\S")
RULE = re.compile(r"@(?:get:)?Rule\b")

RUST = [r"temp_dir\(\)", r'"/tmp', r"\.keep\(\)", r"\.into_path\(\)", r"\.persist\("]
KOTLIN = [r"createTempDirectory", r"createTempFile", r"createTempDir", r"java\.io\.tmpdir", r'"/tmp']
RUST_PAT = re.compile("|".join(RUST))
KOTLIN_PAT = re.compile("|".join(KOTLIN))
FOLDER = re.compile(r"TemporaryFolder\(")

RUST_HINT = ("一時フォルダは tempfile::TempDir（tempfile::Builder::new().prefix(\"…\").tempdir()）を試験の最後まで持つ。"
             "読むだけなど手で要るなら同じ行か 1 行上に `// tmp-ok: <理由>`")
KOTLIN_HINT = ("一時フォルダは `@get:Rule val tmp = TemporaryFolder()` と `tmp.newFolder()`（@get:Rule は同じ行か 1 行上）。"
               "手で要るなら同じ行か 1 行上に `// tmp-ok: <理由>`")


def near(lines, ln, pat):
    """ln（1 始まり）の行か、その 1 行上に pat があるか。"""
    return any(pat.search(lines[k]) for k in range(max(0, ln - 2), ln))


def check(path, text, pats, hint, folder):
    lines = text.splitlines()
    out = []
    for i, line in enumerate(lines, 1):
        hit = [m.group(0) for p in pats for m in [p.search(line)] if m]
        if folder and FOLDER.search(line) and not near(lines, i, RULE):
            hit.append("TemporaryFolder( に規則が無い")
        if hit and not near(lines, i, MARK):
            out.append(f"{path}:{i}: 一時フォルダを手で作る書き方 `{hit[0]}`（{hint}）")
    return out


def listing(root):
    if root is None:
        return subprocess.run(["git", "ls-files"], capture_output=True, text=True, check=True).stdout.split("\n")
    return [os.path.relpath(os.path.join(d, f), root) for d, _, fs in os.walk(root) for f in fs]


def scan(root=None):
    out = []
    for p in sorted(x for x in listing(root) if x):
        full = os.path.join(root, p) if root else p
        if not os.path.isfile(full):  # 作業木で消して stage していない追跡ファイル（消したものに違反は無い）
            continue
        if p.startswith("crates/") and p.endswith(".rs"):
            out += check(p, open(full, encoding="utf-8").read(), [RUST_PAT], RUST_HINT, False)
        elif p.startswith("collector-android/app/src/test/") and p.endswith(".kt"):
            out += check(p, open(full, encoding="utf-8").read(), [KOTLIN_PAT], KOTLIN_HINT, True)
    return out


root = None
if "--root" in sys.argv:
    root = sys.argv[sys.argv.index("--root") + 1]
found = scan(root)
if found:
    print("試験の一時フォルダを手で作る書き方がある（CH03。落ちた試験の一時フォルダは残り、/tmp を埋める）:")
    print("\n".join("  " + f for f in found))
    sys.exit(1)
print("check-tmp-hygiene: OK")
PY
)

case "$#:${1:-}" in
  0:) exec python3 -c "$PYSRC" ;;
  1:--self-test) ;;
  *) echo "使い方: $0 [--self-test]（知らない引数: $*）" >&2; exit 2 ;;
esac

# ---- --self-test ----
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fails=0
RS=crates/x/src/a.rs
KT=collector-android/app/src/test/kotlin/A.kt

# case <期待 rc> <相対パス> <内容>: 新しい木に 1 ファイル置いて検査する。出力は $work/out
n=0
case_run() {
  local want=$1 rel=$2 body=$3 dir
  n=$((n + 1)); dir=$work/t$n
  mkdir -p "$dir/$(dirname "$rel")"
  printf '%s\n' "$body" > "$dir/$rel"
  local rc=0
  python3 -c "$PYSRC" --root "$dir" > "$work/out" 2>&1 || rc=$?
  if [ "$rc" != "$want" ]; then
    echo "  FAIL: rc=$rc（期待 $want）: ${body//$'\n'/ ⏎ }"; sed 's/^/    /' "$work/out"
    fails=$((fails + 1)); return
  fi
  if [ "$want" = 1 ] && ! grep -q "$rel:" "$work/out"; then
    echo "  FAIL: ファイルと行番号を挙げない: ${body//$'\n'/ ⏎ }"; fails=$((fails + 1))
  fi
}
says() {   # says <文言>: 直前の出力にその文言がある
  grep -qF -- "$1" "$work/out" || { echo "  FAIL: 出力に「$1」が無い"; fails=$((fails + 1)); }
}

echo "Scenario: Rust の試験で一時フォルダを手で作ると、検査が落ち、TempDir を示す"
case_run 1 $RS 'fn t() { let d = std::env::temp_dir().join("x"); }'
says "$RS:1:"; says "tempfile::TempDir"; says "tmp-ok:"

echo "Scenario: 書き方を変えても、検査は素通りさせない"
for s in 'let d = env::temp_dir();' 'let d = "/tmp/x";' 'let p = d.keep();' 'let p = d.into_path();' 'f.persist(p);'; do
  case_run 1 $RS "fn t() { $s }"
done

echo "Scenario: 製品のコードの一時フォルダも、検査が落とす"
case_run 1 $RS $'fn product() { let d = std::env::temp_dir(); }\n#[cfg(test)]\nmod tests {}'

echo "Scenario: 携帯の単体試験で一時フォルダを手で作ると、検査が落ち、TemporaryFolder を示す"
for s in 'val d = Files.createTempDirectory("x")' 'val f = File.createTempFile("a", "b")' 'val d = createTempDir()' \
         'val d = System.getProperty("java.io.tmpdir")' 'val d = File("/tmp/x")' 'val t = TemporaryFolder()' \
         $'@get:Rule\n\nval t = TemporaryFolder()'; do
  case_run 1 $KT "$s"
  says "@get:Rule"; says "TemporaryFolder"; says "tmp-ok:"
done

echo "Scenario: 規則の付いた TemporaryFolder と tempfile は、検査を通る"
case_run 0 $RS 'fn t() { let d = tempfile::TempDir::new().unwrap(); }'
case_run 0 $KT '@get:Rule val tmp = TemporaryFolder()'
case_run 0 $KT $'@get:Rule\nval tmp = TemporaryFolder()'

echo "Scenario: 理由を書いた tmp-ok の印が同じ行か 1 行上にあれば、検査を通る"
case_run 0 $RS 'let d = std::env::temp_dir(); // tmp-ok: 一時フォルダを読むだけ'
case_run 0 $RS $'// tmp-ok: 一時フォルダを読むだけ\nlet d = std::env::temp_dir();'

echo "Scenario: 理由の無い tmp-ok の印と、2 行以上離れた印は、検査を通らない"
case_run 1 $RS 'let d = std::env::temp_dir(); // tmp-ok:'
case_run 1 $RS $'// tmp-ok: 一時フォルダを読むだけ\n\nlet d = std::env::temp_dir();'

echo "Scenario: 携帯の main と androidTest は、検査の外"
body='val d = Files.createTempDirectory("x")'
case_run 0 collector-android/app/src/main/kotlin/A.kt "$body"
case_run 0 collector-android/app/src/androidTest/kotlin/A.kt "$body"

echo "Scenario: 検査は CI の chain job と commit 前に走る"
chain=$(awk '/^  chain:/{f=1;next} f&&/^  [A-Za-z0-9_-]+:/{f=0} f' .github/workflows/ci.yml)
for c in './tools/check-tmp-hygiene.sh --self-test' './tools/check-tmp-hygiene.sh'; do
  grep -qxF -- "      - run: $c" <<<"$chain" || { echo "  FAIL: ci.yml の chain job に「$c」が無い"; fails=$((fails + 1)); }
done
grep -qF 'tools/check-tmp-hygiene.sh >/dev/null || { tools/check-tmp-hygiene.sh || true; exit 1; }' tools/pre-commit.sh \
  || { echo "  FAIL: tools/pre-commit.sh が検査を呼び、落ちれば exit 1 しない"; fails=$((fails + 1)); }

echo "Scenario: 知らない引数は本番の検査として走らせず、使い方を出して rc=2"
rc=0; ./tools/check-tmp-hygiene.sh --selftest > "$work/out" 2>&1 || rc=$?
[ "$rc" = 2 ] && grep -qF "使い方" "$work/out" || { echo "  FAIL: --selftest が rc=$rc（期待 2 と使い方）"; fails=$((fails + 1)); }

echo "Scenario: 作業木で消して stage していない追跡ファイルは、読まずに飛ばす"
g=$work/git; mkdir -p "$g/crates/x/src"
printf 'fn t() {}\n' > "$g/crates/x/src/gone.rs"
git -C "$g" init -q && git -C "$g" add -A && rm "$g/crates/x/src/gone.rs"
rc=0; (cd "$g" && python3 -c "$PYSRC") > "$work/out" 2>&1 || rc=$?
[ "$rc" = 0 ] || { echo "  FAIL: 消した追跡ファイルで rc=$rc"; sed 's/^/    /' "$work/out"; fails=$((fails + 1)); }

echo "Scenario: CH03 の後の木は、検査を通る"
python3 -c "$PYSRC" || { echo "  FAIL: いまの木が検査を通らない"; fails=$((fails + 1)); }

if [ "$fails" != 0 ]; then
  echo "self-test: FAIL（$fails 件）"
  exit 1
fi
echo "self-test: OK"
