#!/usr/bin/env bash
# Task 4.3: CH19 の成果物と review_triage が必須にする申し送りだけを許す。
set -euo pipefail

base=$(git merge-base origin/main HEAD)
files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } )
! printf '%s\n' "$files" | grep -vE '^(tools/android-emulator\.sh|tools/test-android-emulator\.sh|AGENTS\.md|docs/changes/CH19\.md|docs/handoff/CH19\.md|openspec/changes/ch19-emulator-only-class/|docs/briefs/CH19|\.superpowers/|$)'
