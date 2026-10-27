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
# OK 行はこの節の見出しから次の見出しまでだけを数える（ほかの節の OK 行で足りてしまわないように。code-verify R17）。
# 下限は節のいまの OK 行の数（節に確かめを足したら上げる）
n=$(awk '/^== 書き換え禁止の節: .*_st23-derived-erasure/{f=1;next} /^== /{f=0} f && /^  OK /' "$log" | wc -l)
[ "$n" -ge 11 ] || { echo "NG 節の OK 行が $n 本（11 本以上のはず）"; exit 1; }
echo "OK 節の OK 行 $n 本"
