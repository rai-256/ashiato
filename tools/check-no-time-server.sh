#!/usr/bin/env bash
# 時刻サーバへの経路が、端末にも PC にも無いことの静的検査（ST05 / FR-7 / Q1 / design D7）。
#
#   ./tools/check-no-time-server.sh              → 実物（端末の main と PC の src）を見る
#   ./tools/check-no-time-server.sh --self-test  → 経路を 1 種ずつ植えた一時ディレクトリで、どれも見つかることを確かめる
#
# 見るのは 5 種類: 時刻のプロトコルの送信（部品の名前も）/ 時刻サーバの宛先 / 同期を起こす・設定する・
# 外へ問い合わせる `w32tm` の指示 / **PC の子プロセスは `w32tm` と `wevtutil` の照会だけ**（`Command::new` の相手は `program(` だけを許す。
# 引数の固定は `time_sync_*arguments_are_pinned_to_the_query` の試験が持つ。
# `sc start w32time` のような同期させる操作もここで止まる）/ 端末の子プロセス。
# **依存（`Cargo.toml` / `build.gradle.kts`）も同じ一覧で見る**（NTP の部品を足しただけで通っていた。review R20）。
# **コメント行と、Rust の最上位の `#[cfg(test)]` が付いた `mod … {` の塊（試験の値）は数えない** ——
# 出力の見本に `time.windows.com` が出てくるだけで、通信は起きない。
# 塊の終わりは行頭の `}`（rustfmt の形）。塊の後ろにある本物のコードはまた数える（review R14）。
# `#[cfg(test)]` が `mod … {` 以外（関数・欄・`mod tests;`）に付いているときは読み飛ばさない（厳しい側）。
set -uo pipefail
cd "$(dirname "$0")/.."

PATTERN='DatagramSocket|DatagramChannel|UdpSocket|[Ss]ntp|[Nn]tp[A-Z_:.-]|NTPUDPClient|TrustedTime|commons-net|play-services-time|ntp\.org|time\.google\.com|time\.windows\.com|time\.apple\.com|time\.nist\.gov|nict\.(go\.)?jp|:123([^0-9]|$)|/resync|/stripchart|/config|/computer|Command::new\((\)|[^p]|p[^r]|pr[^o]|pro[^g])|ProcessBuilder|getRuntime\(\)\.exec'

