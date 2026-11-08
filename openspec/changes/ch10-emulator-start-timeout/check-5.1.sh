#!/usr/bin/env bash
# Task 5.1 の検証（tasks.md の旧コマンドは backtick を含み、検証コマンドとして拾うと途中で切れた。2026-11-07 doctor）。
# 確かめることは旧コマンドと同じ 3 つ:
#   1. AGENTS.md に「error: /dev/kvm を開けない」がある
#   2. 収集アプリの起動台本に注記「数十倍遅い」が残っていない
#   3. docs/testing.md の「手元: `./tools/<起動台本>`」の行に「同じ AVD」がある
set -euo pipefail
cd "$(dirname "$0")/../../.."
script="android-emulator.sh"
grep -q "error: /dev/kvm を開けない" AGENTS.md
if grep -q "数十倍遅い" "tools/$script"; then exit 1; fi   # `! grep` は set -e で止まらない
grep -F "手元: \`./tools/$script\`" docs/testing.md | grep -q "同じ AVD"
