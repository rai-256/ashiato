#!/usr/bin/env bash
set -euo pipefail

usage() { echo "使い方: $0 code|deps" >&2; exit 2; }

code() {
  if grep -vE '^(docs/|openspec/)|\.md$' | grep -q .; then
    exit 0
  fi
  exit 1
}

deps() {
  if grep -qE '^(Cargo\.lock|web/package-lock\.json|osv-scanner\.toml|tools/check-vulns\.sh|tools/check-pg-minor\.sh)$|^collector-android/.*\.lockfile$'; then
    exit 0
  fi
  exit 1
}

case "${1:-}" in
  code) [[ $# == 1 ]] || usage; code ;;
  deps) [[ $# == 1 ]] || usage; deps ;;
  *) usage ;;
esac
