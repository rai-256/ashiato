#!/usr/bin/env bash
# サーバが記録の写しを拠点外へ書き出す部品を持たないことを確かめる（ST28 / design D14 / NFR-15）。
#
#   tools/check-offsite.sh              # cargo tree の crate 名を一覧と突き合わせる
#   tools/check-offsite.sh --self-test  # 一覧の名前を 1 つ含む偽の出力で落ちること
#
# **限界**: tokio の `net` と sqlx で外へ接続を張るコードは書ける。この検査が止めるのは
# 「外への送り手の部品を足すこと」まで（DB の接続先は起動時の検査が見る）。
# 暗号化されたバックアップの送り手を足す Story は、下の一覧からその crate だけを外し、spec を MODIFIED にする。
set -euo pipefail
cd "$(dirname "$0")/.."

# 外部の宛先へ送る crate の一覧。**一覧はここに持つ。** 完全一致
exact=(reqwest ureq isahc surf attohttpc curl lettre object_store opendal)
# 前方一致（`aws-sdk-*` など）
prefix=(aws-sdk- rusoto_ google-cloud- azure_)

# 空の一覧では走らない（空だと何を見ても通る）
if [ "$(( ${#exact[@]} + ${#prefix[@]} ))" -lt 1 ]; then
  echo "error: 外部の宛先へ送る部品の一覧が空。検査にならない" >&2; exit 1
fi

# 標準入力の cargo tree（--prefix none）から一覧に当たる crate 名を出す
scan() {
  local name e p
  while read -r name _; do
    [ -n "$name" ] || continue
    for e in "${exact[@]}"; do if [ "$name" = "$e" ]; then echo "$name"; fi; done
    for p in "${prefix[@]}"; do if [[ "$name" == "$p"* ]]; then echo "$name"; fi; done
  done | sort -u
}

if [ "${1:-}" = "--self-test" ]; then
  fake_ok=$'ashiato-server v0.1.0\nsqlx v0.8.0\ntokio v1.0.0'
  [ -z "$(printf '%s\n' "$fake_ok" | scan)" ] || { echo "error: 一覧に無い名前を拾った" >&2; exit 1; }
  for hit in "${exact[0]}" "${prefix[0]}s3"; do
    got="$(printf '%s\n%s v1.2.3\n' "$fake_ok" "$hit" | scan)"
    [ "$got" = "$hit" ] || { echo "error: 一覧の名前 $hit を拾えなかった" >&2; exit 1; }
  done
  echo "OK check-offsite の自己検査（一覧 $(( ${#exact[@]} + ${#prefix[@]} )) 件）"
  exit 0
fi

hits="$(cargo tree -p ashiato-server -e normal --prefix none | scan)"
if [ -n "$hits" ]; then
  echo "error: サーバが拠点外へ書き出す部品を持っている:" >&2
  printf '  NG %s\n' $hits >&2
  exit 1
fi
echo "Scenario: サーバは拠点外へ書き出す部品を持たない"
echo "  （一覧 $(( ${#exact[@]} + ${#prefix[@]} )) 件と突き合わせた）"
