#!/usr/bin/env bash
# 組み立ての印（ST49 / design D5・D6）の検査。節ごとに repo を使い捨ての場所へ写し、本物の部品を組み立てて埋め込まれた印を読む。
#   tool    crates/build-stamp の関数を最小の crate から呼んで印を出す
#   server  S-01（Rust は組み立てた bin を strings して読む）。commit を進めた後・add していない変更の後で印が変わる
#   windows C-02（release で組み立てた rlib（libashiato_collector_windows.rlib）を strings して読む。Linux の bin は収集の本体を持たず印が残らない）。commit を進めた後・add していない変更の後で印が変わる
#   android C-01（gradle で :app:generate{Debug,Release}BuildConfig を走らせ、生成された BuildConfig.java を読む）。--configuration-cache --build-cache を付ける。
#           環境変数 ASHIATO_BUILD_COMMIT が git の HEAD より先に使われることも見る（design D5）
# 版番号は決め打ちしない: Rust の 2 つは Cargo.toml の [workspace.package] の version、C-01 は build.gradle.kts の versionName と比べる
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
section="${1:-}"
# 印の形（design D5）。4 つの組み立ての検査が同じ式で見る
STAMP_RE='[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?'

case "$section" in
  tool | server | windows | android) ;;
  *) echo "usage: $0 tool|server|windows|android（節が無い: '$section'）"; exit 2;;
esac

work="$(mktemp -d "${TMPDIR:-/tmp}/build-stamp.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="$work/target"
# 検査の木の中で git が commit を作れるように。呼び出し側の env は変えない
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
unset ASHIATO_BUILD_COMMIT

fail() { echo "NG: $*" >&2; exit 1; }
# 正規表現の中で文字どおりに読むよう、版番号の . や + を逃がす
re_quote() { printf '%s' "$1" | sed 's/[][\\.^$*+?(){}|/]/\\&/g'; }
# 宣言した版番号（design D5 の出所）
cargo_version="$(sed -n '/^\[workspace\.package\]/,/^\[/{s/^version *= *"\([^"]*\)".*/\1/p}' Cargo.toml | head -n1)"
[ -n "$cargo_version" ] || fail "Cargo.toml の [workspace.package] に version が無い"
# 組み立てた成果物から印を拾う式（版番号は宣言した値）
rust_stamp_re="$(re_quote "$cargo_version")\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?"
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

if [ "$section" = windows ]; then
  clone="$work/clone"
  git clone -q --local "$root" "$clone"
  # release で組み立てた rlib から印を読む（Linux の bin は収集の本体を持たず印が残らない。印は 1 つだけ。release は .debug を持たない）
  win_stamp() { # <ラベル>
    local s n
    (cd "$clone" && cargo build -q --release -p ashiato-collector-windows)
    s="$(strings -n 8 "$CARGO_TARGET_DIR/release/libashiato_collector_windows.rlib" | grep -Eo "$rust_stamp_re" | sort -u)"
    n="$(printf '%s\n' "$s" | grep -c .)"
    [ "$n" = 1 ] || fail "$1: 印が $n 個: $s"
    need_shape "$1" "$s"
    printf '%s' "$s"
  }
  a="$(win_stamp 最初の組み立て)"
  head_a="$(git -C "$clone" rev-parse HEAD)"
  case "$a" in "$cargo_version+$head_a") ;; *) fail "最初の組み立て: 印が $cargo_version+HEAD($head_a)でない: $a";; esac

  echo "== Scenario: PC の収集アプリの組み立ての印は、commit を進めた後の差分の組み立てでも新しい commit になる"
  git -C "$clone" commit -q --allow-empty -m "stamp check"
  head_b="$(git -C "$clone" rev-parse HEAD)"
  b="$(win_stamp commit後の組み立て)"
  [ "$head_b" != "$head_a" ] || fail "commit が進んでいない"
  case "$b" in "$cargo_version+$head_b") echo "OK: $a -> $b";; *) fail "commit を進めた後の印が新しい HEAD($head_b)でない: $b";; esac

  echo "== Scenario: PC の収集アプリの印は、add していないソースの変更の後に .dirty になり、release では .debug を持たない"
  echo "// stamp check" >> "$clone/crates/collector-windows/src/lib.rs"
  c="$(win_stamp 未addの変更後の組み立て)"
  case "$c" in "$cargo_version+$head_b.dirty") echo "OK: $b -> $c";; *) fail "add していない変更の後の印が .dirty（.debug なし）でない: $c";; esac
  exit 0
fi

