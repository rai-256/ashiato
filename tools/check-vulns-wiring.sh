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
  expect_fail 'wiring の本番の実行' sed -i '/run: \.\/tools\/check-vulns-wiring\.sh$/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review scopes の一部' sed -i 's/fail-on-scopes: runtime, development, unknown/fail-on-scopes: runtime  # development unknown/' "$tmp/.github/workflows/ci.yml"
  expect_fail 'gradle lock の自己試験' sed -i '/check-gradle-locks.sh --self-test/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'gradle lock の判定' sed -i 's@|tools/check-gradle-locks\\.sh\$@@' "$tmp/.github/workflows/ci.yml"
  expect_fail 'release vulns を無効化' sed -i 's@run: tools/check-vulns.sh$@run: tools/check-vulns.sh || true@' "$tmp/.github/workflows/release.yml"
  expect_fail 'dependency-review を無効化' sed -i '/uses: actions\/dependency-review-action/a\        continue-on-error: true' "$tmp/.github/workflows/ci.yml"
  expect_fail 'PR vulns を無効化' sed -i 's@run: tools/check-vulns.sh --base "\$BASE_SHA"$@run: tools/check-vulns.sh --base "\$BASE_SHA" || echo skip@' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review severity' sed -i '/fail-severity: high/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review scopes' sed -i '/fail-on-scopes:/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review allow list' sed -i '/allow-ghsas:/d' "$tmp/.github/workflows/ci.yml"
  expect_fail 'dependency-review comments' sed -i '/fail-severity: high/a\          comment-summary-in-pr: always' "$tmp/.github/workflows/ci.yml"
  expect_fail 'issues always' sed -i '/if: always()/d' "$tmp/.github/workflows/vulns-nightly.yml"
  expect_fail 'CI vulns の時間の上限' sed -i '/^  vulns:$/,/^  [a-z-]*:$/{/^    timeout-minutes:/d}' "$tmp/.github/workflows/ci.yml"
  expect_fail 'release vulns の時間の上限' sed -i '/^  vulns:$/,/^  [a-z-]*:$/{/^    timeout-minutes:/d}' "$tmp/.github/workflows/release.yml"
  expect_fail '毎晩の時間の上限' sed -i '0,/timeout-minutes: 20/{/timeout-minutes: 20/d}' "$tmp/.github/workflows/vulns-nightly.yml"
  expect_fail 'release needs' sed -i 's/needs: \[plan, vulns\]/needs: plan/' "$tmp/.github/workflows/release.yml"
  echo 'OK: 脆弱性 CI 配線の自己テストに成功'
  exit 0
fi

python3 - "$ROOT" "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" <<'PY'
import re
import subprocess
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

def has_exact_run(body, command):
    pattern = rf"^\s*- run: (?:\./)?{re.escape(command)}\s*$"
    return any(re.match(pattern, line) for line in body.splitlines())

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
require(has_exact_run(vulns, "tools/check-vulns.sh --self-test") and
        has_exact_run(vulns, 'tools/check-vulns.sh --base "$BASE_SHA"'),
        "CI vulns の検査が不足している")
require("tools/check-vulns.sh allow-ghsas" in vulns, "allow-ghsas を生成していない")
require("actions/dependency-review-action@" in vulns and
        re.search(r"actions/dependency-review-action@[0-9a-f]{40}", vulns),
        "dependency-review-action が SHA 固定でない")
require("fail-severity: high" in vulns, "dependency-review の重大度が high でない")
require(re.search(r"^\s*fail-on-scopes: runtime, development, unknown\s*$", vulns, re.M),
        "dependency-review の範囲が不足")
require("allow-ghsas: ${{ steps.allow.outputs.ghsas }}" in vulns, "dependency-review に許可一覧を渡していない")
require("comment-summary-in-pr" not in vulns or "comment-summary-in-pr: never" in vulns,
        "dependency-review に PR コメントを書かせる設定がある")
require("continue-on-error: true" not in vulns and "set +e" not in vulns,
        "CI vulns に失敗を無視する設定がある")
# 外へ問い合わせる脆弱性の job はすべて時間の上限を持つ（review/code.md R24・R33）。job を足したらここへ足す
for label, workflow, name in (("CI", ci, "vulns"), ("毎晩", vulns_nightly, "vulns"),
                              ("毎晩", vulns_nightly, "pg-minor"), ("毎晩", vulns_nightly, "issues"),
                              ("release", release, "vulns")):
    require(re.search(r"^    timeout-minutes: [0-9]+\s*$", workflow.get(name, ""), re.M),
            f"{label} の {name} に時間の上限が無い")
# 判定の台本を変えた PR で、判定の自己試験の入った vulns の job が走ること（review/code.md R32）
changes_sh = Path(sys.argv[2]) / "release/changes.sh"
require(subprocess.run([str(changes_sh), "deps"], input="tools/release/changes.sh\n", text=True).returncode == 0,
        "deps の判定が tools/release/changes.sh の変更を拾わない")
rust = ci.get("rust", "")
require(has_exact_run(rust, "tools/check-vulns-wiring.sh --self-test") and
        has_exact_run(rust, "tools/check-vulns-wiring.sh"), "rust に配線検査がない")
require("tools/check-pg-minor.sh --self-test" in ci.get("rust", ""), "rust に pg minor 検査がない")
android = ci.get("android", "")
require(re.search(r"./gradlew detekt.*\n.*check-gradle-locks.sh", android, re.S),
        "android の detekt 後に Gradle lock 検査がない")
require(has_exact_run(android, "../tools/check-gradle-locks.sh --self-test") and
        has_exact_run(android, "../tools/check-gradle-locks.sh"), "android に Gradle lock の検査と自己試験がない")
require("tools/check-gradle-locks\\.sh" in changes, "android の判定が Gradle lock の検査台本の変更を拾わない")

require("vulns" in release and "needs: plan" in release["vulns"] and
        "contents: read" in release["vulns"] and has_exact_run(release["vulns"], "tools/check-vulns.sh") and
        "continue-on-error: true" not in release["vulns"] and "set +e" not in release["vulns"],
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
