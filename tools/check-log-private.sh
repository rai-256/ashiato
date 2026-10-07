#!/usr/bin/env bash
# ログに私的データを出す**書き方**を止める（製造準備 A-2「私的データを出さない側が既定」）。
#
#   tools/check-log-private.sh              # 追跡ファイル全部を見る（CI の chain job・pre-commit）
#   tools/check-log-private.sh --self-test  # 違反を差し込んだ断片で落ち、正しい書き方で通ること
#
# **なぜ要るか**: 方針は「出すのは件数・ソース名・所要時間・エラーの種別だけ。位置・本文・写真のパス・主観・
# 人物名は出さない。一度出たログは消せない」と決めてあるが、守らせていたのは人の目だけだった（2026-10-07 時点で
# サーバと PC の収集アプリに tracing の呼び出し 60 か所、携帯に Log の呼び出し 40 か所）。値が混ざるかは
# 型では決まらないので、**値を混ぜられる書き方そのもの**を止める:
#
#   Rust（crates/*/src。テスト・src/bin・`#[cfg(test)]` 以降は見ない）
#     - tracing のメッセージに `{}` で値を埋め込まない。例外は `"{}", <…>::telemetry::line(…)` の形だけ
#       （telemetry::line は件数・所要時間・種別しか受け取らない）
#     - 構造化フィールドの名前は下の ALLOWED だけ。足すときは、私的データにならない理由をここに書く
#     - `?`（Debug）で値を出さない（エラーの Debug には要求の本文が入ることがある）。例外は ALLOWED_DEBUG
#     - println! / eprintln! / print! / eprint! / dbg! を使わない
#   Kotlin（collector-android の main）
#     - `Log.x(TAG, …)` と、ログ用のコールバック（`(String) -> Unit` の型を持つ名前）に渡すのは
#       `Telemetry.line(…)` か、コールバックの中継の `it` だけ。文字列を手で組み立てない
#   TypeScript（web/src。テストは見ない）
#     - console.* を使わない（画面のログは利用者の端末の外へ出る経路を持たないが、記録を出す癖を付けない）
#
# どうしても要る箇所は、呼び出しの直前（3 行以内）か同じ行に `// log-ok: <私的データにならない理由>` を書く。
# 理由の無い印は印として数えない。
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - "$@" <<'PY'
import re, subprocess, sys

# 構造化フィールドとして出してよい名前。**値の中身が私的データにならない**ものだけ。
# 方針の 4 つ（件数・ソース名・所要時間・エラーの種別）の外にあるものは、理由と決めた日を添える。
ALLOWED = {
    # 種別・理由（自分で名付けた語。下位の文言を素通しさせない）
    "kind", "reason", "why", "failure", "error", "op", "sqlstate", "field",
    # 件数・所要時間・深さ
    "count", "sent", "accepted", "read", "dropped", "successors", "versions", "took_ms", "depth",
    "days", "days_done", "stays_before", "stays_after",
    # ソース名（cycle_at は引き継ぎの鎖が輪になった所のソース名）・真偽
    "logical_source", "cycle_at", "credential_present", "has_external_id",
    # コードの位置（panic の file:line）
    "location",
    # ---- 方針の 4 つの外。本人が 2026-10-07 に認めた（記録の中身ではなく、調査に要る値）----
    # user: 利用者の UUID（いまは本人 1 人なので常に同じ値）/ id: 壊れた属性の行を DB で探す UUID（値は出ない）
    "user", "id",
    # day: 滞在を作り直した日付（JST）。「その日に位置の記録がある」は分かるが場所は出ない。失敗した日を手で作り直すのに要る
    # at: 携帯が生存信号を出した時刻（登録簿より前の信号の調査）
    "day", "at",
    # addr: サーバの待ち受けの番地（起動時に 1 回）。私設網の IP のことがある
    "addr",
}
ALLOWED_DEBUG = {"location"}   # panic の発生位置（ソースの file:line）

MARK = re.compile(r"//\s*log-ok:\s*\S")
RUST_MAC = re.compile(r"\b(?:tracing::)?(info|warn|error|debug|trace)!\s*\(|\b(println|eprintln|print|eprint|dbg)!\s*\(")
TELEMETRY = re.compile(r"^\s*(?:\w+::)*telemetry::line\s*\(")
STR = re.compile(r'"(?:[^"\\]|\\.)*"')


def balanced(text, i):
    """text[i] は開き括弧の直後。対応する閉じ括弧の位置を返す（文字列リテラルの中は数えない）。"""
    d = 1
    while i < len(text):
        c = text[i]
        if c == '"':
            i += 1
            while i < len(text) and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
        elif c in "([{":
            d += 1
        elif c in ")]}":
            d -= 1
            if d == 0:
                return i
        i += 1
    return i


