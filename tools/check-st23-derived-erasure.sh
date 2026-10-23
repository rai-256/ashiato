#!/usr/bin/env bash
# ST23 の派生の門・消去の操作の台帳の節（tools/check-immutable.d/*_st23-derived-erasure.sh）が、
# tools/check-immutable.sh の中で走り、NG が無く、節の OK 行が出ていることを確かめる。
# 節は psql と fail を共有して . で読まれる形なので単独では走らない。
set -euo pipefail
cd "$(dirname "$0")/.."
log="$(mktemp)"; trap 'rm -f "$log"' EXIT
tools/check-immutable.sh > "$log" 2>&1 || { tail -20 "$log"; exit 1; }
grep -q "書き換え禁止の節: .*_st23-derived-erasure" "$log" || { echo "NG 節が走っていない"; exit 1; }
! grep -q "  NG" "$log" || { grep "  NG" "$log"; exit 1; }
n=$(grep -c "  OK .*\(台帳\|書き戻\|作り直し\|原文だけ\)" "$log")
[ "$n" -ge 10 ] || { echo "NG 節の OK 行が $n 本（10 本以上のはず）"; exit 1; }
echo "OK 節の OK 行 $n 本"
