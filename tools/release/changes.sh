#!/usr/bin/env bash
set -euo pipefail

usage() { echo "使い方: $0 code|deps" >&2; exit 2; }

code() {
  # grep 1 本で済ます（2 本つなぐと、先の grep が SIGPIPE で落ち、pipefail が「文書だけ」と誤読する。大きな一覧で起きる）
  if grep -qvE '^(docs/|openspec/)|\.md$'; then
    exit 0
  fi
  exit 1
}

deps() {
  # 判定そのもの（この台本）を変えた PR でも vulns の job を走らせ、判定の自己試験（check-vulns.sh --self-test）を通す
  if grep -qE '^(Cargo\.lock|web/package-lock\.json|osv-scanner\.toml|tools/check-vulns(-issues)?\.(sh|py)|tools/check-pg-minor\.sh|tools/release/changes\.sh)$|^collector-android/.*\.lockfile$'; then
    exit 0
  fi
  exit 1
}

case "${1:-}" in
  code) [[ $# == 1 ]] || usage; code ;;
  deps) [[ $# == 1 ]] || usage; deps ;;
  *) usage ;;
esac
