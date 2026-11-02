#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--self-test" ]]; then
  python3 - <<'PY'
from pathlib import Path
from tempfile import TemporaryDirectory
import subprocess

checker = Path("tools/check-api-token-reads.sh").resolve()
with TemporaryDirectory() as directory:
    root = Path(directory)
    (root / "allowed.ts").write_text(
        'request.post("/api/ingest", { headers: BEARER });\n'
        'curl "${AUTH[@]}" http://127.0.0.1/api/outages\n'
        # bash の台本は `/api` を付けない（ST44 R16）
        'curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -d @x.json\n'
        'curl -sf "${AUTH[@]}" -d @x.json "http://$BIND/heartbeat"\n'
        'curl -sf "${AUTH[@]}" "http://$BIND/contract"\n'
        'curl -sf -I "${AUTH[@]}" "http://$BIND/media/originals/abc"\n'
        'curl -sf "${READ[@]}" "http://$BIND/events?date=2026-01-01"\n'
        'curl -s -H "authorization: Bearer $SAT_VALUE" "http://$SAT_BIND/events"\n', encoding="utf-8"
    )
    denied = {
        "post.ts": 'request.post("/api/events", { headers: BEARER });\n',
        "post-with-header.ts": (
            'request.post("/api/events", { extraHTTPHeaders: BEARER });\n'
        ),
        "context.ts": (
            'const ctx = await playwright.request.newContext({ extraHTTPHeaders: BEARER });\n'
            'await ctx.get("/api/events");\n'
        ),
        "context-after-assignment.ts": (
            'const unrelated = "not a context";\n'
            'const ctx = await playwright.request.newContext({ extraHTTPHeaders: BEARER });\n'
            'await ctx.get("/api/events");\n'
        ),
        "typed-context-after-assignment.ts": (
            'const unrelated = "not a context";\n'
            'const ctx: APIRequestContext = await playwright.request.newContext(\n'
            '  { extraHTTPHeaders: BEARER },\n'
            ');\n'
            'await ctx.get("/api/events");\n'
        ),
        "curl.sh": 'curl "${AUTH[@]}" -X POST http://127.0.0.1/api/events\n',
        "curl-bind.sh": 'curl -sf "${AUTH[@]}" "http://$BIND/events?date=2026-01-01"\n',
        "curl-bind-bare.sh": 'curl -sf "${AUTH[@]}" "$BIND/day"\n',
        "curl-post-data.sh": 'curl -sf "${AUTH[@]}" -d @x.json "http://$BIND/feelings/reminder"\n',
        "curl-bearer.sh": 'curl -sf -H "authorization: Bearer $API_TOKEN" "http://$BIND/api/events"\n',
        "curl-auth-header.sh": 'curl -sf -H "$AUTH_HEADER" "http://127.0.0.1:8787/events"\n',
        "curl-head-wrong.sh": 'curl -sf -I "${AUTH[@]}" "http://$BIND/events"\n',
    }
    allowed = subprocess.run([str(checker), str(root / "allowed.ts")], capture_output=True, text=True)
    assert allowed.returncode == 0, allowed.stderr
    for name, source in denied.items():
        path = root / name
        path.write_text(source, encoding="utf-8")
        result = subprocess.run([str(checker), str(path)], capture_output=True, text=True)
        assert result.returncode == 1, (name, result.stdout, result.stderr)
PY
  exit 0
fi