scan() {   # $@ = 見る根。当たった行を `path:行: 本文` で出す。読めなければ rc=2
  local root f files
  for root in "$@"; do
    if [ -f "$root" ]; then
      files="$root"
    else
      [ -d "$root" ] || { echo "見る場所が無い: $root" >&2; return 2; }
      files="$(find "$root" -type f \( -name '*.kt' -o -name '*.java' -o -name '*.rs' -o -name '*.xml' \
        -o -name 'Cargo.toml' -o -name '*.gradle.kts' \) | sort)" \
        || { echo "見る場所を列挙できない: $root" >&2; return 2; }
    fi
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      PAT="$PATTERN" awk -v file="$f" '
        skip == 1 { if ($0 ~ /^}/) skip = 0; next }
        pending == 1 {
          if ($0 ~ /^[[:space:]]*$/) next
          pending = 0
          if ($0 ~ /^(pub(\([a-z]+\))? )?mod [A-Za-z_][A-Za-z0-9_]* *\{/) { if ($0 !~ /}[[:space:]]*$/) skip = 1; next }
        }
        /^#\[cfg\(test\)\][[:space:]]*$/ { pending = 1; next }
        /^[[:space:]]*(\/\/|\*|\/\*|#)/ { next }
        $0 ~ ENVIRON["PAT"] { printf "%s:%d: %s\n", file, FNR, $0 }
      ' "$f" || { echo "読めない: $f" >&2; return 2; }
    done <<< "$files"
  done
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp -d)"; trap 'chmod -R u+rwx "$tmp" 2>/dev/null; rm -rf "$tmp"' EXIT
  fail() { echo "自己検査 FAIL: $*"; exit 1; }
  # 経路の無い場所: コメントと、最上位の試験の塊の中は数えない
  mkdir -p "$tmp/clean"
  printf 'fn main() {}\n// UdpSocket は書かない（コメントは数えない）\n#[cfg(test)]\nmod tests {\n    const S: &str = "time.windows.com";\n    fn f() {\n        let _ = "/resync";\n    }\n}\n' > "$tmp/clean/a.rs"
  out="$(scan "$tmp/clean")" || fail "経路の無い場所で検査が落ちた（rc=$?）"
  [ -z "$out" ] || fail "経路の無い場所で当たった: $out"
  # 植える経路。**1 つずつ別の場所に植え、どれも止まる**ことを見る
  plant() {   # $1 = 名前, $2 = ファイル名, $3 = 中身
    mkdir -p "$tmp/planted-$1"
    cp "$tmp/clean/a.rs" "$tmp/planted-$1/a.rs"
    printf '%s\n' "$3" > "$tmp/planted-$1/$2"
    out="$(scan "$tmp/planted-$1")" || fail "$1: 検査が落ちた"
    [ -n "$out" ] || fail "植えた経路（$1）を見つけられなかった"
  }
  plant datagram B.kt 'val s = DatagramSocket()'
  plant sntp B.kt 'val c = SntpClient()'
  plant ntp-trusted B.java 'NtpTrustedTime t = NtpTrustedTime.getInstance(ctx);'
  plant udp b.rs 'let s = std::net::UdpSocket::bind("0.0.0.0:0");'
  plant resync b.rs 'Command::new("w32tm").args(["/resync"]).status();'
  plant host b.rs 'const H: &str = "time.windows.com";'
  plant ntp-pool B.kt 'val h = "pool.ntp.org"'
  plant port b.rs 'let a = "192.0.2.1:123";'
  plant xml c.xml '<string name="t">time.google.com</string>'
  # 一覧の外だった経路（review R20 の 5 種）と、依存に足した NTP の部品
  plant commons-ntp B.kt 'val t = org.apache.commons.net.ntp.NTPUDPClient().getTime(InetAddress.getByName("ntp.nict.jp"))'
  plant channel B.kt 'java.nio.channels.DatagramChannel.open().send(buf, InetSocketAddress("ntp.nict.jp", 123))'
  plant stripchart b.rs 'Command::new("w32tm").args(["/stripchart", "/computer:time.nist.gov", "/samples:1"]).status();'
  plant sc-start b.rs 'Command::new("sc").args(["start", "w32time"]).status();'
  plant sntpc b.rs 'let t = sntpc::simple_get_time(("ntp.nict.jp", 123), &sock);'
  plant cargo-dep Cargo.toml 'sntpc = "0.5"'
  plant gradle-dep build.gradle.kts '    implementation("commons-net:commons-net:3.11.1")'
  plant gradle-trusted build.gradle.kts '    implementation("com.google.android.gms:play-services-time:16.0.1")'
  plant process B.kt 'ProcessBuilder("toybox", "ntpd").start()'
  # 試験の塊の**後ろ**にある本物のコードは数える（最上位の #[cfg(test)] 以降を全部読み飛ばしていた。review R14）
  plant after-tests d.rs "$(printf '#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n\nfn sync() {\n    let _ = std::net::UdpSocket::bind("0.0.0.0:0");\n}')"
  # `#[cfg(test)]` が mod の塊以外に付いていても、その後ろは読み飛ばさない
  plant cfg-fn e.rs "$(printf '#[cfg(test)]\nfn helper() {}\nfn sync() { let _ = "/resync"; }')"
  # 読めないファイルは「経路が無い」ではなく失敗にする（awk の失敗を拾う）
  if [ "$(id -u)" != 0 ]; then
    mkdir -p "$tmp/unreadable"
    printf 'fn main() {}\n' > "$tmp/unreadable/a.rs"; chmod 000 "$tmp/unreadable/a.rs"
    if scan "$tmp/unreadable" >/dev/null 2>&1; then fail "読めないファイルを通した"; fi
  fi
  # 許してよい子プロセスは `w32tm` の照会（`Command::new(program(...))`）だけ
  mkdir -p "$tmp/allowed"
  printf 'fn c() { let mut c = Command::new(program(std::env::var_os("SystemRoot"))); }\n' > "$tmp/allowed/a.rs"
  out="$(scan "$tmp/allowed")" || fail "照会の子プロセスで検査が落ちた"
  [ -z "$out" ] || fail "照会の子プロセスを止めた: $out"
  echo "自己検査 OK（経路の無い場所と照会の子プロセスは通り、植えた経路は 20 種とも止まり、読めないものは落ちる）"
  exit 0
fi

echo "Scenario: 外部の時刻サーバへ問い合わせない"
echo "Scenario: PC は外部の時刻サーバへ問い合わせない"
hits="$(scan collector-android/app/src/main crates/collector-windows/src \
  collector-android/app/build.gradle.kts collector-android/build.gradle.kts \
  crates/collector-windows/Cargo.toml Cargo.toml)" || exit 2
if [ -n "$hits" ]; then
  echo "時刻サーバへの経路がある:"; echo "$hits"; exit 1
fi
echo "時刻サーバへの経路は無い"