if [ "$section" = android ]; then
  clone="$work/clone"
  git clone -q --local "$root" "$clone"
  # 宣言した版番号（build.gradle.kts の versionName）
  av="$(sed -n 's/^ *versionName *= *"\([^"]*\)".*/\1/p' collector-android/app/build.gradle.kts | head -n1)"
  [ -n "$av" ] || fail "collector-android/app/build.gradle.kts に versionName が無い"
  # 使い捨ての木の中の gradle。構成キャッシュとビルドキャッシュを付けても、印が古いまま残らないことを見る
  android_stamps() { # <ラベル> -> "<debug> <release>"
    (cd "$clone/collector-android" && ./gradlew -q --configuration-cache --build-cache :app:generateDebugBuildConfig :app:generateReleaseBuildConfig >&2) \
      || fail "$1: gradle が落ちた"
    local out="$clone/collector-android/app/build/generated/source/buildConfig" d r
    d="$(grep -Eo 'COMPONENT_VERSION = "[^"]*"' "$out/debug/dev/ashiato/collector/BuildConfig.java" | cut -d'"' -f2)"
    r="$(grep -Eo 'COMPONENT_VERSION = "[^"]*"' "$out/release/dev/ashiato/collector/BuildConfig.java" | cut -d'"' -f2)"
    need_shape "$1 debug" "$d"
    need_shape "$1 release" "$r"
    printf '%s %s' "$d" "$r"
  }
  # `read <<<"$(…)"` は置き換えの中の失敗を拾わないので、一度変数に受けて止める（gradle が落ちた理由で止まる）
  head_a="$(git -C "$clone" rev-parse HEAD)"
  got="$(android_stamps 最初の組み立て)" || exit 1
  read -r da ra <<<"$got"
  case "$da" in "$av+$head_a.debug") ;; *) fail "最初の組み立て: debug の印が $av+HEAD($head_a).debug でない: $da";; esac

  echo "== Scenario: 携帯の収集アプリの組み立ての印は、組み立てのたびに作り直される"
  git -C "$clone" commit -q --allow-empty -m "stamp check"
  head_b="$(git -C "$clone" rev-parse HEAD)"
  [ "$head_b" != "$head_a" ] || fail "commit が進んでいない"
  got="$(android_stamps commit後の組み立て)" || exit 1
  read -r db rb <<<"$got"
  case "$db" in "$av+$head_b.debug") echo "OK: $da -> $db";; *) fail "commit を進めた後の debug の印が新しい HEAD($head_b)でない: $db";; esac

  echo "== Scenario: 携帯の収集アプリの release の印は .debug を持たず、debug の印は持つ"
  case "$rb" in "$av+$head_b") echo "OK: debug=$db release=$rb";; *) fail "release の印が $av+HEAD($head_b)だけでない: $rb";; esac
  case "$rb" in *.debug*) fail "release の印が .debug を持つ: $rb";; esac

  echo "== Scenario: 携帯の収集アプリの印は、add していないソースの変更の後に .dirty になる"
  echo "// stamp check" >> "$clone/collector-android/app/src/main/kotlin/dev/ashiato/collector/Config.kt"
  got="$(android_stamps 未addの変更後の組み立て)" || exit 1
  read -r dc rc <<<"$got"
  case "$dc" in "$av+$head_b.dirty.debug") ;; *) fail "add していない変更の後の debug の印が .dirty.debug でない: $dc";; esac
  case "$rc" in "$av+$head_b.dirty") echo "OK: $db -> $dc / release $rc";; *) fail "add していない変更の後の release の印が .dirty（.debug なし）でない: $rc";; esac

  echo "-- 環境変数 ASHIATO_BUILD_COMMIT が空でなければ git の HEAD より先に使われる（design D5）"
  env_commit=0123456789abcdef0123456789abcdef01234567
  got="$(ASHIATO_BUILD_COMMIT="$env_commit" android_stamps 環境変数の組み立て)" || exit 1
  read -r de re <<<"$got"
  case "$de" in "$av+$env_commit.dirty.debug") ;; *) fail "環境変数の commit が debug の印に入らない: $de";; esac
  case "$re" in "$av+$env_commit.dirty") echo "OK: env -> $de / release $re";; *) fail "環境変数の commit が release の印に入らない: $re";; esac
  got="$(ASHIATO_BUILD_COMMIT="" android_stamps 空の環境変数の組み立て)" || exit 1
  read -r dz _ <<<"$got"
  case "$dz" in "$av+$head_b.dirty.debug") echo "OK: 空の env -> $dz";; *) fail "空の環境変数で git の HEAD に戻らない: $dz";; esac
  exit 0
fi

# ---- server ----
clone="$work/clone"
git clone -q --local "$root" "$clone"

# 組み立てた bin から印を読む（bin に残る印は 1 つだけ）
build_stamp() {
  (cd "$clone" && cargo build -q -p ashiato-server --bin openapi)
  strings -n 8 "$CARGO_TARGET_DIR/debug/openapi" | grep -Eo "$rust_stamp_re" | sort -u
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
case "$a" in "$cargo_version+$head_a.debug") ;; *) fail "最初の組み立て: 印が $cargo_version+HEAD($head_a).debug でない: $a";; esac

echo "== Scenario: S-01 の組み立ての印は、commit を進めた後の差分の組み立てでも新しい commit になる"
git -C "$clone" commit -q --allow-empty -m "stamp check"
head_b="$(git -C "$clone" rev-parse HEAD)"
b="$(one_stamp commit後の組み立て)"
[ "$head_b" != "$head_a" ] || fail "commit が進んでいない"
case "$b" in "$cargo_version+$head_b.debug") echo "OK: $a -> $b";; *) fail "commit を進めた後の印が新しい HEAD($head_b)でない: $b";; esac

echo "== Scenario: S-01 の組み立ての印は、add していないソースの変更の後の差分の組み立てで .dirty になる"
echo "// stamp check" >> "$clone/crates/server/src/lib.rs"
c="$(one_stamp 未addの変更後の組み立て)"
case "$c" in "$cargo_version+$head_b.dirty.debug") echo "OK: $b -> $c";; *) fail "add していない変更の後の印が .dirty.debug でない: $c";; esac
