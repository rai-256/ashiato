#!/usr/bin/env bash
# 組み立ての印（ST49 / design D5・D6）の検査。節ごとに repo を使い捨ての場所へ写し、本物の部品を組み立てて埋め込まれた印を読む。
#   tool    crates/build-stamp の関数を最小の crate から呼んで印を出す
#   server  S-01（Rust は組み立てた bin を strings して読む）。commit を進めた後・add していない変更の後で印が変わる
#   windows / android  Task 8・9 で足す
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
section="${1:-}"
# 印の形（design D5）。4 つの組み立ての検査が同じ式で見る
STAMP_RE='[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?'

case "$section" in
  tool | server) ;;
  windows | android) echo "NG: 節 $section はまだ無い（Task 8・9 で足す）"; exit 2;;
  *) echo "usage: $0 tool|server|windows|android（節が無い: '$section'）"; exit 2;;
esac

work="$(mktemp -d "${TMPDIR:-/tmp}/build-stamp.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="$work/target"
# 検査の木の中で git が commit を作れるように。呼び出し側の env は変えない
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
unset ASHIATO_BUILD_COMMIT

fail() { echo "NG: $*"; exit 1; }
need_shape() { # <ラベル> <印>
  printf '%s' "$2" | grep -Eq "^${STAMP_RE}\$" || fail "$1: 印の形が違う: '$2'"
}

if [ "$section" = tool ]; then
  mkdir -p "$work/probe/src"
  cat > "$work/probe/Cargo.toml" <<TOML
[package]
name = "probe"
version = "0.1.0"
edition = "2021"
[workspace]
[build-dependencies]
ashiato-build-stamp = { path = "$root/crates/build-stamp" }
TOML
  cat > "$work/probe/build.rs" <<RS
fn main() {
    let repo = std::path::Path::new("$root");
    let s = ashiato_build_stamp::stamp("0.1.0", &std::env::var("PROFILE").unwrap_or_default(), repo, &["crates/build-stamp"]);
    ashiato_build_stamp::emit_rerun(repo, &["crates/build-stamp"]);
    println!("cargo:rustc-env=PROBE_STAMP={s}");
}
RS
  echo 'fn main() { println!("{}", env!("PROBE_STAMP")); }' > "$work/probe/src/main.rs"
  got="$(cargo run -q --manifest-path "$work/probe/Cargo.toml")"
  need_shape tool "$got"
  head="$(git rev-parse HEAD)"
  case "$got" in "0.1.0+$head"*) ;; *) fail "tool: commit が HEAD($head)でない: $got";; esac
  echo "OK tool: $got"
  exit 0
fi

# ---- server ----
clone="$work/clone"
git clone -q --local "$root" "$clone"

# 組み立てた bin から印を読む（bin に残る印は 1 つだけ）
build_stamp() {
  (cd "$clone" && cargo build -q -p ashiato-server --bin openapi)
  strings -n 8 "$CARGO_TARGET_DIR/debug/openapi" | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?' | sort -u
}
one_stamp() { # <ラベル>
  local s n
  s="$(build_stamp)"
  n="$(printf '%s\n' "$s" | grep -c .)"
  [ "$n" = 1 ] || fail "$1: bin に残る印が $n 個: $s"
  need_shape "$1" "$s"
  printf '%s' "$s"
}

a="$(one_stamp 最初の組み立て)"
head_a="$(git -C "$clone" rev-parse HEAD)"
case "$a" in "0."*"+$head_a.debug") ;; *) fail "最初の組み立て: 印が HEAD($head_a)+.debug でない: $a";; esac

echo "== Scenario: S-01 の組み立ての印は、commit を進めた後の差分の組み立てでも新しい commit になる"
git -C "$clone" commit -q --allow-empty -m "stamp check"
head_b="$(git -C "$clone" rev-parse HEAD)"
b="$(one_stamp commit後の組み立て)"
[ "$head_b" != "$head_a" ] || fail "commit が進んでいない"
case "$b" in "0."*"+$head_b.debug") echo "OK: $a -> $b";; *) fail "commit を進めた後の印が新しい HEAD($head_b)でない: $b";; esac

echo "== Scenario: S-01 の組み立ての印は、add していないソースの変更の後の差分の組み立てで .dirty になる"
echo "// stamp check" >> "$clone/crates/server/src/lib.rs"
c="$(one_stamp 未addの変更後の組み立て)"
case "$c" in "0."*"+$head_b.dirty.debug") echo "OK: $b -> $c";; *) fail "add していない変更の後の印が .dirty.debug でない: $c";; esac