if (($# == 0)); then
  echo "使い方: $0 --self-test | <ファイルかフォルダ…>" >&2
  exit 2
fi

ASHIATO_ROOT="$(cd "$(dirname "$0")/.." && pwd)" python3 - "$@" <<'PY'
from pathlib import Path
import os
import re
import sys

# 許可の一覧はサーバの正本（collector_scope.rs の COLLECTOR_ALLOWED）から読む。写しを持たない（ST44 R25）
SCOPE = Path(os.environ["ASHIATO_ROOT"]) / "crates/server/src/collector_scope.rs"
ALLOWED = [(m.upper(), p) for m, p in re.findall(r'\(Method::(\w+),\s*"([^"]+)"\)', SCOPE.read_text(encoding="utf-8"))]
if not ALLOWED:
    print(f"{SCOPE}: COLLECTOR_ALLOWED を読めない", file=sys.stderr)
    sys.exit(2)
TOKEN = re.compile(r"\b(?:BEARER|AUTH\[@\]|API_TOKEN)\b")
HTTP_METHOD = re.compile(r"(?P<receiver>\w+)\.(?P<method>get|head|post|put|patch|delete|fetch)\s*\(", re.I)
URL = re.compile(r"[\"'`](/api/[^\"'`?${} ]+)")
CURL_TOKEN = re.compile(r"AUTH\[@\]|\bAPI_TOKEN\b|\bAUTH_HEADER\b")
# `http://host:port/<口>`・`http://$BIND/<口>`・`$BIND/<口>`・`${BIND}/<口>`（`/api` の有無を問わない）
CURL_URL = re.compile(r"(?:https?://[^\s\"'/]+|\$\{?BIND\}?)(/[^\s\"']*)")
CONTEXT = re.compile(
    r"(?m)^\s*(?:const\s+|let\s+|var\s+)?(\w+)\s*(?::[^=\n]+)?=\s*"
    r"(?:await\s+)?(?:[\w.]+\.)?newContext\s*\("
)

def allowed(method: str, path: str) -> bool:
    method = method.upper()
    path = path.split("?", 1)[0]
    if path.startswith("/api/"):
        path = path[len("/api"):]
    for allowed_method, template in ALLOWED:
        pattern = "^" + re.sub(r"\\\{[^}]*\\\}", "[^/]+", re.escape(template)) + "$"
        if allowed_method == method and re.match(pattern, path):
            return True
    return False

def call_body(source: str, open_paren: int) -> str:
    depth = 0
    quote = None
    escaped = False
    end = len(source)
    for index in range(open_paren, len(source)):
        char = source[index]
        if quote:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char in "\"'`":
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                end = index + 1
                break
    return source[open_paren:end]

def calls(source: str):
    for match in HTTP_METHOD.finditer(source):
        start = match.start()
        body = call_body(source, match.end() - 1)
        method = match.group("method").upper()
        if method == "FETCH":
            method_match = re.search(r"\bmethod\s*:\s*[\"']([A-Za-z]+)", body, re.I)
            method = method_match.group(1).upper() if method_match else "GET"
        url = URL.search(body)
        if url:
            yield start, match.group("receiver"), method, url.group(1), bool(TOKEN.search(body))

def check(path: Path) -> int:
    source = path.read_text(encoding="utf-8")
    bad = 0
    inherited_contexts = set()
    for context in CONTEXT.finditer(source):
        body = call_body(source, context.end() - 1)
        if TOKEN.search(body):
            inherited_contexts.add(context.group(1))
    for offset, receiver, method, mouth, has_token in calls(source):
        line = source.count("\n", 0, offset) + 1
        inherited = receiver in inherited_contexts
        if has_token or inherited:
            if not allowed(method, mouth):
                print(f"{path}:{line}: API token is not allowed for {method} {mouth}")
                bad = 1

    # bash の台本は `http://$BIND/<口>` の形で `/api` を付けない（ST44 R16・R21）。資格情報は AUTH[@] のほか、
    # 見出しを直に書いたもの（`Bearer $API_TOKEN`・`$AUTH_HEADER`）も見る。衛星の `Bearer $SAT_VALUE` は API の合言葉でないので見ない
    for match in re.finditer(r"\bcurl\b[^\n]*", source):
        command = match.group(0)
        if not CURL_TOKEN.search(command):
            continue
        method_match = re.search(r"(?:-X|--request)\s+([A-Za-z]+)", command)
        if method_match:
            method = method_match.group(1).upper()
        elif re.search(r"\s(?:-I|--head)\b", command):
            method = "HEAD"
        elif re.search(r"\s(?:-d|--data[\w-]*|-F|--form|-T|--upload-file)\b", command):
            method = "PUT" if re.search(r"\s(?:-T|--upload-file)\b", command) else "POST"
        else:
            method = "GET"
        url = URL.search(command) or CURL_URL.search(command)
        if not url:
            continue
        mouth = url.group(1)
        line = source.count("\n", 0, match.start()) + 1
        if not allowed(method, mouth):
            print(f"{path}:{line}: API token is not allowed for {method} {mouth}")
            bad = 1
    return bad

status = 0
for argument in sys.argv[1:]:
    target = Path(argument)
    if not target.exists():
        print(f"{target}: ファイルまたはフォルダが無い", file=sys.stderr)
        status = 1
        continue
    files = [target] if target.is_file() else [p for p in target.rglob("*") if p.suffix in {".ts", ".tsx", ".sh"}]
    for file in files:
        status |= check(file)
sys.exit(status)
PY
