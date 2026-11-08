#!/usr/bin/env bash
set -euo pipefail

ROOT=${CHECK_VULNS_WIRING_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}

if [[ ${1:-} == --self-test ]]; then
  tmp=$(mktemp -d)
  trap 'rm -rf -- "$tmp"' EXIT
  mkdir -p "$tmp/.github/workflows"
  cp "$ROOT/.github/workflows/"*.yml "$tmp/.github/workflows/"

  CHECK_VULNS_WIRING_ROOT="$tmp" "$0" >/dev/null

  expect_fail() {
    local label=$1
    shift
    cp "$ROOT/.github/workflows/"*.yml "$tmp/.github/workflows/"
    "$@"
    if CHECK_VULNS_WIRING_ROOT="$tmp" "$0" >/dev/null 2>&1; then
      echo "self-test: $label が欠けても通った" >&2
      exit 1
    fi
  }

  expect_fail 'deps output' sed -i 's/      deps:.*$/      # deps を削除/' "$tmp/.github/workflows/ci.yml"
  expect_fail 'all deps' sed -i 's/for k in code android windows stamp deps/for k in code android windows stamp/' "$tmp/.github/workflows/ci.yml"
  expect_fail 'vulns PR 条件' sed -i "s/if: github.event_name == 'pull_request' && needs.changes.outputs.deps == 'true'/if: needs.changes.outputs.deps == 'true'/" "$tmp/.github/workflows/ci.yml"
  expect_fail 'wiring checks' sed -i '/check-vulns-wiring.sh --self-test/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review severity' sed -i '/fail-severity: high/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review scopes' sed -i '/fail-on-scopes:/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review allow list' sed -i '/allow-ghsas:/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review comments' sed -i '/fail-severity: high/a\          comment-summary-in-pr: always' "$tmp/.github/workflows/ci.yml"
  expect_fail 'issues always' sed -i '/if: always()/d' "$tmp/.github/workflows/vulns-nightly.yml"
  expect_fail 'release needs' sed -i 's/needs: \[plan, vulns\]/needs: plan/' "$tmp/.github/workflows/release.yml"
  echo 'OK: 脆弱性 CI 配線の自己テストに成功'
  exit 0
fi

python3 - "$ROOT" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])

def jobs(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    start = next((i for i, line in enumerate(lines) if line == "jobs:"), None)
    if start is None:
        return {}
    found = {}
    current = None
    for line in lines[start + 1:]:
        if line and not line.startswith(" "):
            break
        match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if match:
            current = match.group(1)
            found[current] = []
        elif current is not None:
            found[current].append(line)
    return {name: "\n".join(body) for name, body in found.items()}

def require(condition, message):
    if not condition:
        raise SystemExit(f"配線エラー: {message}")

ci = jobs(root / ".github/workflows/ci.yml")
release = jobs(root / ".github/workflows/release.yml")
vulns_nightly = jobs(root / ".github/workflows/vulns-nightly.yml")
nightly = jobs(root / ".github/workflows/nightly.yml")

changes = ci.get("changes", "")
require("deps: ${{ steps.f.outputs.deps }}" in changes, "changes が deps を出力していない")
all_line = re.search(r"all\(\).*", changes)
require(all_line and "deps" in all_line.group(0), "all() が deps を出力していない")

vulns = ci.get("vulns", "")
require("needs: changes" in vulns, "CI vulns が changes を待っていない")
require("if: github.event_name == 'pull_request' && needs.changes.outputs.deps == 'true'" in vulns,
        "CI vulns の条件が PR かつ deps ではない")
require("git fetch --no-tags --depth=1 origin \"$BASE_SHA\"" in vulns, "base の commit を取得していない")
require("git cat-file -e \"$BASE_SHA^{commit}\"" in vulns, "base の commit を確認していない")
require("tools/check-vulns.sh --self-test" in vulns and "tools/check-vulns.sh --base \"$BASE_SHA\"" in vulns,
        "CI vulns の検査が不足している")
require("tools/check-vulns.sh allow-ghsas" in vulns, "allow-ghsas を生成していない")
require("actions/dependency-review-action@" in vulns and
        re.search(r"actions/dependency-review-action@[0-9a-f]{40}", vulns),
        "dependency-review-action が SHA 固定でない")
require("fail-severity: high" in vulns, "dependency-review の重大度が high でない")
require(all(x in vulns for x in ("runtime", "development", "unknown")), "dependency-review の範囲が不足")
require("allow-ghsas: ${{ steps.allow.outputs.ghsas }}" in vulns, "dependency-review に許可一覧を渡していない")
require("comment-summary-in-pr" not in vulns or "comment-summary-in-pr: never" in vulns,
        "dependency-review に PR コメントを書かせる設定がある")
require("tools/check-vulns-wiring.sh --self-test" in ci.get("rust", "") and
        "tools/check-vulns-wiring.sh" in ci.get("rust", ""), "rust に配線検査がない")
require("tools/check-pg-minor.sh --self-test" in ci.get("rust", ""), "rust に pg minor 検査がない")
android = ci.get("android", "")
require(re.search(r"./gradlew detekt.*\n.*check-gradle-locks.sh", android, re.S),
        "android の detekt 後に Gradle lock 検査がない")

require("vulns" in release and "needs: plan" in release["vulns"] and
        "contents: read" in release["vulns"] and "tools/check-vulns.sh" in release["vulns"],
        "release の vulns job が不足")
require("needs: [plan, vulns]" in release.get("package-linux", ""), "Linux package が vulns を待っていない")
require("needs: [plan, vulns]" in release.get("package-windows", ""), "Windows package が vulns を待っていない")

require("schedule:" in (root / ".github/workflows/vulns-nightly.yml").read_text() and
        "workflow_dispatch:" in (root / ".github/workflows/vulns-nightly.yml").read_text(),
        "毎晩 workflow の起動条件が不足")
issue_jobs = [body for body in vulns_nightly.values() if "issues: write" in body]
require(len(issue_jobs) == 1, "issues: write の job が 1 つではない")
issues = vulns_nightly.get("issues", "")
require("needs: [vulns, pg-minor]" in issues and "if: always()" in issues, "issues job の待機条件が不足")
nightly_text = (root / ".github/workflows/nightly.yml").read_text(encoding="utf-8")
require("check-vulns" not in nightly_text and "check-pg-minor" not in nightly_text, "nightly に脆弱性検査が混入")

print("== Scenario: CI は判定の結果で脆弱性の job を出し分ける")
print("== Scenario: dependency-review の設定は high・全部の範囲・表からの許可")
print("== Scenario: 毎晩の検査は公開の流れから独立し、issue の権限は 1 つの job だけ")
print("== Scenario: 印の組み立ての job は脆弱性の検査を待つ")
print("== Scenario: 開発版の組み立てと公開は脆弱性の検査を待たない")
PY
