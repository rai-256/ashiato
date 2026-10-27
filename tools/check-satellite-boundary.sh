#!/usr/bin/env bash
# 本体が衛星のコードを読み込まないことの静的検査（ST29 / FR-77 / design D10）。
#
#   ./tools/check-satellite-boundary.sh              → 本体のサーバの依存とソースを見る
#   ./tools/check-satellite-boundary.sh --self-test  → 禁じる部品を含む固定の一覧で、落ちて名前が出ることを確かめる
#
# 見るのは 2 つ: 本体のサーバの依存（通常の依存のみ）に動的ライブラリの読み込み・WebAssembly の実行・
# スクリプト言語の実行の部品が無いこと / ソースに `dlopen` `LoadLibrary` `Library::new` の字面が無いこと（依存を通さない FFI の抜け道）。
# `Cargo.lock` の字面では見ない —— `libloading` は `clang-sys`（ビルドの道具）経由で既に lock にある。
set -uo pipefail
cd "$(dirname "$0")/.."

FORBIDDEN="libloading dlopen dlopen2 wasmtime wasmer wasmi extism mlua rlua rhai pyo3 deno_core v8 boa_engine abi_stable"
SOURCE_PATTERN='dlopen|LoadLibrary|Library::new'

# 依存の名前の一覧（1 行 1 名前）を標準入力から読み、禁じる部品を出す。1 つでもあれば rc=1
forbidden_in_deps() {
  local found=0 name
  while read -r name _; do
    [ -n "$name" ] || continue
    for f in $FORBIDDEN; do
      if [ "$name" = "$f" ]; then echo "  NG 禁じる部品が依存にある: $name"; found=1; fi
    done
  done
  return "$found"
}

if [ "${1:-}" = "--self-test" ]; then
  out="$(printf '%s\n' anyhow libloading axum wasmtime | forbidden_in_deps)"; rc=$?
  [ "$rc" -ne 0 ] || { echo "self-test NG: 禁じる部品を含む一覧で落ちなかった"; exit 1; }
  for n in libloading wasmtime; do
    echo "$out" | grep -q "$n" || { echo "self-test NG: $n の名前が出ていない"; exit 1; }
  done
  printf '%s\n' anyhow axum tokio | forbidden_in_deps >/dev/null || { echo "self-test NG: 無害な一覧で落ちた"; exit 1; }
  # Scenario: 読み込む部品が依存に入ると検査が落ちる
  echo "Scenario: 読み込む部品が依存に入ると検査が落ちる"
  echo "self-test OK"
  exit 0
fi

deps="$(cargo tree -p ashiato-server -e normal --target all --prefix none)" || { echo "cargo tree が失敗した" >&2; exit 2; }
echo "$deps" | awk '{print $1}' | sort -u | forbidden_in_deps || exit 1
if grep -rnE "$SOURCE_PATTERN" crates/server/src; then
  echo "  NG ソースに動的読み込みの字面がある（上）"
  exit 1
fi
# Scenario: 本体の依存に衛星のコードを読み込む部品は無い
echo "Scenario: 本体の依存に衛星のコードを読み込む部品は無い"
echo "衛星の境界 OK"
