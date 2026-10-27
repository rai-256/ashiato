#!/usr/bin/env bash
# 書き出しと件数の数えの時間を、記録の行数 N と消した滞在の数 M を一緒に倍にした 3 通りで測って残す（CH04 design D14）。
#
#   tools/export-scaling.sh              # 3 回走らせる。120 万行で数分。CI では走らせない
#   tools/export-scaling.sh --self-test  # 偽の時間で、値と比を出すこと・試験の rc を返すことを見る
#
# 出すのは各回の seconds・summary_seconds と、隣り合う組の比だけ。比で合否は決めない（合否は読んだ行の数。tasks 8.3）。
# EXPORT_SCALING_CMD に、`kind=export rows=… seconds=… summary_seconds=…` の行を標準エラーか標準出力へ出すコマンドを渡すと、
# 試験の代わりにそれを走らせる（環境に ASHIATO_EXPORT_VOLUME_ROWS と ASHIATO_EXPORT_VOLUME_ERASED を渡す）。
set -euo pipefail
cd "$(dirname "$0")/.."

POINTS=("300000 25" "600000 50" "1200000 100")

run_points() {
  local out line n m rc secs sum
  out=$(mktemp)
  trap 'rm -f "$out"' RETURN
  local prev_secs="" prev_sum=""
  for point in "${POINTS[@]}"; do
    read -r n m <<<"$point"
    rc=0
    ASHIATO_EXPORT_VOLUME_ROWS=$n ASHIATO_EXPORT_VOLUME_ERASED=$m ASHIATO_TEST_STATEMENT_TIMEOUT=0 \
      bash -c "${EXPORT_SCALING_CMD:-set -a; [ -f .env ] && . ./.env; set +a; cargo test -p ashiato-server export_volume_one_run_reports_time_and_memory -- --nocapture}" \
      >"$out" 2>&1 || rc=$?
    if [ "$rc" -ne 0 ]; then
      tail -n 30 "$out" >&2
      echo "N=$n M=$m の試験が落ちた（rc=$rc）" >&2
      return "$rc"
    fi
    line=$(grep -E 'kind=export rows=[0-9]+ seconds=' "$out" | tail -n 1 || true)
    secs=$(sed -n 's/.* seconds=\([0-9.]*\).*/\1/p' <<<"$line")
    sum=$(sed -n 's/.* summary_seconds=\([0-9.]*\).*/\1/p' <<<"$line")
    if [ -z "$secs" ] || [ -z "$sum" ]; then
      echo "N=$n M=$m の時間の行が読めない" >&2
      return 2
    fi
    echo "N=$n M=$m seconds=$secs summary_seconds=$sum"
    if [ -n "$prev_secs" ]; then
      awk -v a="$prev_secs" -v b="$secs" -v c="$prev_sum" -v d="$sum" \
        'BEGIN { printf "  前の組との比 seconds=%.2f summary_seconds=%.2f\n", (a > 0 ? b / a : 0), (c > 0 ? d / c : 0) }'
    fi
    prev_secs=$secs
    prev_sum=$sum
  done
}

self_test() {
  local out fake
  fake='n=$ASHIATO_EXPORT_VOLUME_ROWS; echo "kind=export rows=$n seconds=$((n / 100000)).0 summary_seconds=0.5 max_rss_kib=1"'
  out=$(EXPORT_SCALING_CMD="$fake" run_points)
  grep -q '^N=300000 M=25 seconds=3.0 summary_seconds=0.5$' <<<"$out" || { echo "1 点目の値が違う: $out" >&2; exit 1; }
  grep -q '^N=1200000 M=100 seconds=12.0 summary_seconds=0.5$' <<<"$out" || { echo "3 点目の値が違う: $out" >&2; exit 1; }
  [ "$(grep -c '前の組との比 seconds=2.00 summary_seconds=1.00' <<<"$out")" -eq 2 ] || { echo "比が違う: $out" >&2; exit 1; }
  # 比が大きくても rc は 0（比で合否を決めない）。seconds を N の 2 乗で伸ばし、隣り合う比を 4.00 にする
  local rc=0
  out=$(EXPORT_SCALING_CMD='k=$((ASHIATO_EXPORT_VOLUME_ROWS / 100000)); echo "kind=export rows=1 seconds=$((k * k)).0 summary_seconds=1.0"' run_points) \
    || rc=$?
  [ "$rc" -eq 0 ] || { echo "比で落ちた: rc=$rc" >&2; exit 1; }
  [ "$(grep -c '前の組との比 seconds=4.00 summary_seconds=1.00' <<<"$out")" -eq 2 ] || { echo "大きな比を出さない: $out" >&2; exit 1; }
  # 試験が落ちたら、その rc で落ちる
  rc=0
  EXPORT_SCALING_CMD='exit 7' run_points >/dev/null 2>&1 || rc=$?
  [ "$rc" -eq 7 ] || { echo "試験の rc を返さない: rc=$rc" >&2; exit 1; }
  # 時間の行が無ければ rc=2
  rc=0
  EXPORT_SCALING_CMD='echo nothing' run_points >/dev/null 2>&1 || rc=$?
  [ "$rc" -eq 2 ] || { echo "時間の行が無いのに rc=$rc" >&2; exit 1; }
  echo "self-test ok"
}

case "${1:-}" in
  --self-test) self_test ;;
  "")
    echo "Scenario: 記録の行数と消した滞在の数を一緒に倍にした 3 通りで、書き出しと件数の数えの時間を測って残す"
    run_points
    ;;
  *) echo "知らない引数: $1" >&2; exit 2 ;;
esac
