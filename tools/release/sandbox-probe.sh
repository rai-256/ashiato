#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
[[ "${1:-}" == p19 ]] || { echo "使い方: $0 p19" >&2; exit 2; }
result="${ASHIATO_SANDBOX_PROBE_RESULT:-$root/.superpowers/sdd/st55-task-3/p19-result.md}"
mkdir -p "$(dirname "$result")"
if ! command -v powershell.exe >/dev/null 2>&1; then
  printf '%s\n' '# P19 probe' '' 'P16=UNAVAILABLE: Windows サンドボックスを起動できない。' 'P19=UNAVAILABLE: 確認は install-windows CI に委ねる。' >"$result"
  exit 0
fi
work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-p19.XXXXXX")"
trap 'rm -rf "$work"' EXIT
cat >"$work/p19.wsb" <<EOF
<Configuration>
  <MappedFolders><MappedFolder><HostFolder>$(cygpath -w "$work")</HostFolder><SandboxFolder>C:\\Users\\WDAGUtilityAccount\\Desktop\\p19</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder></MappedFolders>
  <LogonCommand><Command>powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\\Users\\WDAGUtilityAccount\\Desktop\\p19\\probe.ps1</Command></LogonCommand>
</Configuration>
EOF
cat >"$work/probe.ps1" <<'EOF'
$ErrorActionPreference = 'Stop'
$out = 'C:\Users\WDAGUtilityAccount\Desktop\p19\p19-result.txt'
try {
  $sid = (whoami /user | Select-String 'S-1-' | Out-String).Trim()
  $pg = Get-ChildItem C:\Users\WDAGUtilityAccount\Desktop\p19\pgsql\bin\pg_ctl.exe -ErrorAction SilentlyContinue
  $initdb = Get-ChildItem C:\Users\WDAGUtilityAccount\Desktop\p19\pgsql\bin\initdb.exe -ErrorAction SilentlyContinue
  if (-not $pg -or -not $initdb) { 'P19=UNAVAILABLE: pg_ctl/initdb was not mounted' | Set-Content $out; exit 0 }
  $db = 'C:\Users\WDAGUtilityAccount\Desktop\p19\db'
  New-Item -ItemType Directory -Force $db | Out-Null
  & $initdb.FullName -D $db -U ashiato *> C:\Users\WDAGUtilityAccount\Desktop\p19\initdb.txt
  $initdbExit = $LASTEXITCODE
  if ($initdbExit -ne 0) {
    'P19=FAIL: initdb exited with code ' + $initdbExit | Set-Content $out
    exit 1
  }
  & $pg.FullName register -N ashiato-db -U 'NT SERVICE\ashiato-db' -D $db -S auto -w *> C:\Users\WDAGUtilityAccount\Desktop\p19\pgctl.txt
  $pgctlExit = $LASTEXITCODE
  if ($pgctlExit -ne 0) {
    'P19=FAIL: pg_ctl register exited with code ' + $pgctlExit | Set-Content $out
    exit 1
  }
  'P16=PASS sandbox-logon-and-mapped-write' | Set-Content $out
  'P19=PASS sid=' + $sid + ' register=PASS initdb=PASS' | Add-Content $out
} catch { 'P19=FAIL: ' + $_.Exception.Message | Set-Content $out; exit 1 }
EOF
cp "$root/tools/release/pgsql.sh" "$work/pgsql.sh"
if [[ -n "${ASHIATO_PGSQL_DIR:-}" && -d "$ASHIATO_PGSQL_DIR" ]]; then
  cp -a "$ASHIATO_PGSQL_DIR" "$work/pgsql"
else
  printf '%s\n' '# P19 probe' '' 'P16=UNAVAILABLE: 同梱 PostgreSQL の置き場が無い。' 'P19=UNAVAILABLE: サンドボックスの実測を CI に委ねる。' >"$result"
  exit 0
fi
if powershell.exe -NoProfile -NonInteractive -Command "Start-Process -Wait -FilePath WindowsSandbox.exe -ArgumentList '$work/p19.wsb'"; then
  if [[ -f "$work/p19-result.txt" ]]; then cp "$work/p19-result.txt" "$result"; grep -q 'P19=FAIL' "$result" && exit 1 || exit 0; fi
fi
printf '%s\n' '# P19 probe' '' 'P16=UNAVAILABLE: サンドボックスの結果を回収できない。' 'P19=UNAVAILABLE: 確認は install-windows CI に委ねる。' >"$result"
exit 0
