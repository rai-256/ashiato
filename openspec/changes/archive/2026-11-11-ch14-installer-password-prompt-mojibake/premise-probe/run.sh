#!/usr/bin/env bash
# CH14 premises.md 9 行目の確かめを WSL から走らせ直す（親の出力を tee の管にしたまま）。結果は result.txt と同じ形で出る。
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"; ps=/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
cp "$here/probe.ps1" "$here/child.ps1" "$tmp/"
cmd="cmd.exe /d /c \"chcp 932 >nul & chcp & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $(wslpath -w "$tmp/child.ps1")\""
"$ps" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$(wslpath -w "$tmp/probe.ps1")" -Command "$cmd" -Out "$(wslpath -w "$tmp/out.bin")" | tee "$tmp/pipe.log" >/dev/null
python3 -I -c 'import re,sys;t=open(sys.argv[1],"rb").read().decode("utf-8","replace");s=re.sub(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07","\n",t);print("\n".join(l.strip() for l in s.splitlines() if l.strip()))' "$tmp/out.bin"
