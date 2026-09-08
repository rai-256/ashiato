#!/usr/bin/env bash
# 層の境界とライセンスの境界を機械で確かめる（製造準備 C）。
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0

# FR-77: プラグインと衛星は別プロセス。本体にコードを読み込む経路を生やさない。
# 読み込むと AGPL-3.0 の派生物になり、有料で配れなくなる。
if grep -rEn 'libloading|dlopen|LoadLibrary|Command::new\("[^"]*plugin' crates/ --include=*.rs; then
  echo "  NG 本体にコードを読み込む経路がある（FR-77 違反）"; bad=1
fi
# 収集側がサーバの内部に依存しない（契約は docs/collector-contract.md と HTTP だけ）
if grep -qE '^\s*ashiato-server' crates/collector-windows/Cargo.toml; then
  echo "  NG 収集側がサーバの内部に依存している"; bad=1
fi
# 画面がサーバの実装ではなく API を見ていること
if grep -rn "from ['\"]\.\./\.\./crates" web/src >/dev/null 2>&1; then
  echo "  NG 画面がサーバの実装を直接見ている"; bad=1
fi
[ "$bad" -eq 0 ] && echo "境界 OK" || { echo "境界 NG"; exit 1; }
