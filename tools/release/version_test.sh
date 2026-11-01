#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
version="$root/tools/release/version.sh"

test_code() {
  [ "$("$version" code 0.3.0-beta.2)" = 30002 ]
  [ "$("$version" code 1.0.0)" = 1000099 ]
}

test_code
