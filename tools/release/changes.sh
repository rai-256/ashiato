#!/usr/bin/env bash
set -euo pipefail

usage() { echo "使い方: $0 code" >&2; exit 2; }

code() {
  if grep -vE '^(docs/|openspec/)|\.md$' | grep -q .; then
    exit 0
  fi
  exit 1
}

[[ "${1:-}" == code && $# == 1 ]] || usage
code
