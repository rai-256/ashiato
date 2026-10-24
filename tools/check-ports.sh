#!/usr/bin/env bash
# worktree ごとのポートの規則（tools/ports.sh）の自己検査。
#
#   ./tools/check-ports.sh --self-test  → 一時の repo と worktree を作り、どの 2 つの木も DB とサーバの port が被らないことを確かめる
#
# 見るのは: main の checkout は 55432 / `<main>-st<NN>` は 55500+NN / `-verify` は 55499 /
# それ以外（`-ch<NN>`・`-up-st<NN>`・`-rec-st<NN>` など）は台帳で被らない番号・同じ木は何度呼んでも同じ番号・
# 消えた worktree の番号は使い回す・並んで呼んでも被らない・ASHIATO_DB_PORT の上書き。
# 実測 2026-10-21: `-ch01` と `-ch02` が 55432 に落ち、確認バッチの DB と取り合って CH01・CH02 が止まった。
set -euo pipefail
cd "$(dirname "$0")/.."
[[ ${1:-} == --self-test ]] || { echo "使い方: $0 --self-test" >&2; exit 2; }

src="$PWD/tools/ports.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

git -C "$tmp" init -q demo
git -C "$tmp/demo" -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
mk() { git -C "$tmp/demo" worktree add -q --detach "$tmp/$1"; mkdir -p "$tmp/$1/tools"; cp "$src" "$tmp/$1/tools/"; }
mkdir -p "$tmp/demo/tools"; cp "$src" "$tmp/demo/tools/"
ports() { env -u ASHIATO_DB_PORT -u ASHIATO_HTTP_PORT bash -c '. "$0/tools/ports.sh" && echo "$ASHIATO_DB_PORT $ASHIATO_HTTP_PORT"' "$tmp/$1"; }
subnet() { env -u ASHIATO_DB_PORT -u ASHIATO_HTTP_PORT bash -c '. "$0/tools/ports.sh" && ashiato_db_subnet' "$tmp/$1"; }
want() {  # want <説明> <実際> <期待>
  if [[ $2 == "$3" ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1（$2。期待は $3）" >&2; fail=1; fi
}

for w in demo-st05 demo-verify demo-ch01 demo-ch02 demo-up-st05 demo-rec-st22 demo-st06-t7; do mk "$w"; done
want "main の checkout は 55432 / 18787" "$(ports demo)" "55432 18787"
want "下流の Story は 55500+NN / 19000+2×NN" "$(ports demo-st05)" "55505 19010"
want "確認バッチは 55499 / 18787" "$(ports demo-verify)" "55499 18787"
want "change の 1 本目は台帳の 0 番" "$(ports demo-ch01)" "55600 19200"
want "change の 2 本目は台帳の 1 番（1 本目と被らない）" "$(ports demo-ch02)" "55601 19202"
want "同じ木は何度呼んでも同じ番号" "$(ports demo-ch01)" "55600 19200"
want "上流の -up-st05 は下流の -st05 と被らない" "$(ports demo-up-st05)" "55602 19204"
want "名前が -st<NN> を含んでも <main>-st<NN> でなければ台帳" "$(ports demo-st06-t7 | cut -d' ' -f1)$(ports demo-rec-st22 | cut -d' ' -f1)" "5560355604"
want "ASHIATO_DB_PORT / ASHIATO_HTTP_PORT の上書きが勝つ" \
  "$(ASHIATO_DB_PORT=1 ASHIATO_HTTP_PORT=2 bash -c '. "$0/tools/ports.sh" && echo "$ASHIATO_DB_PORT $ASHIATO_HTTP_PORT"' "$tmp/demo-ch01")" "1 2"

git -C "$tmp/demo" worktree remove --force "$tmp/demo-ch01"
mk demo-ch09
want "消えた worktree の番号を使い回す" "$(ports demo-ch09)" "55600 19200"
want "残った worktree の番号は変わらない" "$(ports demo-ch02)" "55601 19202"

# DB の subnet（design D5）。期待値は printf で組む（追跡するファイルに点区切りの 4 つ組を書かない）
echo "Scenario: どの 2 つの木も、DB の subnet が被らない"
all=$(for w in demo demo-st05 demo-verify demo-ch02 demo-up-st05 demo-rec-st22; do subnet "$w"; echo; done; subnet demo-ch09; echo)
want "いまある 7 つの木の subnet が全部違う" "$(sort -u <<<"$all" | grep -c .)" "7"
want "どの subnet も /24 で終わる" "$(grep -c '/24$' <<<"$all")" "7"
want "main の checkout の subnet は off = 32 の値" "$(subnet demo)" "$(printf '10.%d.%d.0/24' 200 32)"
want "範囲の外の port では subnet を出さない" "$(ASHIATO_DB_PORT=1 bash -c '. "$0/tools/ports.sh" && ashiato_db_subnet' "$tmp/demo")" ""

# 並んで呼んでも被らない（台帳は flock で 1 本ずつ書く）
for i in $(seq 10 19); do mk "demo-par$i"; done
for i in $(seq 10 19); do ports "demo-par$i" >"$tmp/par$i" & done
wait
want "並んで呼んだ 10 本がすべて別の番号" "$(cat "$tmp"/par1? | sort -u | wc -l)" "10"

# 台帳が尽きたら 55432 に落とさず、非 0 で止まる
seq 0 99 | while read -r n; do printf '%s\t%s\n' "$n" "$tmp/demo"; done >"$tmp/demo/.git/ashiato-ports"
mk demo-full
if out=$(ports demo-full 2>/dev/null); then got="rc=0 $out"; else got="rc!=0"; fi
want "台帳が尽きたら非 0（55432 に落とさない）" "$got" "rc!=0"

[[ $fail == 0 ]] && echo "self-test OK" || { echo "self-test FAILED" >&2; exit 1; }
