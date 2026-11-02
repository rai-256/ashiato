#!/usr/bin/env bash
set -euo pipefail

allowed='(/api/ingest|/api/heartbeat|/api/drops|/api/media/missing|/api/media/originals/|/api/contract|/api/outages|/api/feelings/reminder)'
check_file() {
  local file=$1
  awk -v file="$file" -v allowed="$allowed" '
    { lines[NR] = $0; if ($0 ~ /headers:[[:space:]]*BEARER|Authorization:[^}]*API_TOKEN|AUTH\[/) markers[NR] = 1 }
    END {
      for (n in markers) { found = 0; start = n - 12; if (start < 1) start = 1; stop = n + 12
        read = 0
        for (i = n; i >= start; i--) {
          if (lines[i] ~ /request\./) { if (lines[i] ~ /\.(get|head)\(/) read = 1; break }
        }
        for (i = start; i <= stop; i++) if (lines[i] ~ allowed) found = 1
        if (read && !found) { printf "%s:%d: API token read is not allowed\n", file, n; bad = 1 } }
      exit bad
    }
  ' "$file"
}
if [[ "${1:-}" == "--self-test" ]]; then
  tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
  printf '%s\n' 'request.post("/api/ingest", { headers: BEARER });' >"$tmp/allowed.ts"
  printf '%s\n' 'request.get("/api/events", { headers: BEARER });' >"$tmp/denied.ts"
  check_file "$tmp/allowed.ts"
  if check_file "$tmp/denied.ts" >/dev/null 2>&1; then echo "self-test: /events が許可されている" >&2; exit 1; fi
  exit 0
fi
if (($# == 0)); then echo "使い方: $0 --self-test | <ファイルかフォルダ…>" >&2; exit 2; fi
status=0
for target in "$@"; do
  if [[ -d "$target" ]]; then
    while IFS= read -r -d '' file; do check_file "$file" || status=1; done < <(find "$target" -type f \( -name '*.ts' -o -name '*.tsx' -o -name '*.sh' \) -print0)
  elif [[ -f "$target" ]]; then check_file "$target" || status=1
  else echo "$target: ファイルまたはフォルダが無い" >&2; status=1
  fi
done
exit "$status"
