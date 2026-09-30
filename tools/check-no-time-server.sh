#!/usr/bin/env bash
# 時刻サーバへの経路が、端末にも PC にも無いことの静的検査（ST05 / FR-7 / Q1 / design D7）。
#
#   ./tools/check-no-time-server.sh              → 実物（端末の main と PC の src）を見る
#   ./tools/check-no-time-server.sh --self-test  → 経路を 1 つ植えた一時ディレクトリで rc=1 になることを確かめる
#
# 見るのは 3 種類: 時刻のプロトコルの送信 / 時刻サーバの宛先 / 同期を起こす指示（`w32tm /resync`）。
# **コメント行と、Rust の最上位の `#[cfg(test)]` 以降（試験の値）は数えない** ——
# 出力の見本に `time.windows.com` が出てくるだけで、通信は起きない。
set -uo pipefail
cd "$(dirname "$0")/.."

PATTERN='DatagramSocket|SntpClient|NtpTrustedTime|UdpSocket|ntp\.org|time\.google\.com|time\.windows\.com|time\.apple\.com|:123([^0-9]|$)|/resync'

scan() {   # $@ = 見る根。当たった行を `path:行: 本文` で出す
  local root f
  for root in "$@"; do
    [ -d "$root" ] || { echo "見る場所が無い: $root" >&2; return 2; }
    while IFS= read -r f; do
      PAT="$PATTERN" awk -v file="$f" '
        /^#\[cfg\(test\)\]/ { exit }
        /^[[:space:]]*(\/\/|\*|\/\*)/ { next }
        $0 ~ ENVIRON["PAT"] { printf "%s:%d: %s\n", file, FNR, $0 }
      ' "$f"
    done < <(find "$root" -type f \( -name '*.kt' -o -name '*.java' -o -name '*.rs' -o -name '*.xml' \) | sort)
  done
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/clean" "$tmp/planted"
  printf 'fn main() {}\n// UdpSocket は書かない（コメントは数えない）\n#[cfg(test)]\nmod t { const S: &str = "time.windows.com"; }\n' > "$tmp/clean/a.rs"
  cp "$tmp/clean/a.rs" "$tmp/planted/a.rs"
  printf 'val s = DatagramSocket()\n' > "$tmp/planted/B.kt"
  [ -z "$(scan "$tmp/clean")" ] || { echo "自己検査 FAIL: 経路の無い場所で当たった"; exit 1; }
  [ -n "$(scan "$tmp/planted")" ] || { echo "自己検査 FAIL: 植えた経路を見つけられなかった"; exit 1; }
  echo "自己検査 OK（経路の無い場所は通り、植えた経路は止まる）"
  exit 0
fi

echo "Scenario: 外部の時刻サーバへ問い合わせない"
echo "Scenario: PC は外部の時刻サーバへ問い合わせない"
hits="$(scan collector-android/app/src/main crates/collector-windows/src)" || exit 2
if [ -n "$hits" ]; then
  echo "時刻サーバへの経路がある:"; echo "$hits"; exit 1
fi
echo "時刻サーバへの経路は無い"