def split_args(body):
    out, d, cur, i = [], 0, "", 0
    while i < len(body):
        c = body[i]
        if c == '"':
            j = i + 1
            while j < len(body) and body[j] != '"':
                j += 2 if body[j] == "\\" else 1
            cur += body[i:j + 1]
            i = j + 1
            continue
        if c in "([{":
            d += 1
        elif c in ")]}":
            d -= 1
        if c == "," and d == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += c
        i += 1
    if cur.strip():
        out.append(cur.strip())
    return out


def marked(lines, ln):
    """ln（1 始まり）の行か、その直前 3 行に理由つきの印があるか。"""
    return any(MARK.search(lines[k]) for k in range(max(0, ln - 4), ln))


def strip_tests(text):
    """`#[cfg(test)]` の付いた項目（`mod x { … }`・関数）だけを空行に置き換える。行番号は保つ。
    `#[cfg(test)] mod x;` の宣言はそのまま（その先を読み飛ばさない —— 2026-10-07: 最初の `#[cfg(test)]` から後ろを
    切っていたので、先頭近くで `mod …_tests;` を宣言する lib.rs のほぼ全部を見ていなかった）。"""
    out, i = [], 0
    for m in re.finditer(r"#\[cfg\(test\)\]", text):
        if m.start() < i:
            continue
        j = m.end()
        semi, brace = text.find(";", j), text.find("{", j)
        if brace == -1 or (semi != -1 and semi < brace):
            continue
        end = balanced(text, brace + 1)
        out.append(text[i:m.start()])
        out.append(re.sub(r"[^\n]", " ", text[m.start():end + 1]))
        i = end + 1
    out.append(text[i:])
    return "".join(out)


def check_rust(path, text):
    text = strip_tests(text)
    lines = text.splitlines()
    out = []
    for m in RUST_MAC.finditer(text):
        ln = text.count("\n", 0, m.start()) + 1
        if lines[ln - 1].lstrip().startswith("//") or marked(lines, ln):
            continue
        if m.group(2):
            out.append(f"{path}:{ln}: {m.group(2)}! を使わない（tracing の構造化フィールドで出す）")
            continue
        body = text[m.end():balanced(text, m.end())]
        args = split_args(body)
        msg_at = next((k for k, a in enumerate(args) if STR.fullmatch(a)), None)
        for k, a in enumerate(args):
            if k == msg_at:
                break
            if a.startswith("target:") or a.startswith("parent:"):
                continue
            f = re.match(r"^([%?]?)([A-Za-z_][\w.]*)\s*(?:=\s*([%?]?)(.*))?$", a, re.S)
            if not f:
                out.append(f"{path}:{ln}: 読めないフィールド `{a[:40]}`")
                continue
            name, debug = f.group(2), "?" in (f.group(1), f.group(3))
            if name not in ALLOWED:
                out.append(f"{path}:{ln}: 許可していないフィールド `{name}`（出してよいのは件数・ソース名・所要時間・"
                           "エラーの種別。足すなら tools/check-log-private.sh の ALLOWED に理由つきで）")
            if debug and name not in ALLOWED_DEBUG:
                out.append(f"{path}:{ln}: `{name}` を ?（Debug）で出さない（エラーの Debug には要求の本文が入ることがある。"
                           "種別の名前を出す）")
        if msg_at is not None and re.search(r"\{[^{}]*\}", args[msg_at]):
            rest = args[msg_at + 1:]
            if not (args[msg_at] == '"{}"' and len(rest) == 1 and TELEMETRY.match(rest[0])):
                out.append(f"{path}:{ln}: メッセージに値を埋め込まない `{args[msg_at][:40]}`（構造化フィールドか telemetry::line で出す）")
    return out


def callback_names(texts):
    names = set()
    for t in texts:
        names |= set(re.findall(r"\b(\w+)\s*:\s*\(String\)\s*->\s*Unit", t))
    return names


def check_kotlin(path, text, names):
    out, lines = [], text.splitlines()
    pat = re.compile(r"\bLog\.[vdiwe]\s*\(|\bprintln\s*\(|(?<![\w.])(" + "|".join(sorted(names)) + r")\s*\(" if names
                     else r"\bLog\.[vdiwe]\s*\(|\bprintln\s*\(")
    for m in pat.finditer(text):
        ln = text.count("\n", 0, m.start()) + 1
        line = lines[ln - 1].lstrip()
        if line.startswith(("//", "*", "/*")) or marked(lines, ln):
            continue
        if re.search(r"\bfun\s+\w+\s*\($", text[max(0, m.start() - 40):m.end()]):
            continue
        if m.group(0).startswith("println"):
            out.append(f"{path}:{ln}: println を使わない（Log と Telemetry.line で出す）")
            continue
        args = split_args(text[m.end():balanced(text, m.end())])
        payload = args[1:] if m.group(0).startswith("Log.") else args
        if not payload:
            continue
        a = payload[0]
        if a == "it" or a.startswith("Telemetry.line("):
            continue
        out.append(f"{path}:{ln}: ログに Telemetry.line を通さない文字列を渡している `{a[:50]}`"
                   "（出してよいのは件数・ソース名・所要時間・エラーの種別だけ。Telemetry.line で組み立てる）")
    return out


