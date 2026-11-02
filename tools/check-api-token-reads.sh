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
        'curl "${AUTH[@]}" http://127.0.0.1/api/outages\n', encoding="utf-8"
    )
    denied = {
        "post.ts": 'request.post("/api/events", { headers: BEARER });\n',
        "context.ts": (
            'const ctx = await playwright.request.newContext({ extraHTTPHeaders: BEARER });\n'
            'await ctx.get("/api/events");\n'
        ),
        "curl.sh": 'curl "${AUTH[@]}" -X POST http://127.0.0.1/api/events\n',
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

python3 - "$@" <<'PY'
from pathlib import Path
import re
import sys

ALLOWED = {
    ("POST", "/api/ingest"),
    ("POST", "/api/heartbeat"),
    ("POST", "/api/drops"),
    ("POST", "/api/media/missing"),
    ("PUT", "/api/media/originals/"),
    ("GET", "/api/contract"),
    ("HEAD", "/api/media/originals/"),
    ("GET", "/api/feelings/reminder"),
    ("GET", "/api/outages"),
}
TOKEN = re.compile(r"\b(?:BEARER|AUTH\[@\]|API_TOKEN)\b")
HTTP_METHOD = re.compile(r"\.(get|head|post|put|patch|delete|fetch)\s*\(", re.I)
URL = re.compile(r"[\"'`](/api/[^\"'`?${} ]+)")

def allowed(method: str, path: str) -> bool:
    method = method.upper()
    path = path.split("?", 1)[0]
    return (method, path) in ALLOWED or (
        method in {"PUT", "HEAD"} and path.startswith("/api/media/originals/")
    )

def calls(source: str):
    for match in HTTP_METHOD.finditer(source):
        start = match.start()
        open_paren = match.end() - 1
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
        body = source[open_paren:end]
        method = match.group(1).upper()
        if method == "FETCH":
            method_match = re.search(r"\bmethod\s*:\s*[\"']([A-Za-z]+)", body, re.I)
            method = method_match.group(1).upper() if method_match else "GET"
        url = URL.search(body)
        if url:
            yield start, method, url.group(1), bool(TOKEN.search(body))

def check(path: Path) -> int:
    source = path.read_text(encoding="utf-8")
    bad = 0
    inherited_contexts = set()
    for context in re.finditer(r"(?:const\s+)?(\w+)\s*=.*?newContext\s*\((.*?)\)", source, re.S):
        if TOKEN.search(context.group(2)):
            inherited_contexts.add(context.group(1))
    for offset, method, mouth, has_token in calls(source):
        line = source.count("\n", 0, offset) + 1
        call_start = source.rfind("\n", 0, offset) + 1
        receiver = re.search(r"(\w+)\s*$", source[call_start:offset])
        inherited = receiver and receiver.group(1) in inherited_contexts
        if has_token or inherited:
            if not allowed(method, mouth):
                print(f"{path}:{line}: API token is not allowed for {method} {mouth}")
                bad = 1

    for match in re.finditer(r"\bcurl\b[^\n]*", source):
        command = match.group(0)
        if not re.search(r"AUTH\[@\]", command):
            continue
        method_match = re.search(r"(?:-X|--request)\s+([A-Za-z]+)", command)
        method = method_match.group(1).upper() if method_match else "GET"
        url = URL.search(command) or re.search(r"https?://[^\s\"']+(/api/[^\s\"']*)", command)
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
