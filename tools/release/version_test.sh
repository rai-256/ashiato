#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
version="$root/tools/release/version.sh"

test_code() {
  [ "$("$version" code 0.2.0)" = 20000 ]
  [ "$("$version" code 0.4.0-beta.2)" = 40000 ]
  [ "$("$version" code 1.0.0)" = 10000000 ]
  ! "$version" code 0.1.10000 2>/dev/null
}

test_code