def check_ts(path, text):
    out, lines = [], text.splitlines()
    for m in re.finditer(r"\bconsole\.(log|info|warn|error|debug)\s*\(", text):
        ln = text.count("\n", 0, m.start()) + 1
        if not lines[ln - 1].lstrip().startswith("//") and not marked(lines, ln):
            out.append(f"{path}:{ln}: console.{m.group(1)} を使わない")
    return out


def files(*globs):
    return subprocess.run(["git", "ls-files", *globs], capture_output=True, text=True, check=True).stdout.split()


def scan():
    out = []
    for p in files("crates/*.rs"):
        if p.endswith("_tests.rs") or "/tests/" in p or "/src/bin/" in p or "/src/" not in p:
            continue
        out += check_rust(p, open(p, encoding="utf-8").read())
    kts = [p for p in files("collector-android/app/src/main/*.kt")]
    texts = {p: open(p, encoding="utf-8").read() for p in kts}
    names = callback_names(texts.values())
    for p, t in texts.items():
        out += check_kotlin(p, t, names)
    for p in files("web/src/*.ts", "web/src/*.tsx"):
        if "__tests__" in p or p.endswith((".test.ts", ".test.tsx")):
            continue
        out += check_ts(p, open(p, encoding="utf-8").read())
    return out


def self_test():
    bad = {
        "埋め込み": ('tracing::info!("置いた場所: {}", path.display());', 1),
        "許可外の項目": ('tracing::warn!(kind = "x", title = %t, "断った");', 1),
        "Debug": ('tracing::warn!(kind = "x", error = ?e, "断った");', 1),
        "eprintln": ('eprintln!("error: {}", e);', 1),
        "印に理由が無い": ('// log-ok:\neprintln!("x");', 1),
    }
    good = {
        "構造化": ('tracing::warn!(kind = "drop_invalid", logical_source = %s, count = n, "断った");', 0),
        "telemetry": ('tracing::info!("{}", c02::telemetry::line("started", None, None, None));', 0),
        "理由つきの印": ('// log-ok: reason() は自分で名付けた &\'static str\neprintln!("kind={}", r.reason());', 0),
        "テストは見ない": ('#[cfg(test)]\nmod tests { fn t() { println!("{}", x); } }', 0),
        "テストの宣言の後も見る": ('#[cfg(test)]\nmod a_tests;\neprintln!("x {}", y);', 1),
    }
    fails = []
    for name, (src, want) in {**bad, **good}.items():
        got = len(check_rust("t.rs", "fn f() {\n" + src + "\n}\n"))
        if (got > 0) != (want > 0):
            fails.append(f"Rust {name}: 期待 {'違反' if want else '通る'} / 実際 {got} 件")
    names = callback_names(["class A(private val log: (String) -> Unit)"])
    kt = {
        "手組みの文字列": ('Log.w(TAG, "kind=x source=$s error=$e")', 1),
        "コールバックへ手組み": ('log("kind=x available=$a")', 1),
        "Telemetry": ('Log.w(TAG, Telemetry.line("x", source = null, count = 1))', 0),
        "中継の it": ('val f = { Log.i(TAG, it) }', 0),
        "コールバックへ Telemetry": ('log(Telemetry.line("x", source = s))', 0),
    }
    for name, (src, want) in kt.items():
        got = len(check_kotlin("t.kt", "fun f() {\n    " + src + "\n}\n", names))
        if (got > 0) != (want > 0):
            fails.append(f"Kotlin {name}: 期待 {'違反' if want else '通る'} / 実際 {got} 件")
    if len(check_ts("t.ts", "console.log(x)\n")) != 1:
        fails.append("TypeScript console.log を見逃した")
    if fails:
        print("self-test: FAIL\n" + "\n".join(fails))
        return 1
    print("self-test: OK")
    return 0


if "--self-test" in sys.argv:
    sys.exit(self_test())
found = scan()
if found:
    print("ログに私的データを出せる書き方がある（製造準備 A-2。一度出たログは消せない）:")
    print("\n".join("  " + f for f in found))
    sys.exit(1)
print("check-log-private: OK")
PY
