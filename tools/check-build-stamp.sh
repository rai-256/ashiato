#!/usr/bin/env bash
# 組み立ての印（ST49 / design D5・D6）の検査。節ごとに repo を使い捨ての場所へ写し、本物の部品を組み立てて埋め込まれた印を読む。
#   tool    crates/build-stamp の関数を最小の crate から呼んで印を出す
#   server  S-01（Rust は組み立てた bin を strings して読む）。commit を進めた後・add していない変更の後で印が変わる
#   windows C-02（release で組み立てた rlib（libashiato_collector_windows.rlib）を strings して読む。Linux の bin は収集の本体を持たず印が残らない）。commit を進めた後・add していない変更の後で印が変わる
#   android C-01（gradle で :app:generate{Debug,Release}BuildConfig を走らせ、生成された BuildConfig.java を読む）。--configuration-cache --build-cache を付ける。
#           環境変数 ASHIATO_BUILD_COMMIT が git の HEAD より先に使われることも見る（design D5）
# 各節の終わりで「部品のソースの道」（design D5）も本物の組み立てで見る: ほかの部品の木だけの変更は .dirty にせず、
# 部品の道の別の場所（S-01 の migrations/・crates/build-stamp、C-02 の crates/build-stamp）の変更は .dirty にする（review/code.md R16）
# 版番号は決め打ちしない: 4 つとも Cargo.toml の [workspace.package] の version と比べる
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
section="${1:-}"
# 印の形（design D5）。4 つの組み立ての検査が同じ式で見る
STAMP_RE='[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?'

case "$section" in
  tool | server | windows | android | release) ;;
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
exact_stamp_in() { # <ラベル> <正規表現> <ファイルまたはディレクトリ>
  local label="$1" pattern="$2" path="$3" stamps count
  stamps="$(grep -RhoE "$pattern" "$path" 2>/dev/null | sort -u || true)"
  count="$(printf '%s\n' "$stamps" | grep -c . || true)"
  [ "$count" = 1 ] || fail "$label: 印が $count 個: $stamps"
  printf '%s' "$stamps"
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
  git clone -q --local --no-hardlinks "$root" "$clone"
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

  echo "-- 部品のソースの道: S-01 の木の変更は .dirty にせず、crates/build-stamp の変更は .dirty にする（design D5）"
  git -C "$clone" commit -q -am "stamp check dirty"
  head_c="$(git -C "$clone" rev-parse HEAD)"
  echo "// stamp check" >> "$clone/crates/server/src/lib.rs"
  d="$(win_stamp ほかの部品の変更後の組み立て)"
  case "$d" in "$cargo_version+$head_c") echo "OK: crates/server の変更 -> $d";; *) fail "ほかの部品（crates/server）の変更で印が変わった: $d";; esac
  echo "// stamp check" >> "$clone/crates/build-stamp/src/lib.rs"
  e="$(win_stamp build-stampの変更後の組み立て)"
  case "$e" in "$cargo_version+$head_c.dirty") echo "OK: crates/build-stamp の変更 -> $e";; *) fail "crates/build-stamp の変更の後の印が .dirty でない: $e";; esac
  exit 0
fi

if [ "$section" = android ]; then
  clone="$work/clone"
  git clone -q --local --no-hardlinks "$root" "$clone"
  # 宣言した版番号（根の Cargo.toml）
  av="$cargo_version"
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

  echo "-- 部品のソースの道: collector-android の外の変更は .dirty にしない（design D5）"
  git -C "$clone" commit -q -am "stamp check dirty"
  head_c="$(git -C "$clone" rev-parse HEAD)"
  echo "// stamp check" >> "$clone/crates/server/src/lib.rs"
  got="$(android_stamps ほかの部品の変更後の組み立て)" || exit 1
  read -r dd _ <<<"$got"
  case "$dd" in "$av+$head_c.debug") echo "OK: crates/server の変更 -> $dd";; *) fail "ほかの部品（crates/server）の変更で印が変わった: $dd";; esac
  exit 0
fi

# release は 4 部品を同じ出所・同じ ASHIATO_BUILD_VERSION で組み立てる入口。
if [ "$section" = release ]; then
  clone="$work/clone"
  git clone -q --local --no-hardlinks "$root" "$clone"
  expected_version="$cargo_version"
  if [ -n "${ASHIATO_BUILD_VERSION:-}" ]; then
    expected_version="$ASHIATO_BUILD_VERSION"
    (cd "$root" && tools/release/version.sh check "$expected_version" >/dev/null)
  fi
  release_stamp_re="$(re_quote "$expected_version")\\+([0-9a-f]{40}|unknown)(\\.dirty)?(\\.debug)?"
  head="$(git -C "$clone" rev-parse HEAD)"
  (cd "$clone/web" && NPM_CONFIG_CACHE="$work/npm-cache" npm ci --ignore-scripts >/dev/null && npm run build >/dev/null)
  (cd "$clone" && cargo build -q --release -p ashiato-server --bin ashiato-server)
  (cd "$clone" && cargo build -q --release -p ashiato-collector-windows)
  (cd "$clone/collector-android" && ./gradlew -q :app:assembleRelease)
  web_stamp="$(exact_stamp_in "画面の release 印" "$release_stamp_re" "$clone/web/dist")"
  [ "$web_stamp" = "$expected_version+$head" ] || fail "画面の release 印が違う: $web_stamp"
  server_stamp="$(strings -n 8 "$CARGO_TARGET_DIR/release/ashiato-server" | grep -Eo "$release_stamp_re" | sort -u)"
  [ "$server_stamp" = "$expected_version+$head" ] || fail "サーバの release 印が違う: $server_stamp"
  windows_stamp="$(strings -n 8 "$CARGO_TARGET_DIR/release/libashiato_collector_windows.rlib" | grep -Eo "$release_stamp_re" | sort -u)"
  [ "$windows_stamp" = "$expected_version+$head" ] || fail "PC の release 印が違う: $windows_stamp"
  apk="$(find "$clone/collector-android/app/build/outputs/apk/release" -maxdepth 1 -name '*.apk' -print -quit)"
  apk_info="$($root/tools/release/check-artifacts.sh apk-version "$apk")" || fail "携帯の release 版番号を読めない"
  grep -Fq "versionName='$expected_version'" <<<"$apk_info" || fail "携帯の release 版番号が出所と違う: $apk_info"
  release_build_config="$clone/collector-android/app/build/generated/source/buildConfig/release/dev/ashiato/collector/BuildConfig.java"
  android_stamp="$(exact_stamp_in "携帯の release 印" "$release_stamp_re" "$release_build_config")"
  [ "$android_stamp" = "$expected_version+$head" ] || fail "携帯の release 印が違う: $android_stamp"
  [ -z "$(git -C "$clone" status --porcelain)" ] || fail "release 組み立て後の git status が空でない"
  echo "== Scenario: 渡した版番号は、4 つの部品の印に同じ値で入り、印は .dirty を持たない"

  no_env="$work/no-env"
  git clone -q --local --no-hardlinks "$root" "$no_env"
  no_env_head="$(git -C "$no_env" rev-parse HEAD)"
  (cd "$no_env/web" && NPM_CONFIG_CACHE="$work/npm-cache-no-env" npm ci --ignore-scripts >/dev/null && env -u ASHIATO_BUILD_VERSION npm run build >/dev/null)
  (cd "$no_env" && env -u ASHIATO_BUILD_VERSION cargo build -q --release -p ashiato-server --bin ashiato-server)
  (cd "$no_env" && env -u ASHIATO_BUILD_VERSION cargo build -q --release -p ashiato-collector-windows)
  (cd "$no_env/collector-android" && env -u ASHIATO_BUILD_VERSION ./gradlew -q :app:assembleRelease)
  no_env_web="$(exact_stamp_in "env 無しの画面の印" "$(re_quote "$cargo_version")\\+([0-9a-f]{40}|unknown)(\\.dirty)?(\\.debug)?" "$no_env/web/dist")"
  [ "$no_env_web" = "$cargo_version+$no_env_head" ] || fail "env 無しの画面の印が出所と違う: $no_env_web"
  no_env_server="$(strings -n 8 "$CARGO_TARGET_DIR/release/ashiato-server" | grep -Eo "$rust_stamp_re" | sort -u)"
  [ "$no_env_server" = "$cargo_version+$no_env_head" ] || fail "env 無しのサーバの印が出所と違う: $no_env_server"
  no_env_windows="$(strings -n 8 "$CARGO_TARGET_DIR/release/libashiato_collector_windows.rlib" | grep -Eo "$rust_stamp_re" | sort -u)"
  [ "$no_env_windows" = "$cargo_version+$no_env_head" ] || fail "env 無しの PC の印が出所と違う: $no_env_windows"
  no_env_apk="$(find "$no_env/collector-android/app/build/outputs/apk/release" -maxdepth 1 -name '*.apk' -print -quit)"
  no_env_apk_info="$($root/tools/release/check-artifacts.sh apk-version "$no_env_apk")" || fail "env 無しの携帯の版番号を読めない"
  grep -Fq "versionName='$cargo_version'" <<<"$no_env_apk_info" || fail "env 無しの携帯の版が出所と違う: $no_env_apk_info"
  no_env_build_config="$no_env/collector-android/app/build/generated/source/buildConfig/release/dev/ashiato/collector/BuildConfig.java"
  no_env_android="$(exact_stamp_in "env 無しの携帯の印" "$(re_quote "$cargo_version")\\+([0-9a-f]{40}|unknown)(\\.dirty)?(\\.debug)?" "$no_env_build_config")"
  [ "$no_env_android" = "$cargo_version+$no_env_head" ] || fail "env 無しの携帯の印が出所と違う: $no_env_android"
  echo "== Scenario: 版番号を渡さない組み立てでは、4 つの部品の版番号が出所の版と同じ"

  if (cd "$clone" && ASHIATO_BUILD_VERSION=0.5.0-beta.1 cargo build -q --release -p ashiato-server --bin ashiato-server); then
    fail "出所と違う版番号の組み立てが成功した"
  fi
  echo "== Scenario: 出所の版と違う版番号を渡すと、組み立てが失敗する"

  for invalid in 0.4.0-rc.1 0.4.0-beta.99 0.4.0-beta.0 0.4.0-dev.2026111; do
    if (cd "$clone" && ASHIATO_BUILD_VERSION="$invalid" cargo build -q --release -p ashiato-server --bin ashiato-server); then
      fail "形が違う版番号の組み立てが成功した: $invalid"
    fi
  done
  echo "== Scenario: 決めた形でない版番号を渡すと、組み立てが失敗する"

  source_clone="$work/source-version"
  git clone -q --local --no-hardlinks "$root" "$source_clone"
  sed -i 's/^version = "[^"]*"/version = "0.9.0"/' "$source_clone/Cargo.toml"
  git -C "$source_clone" add Cargo.toml && git -C "$source_clone" commit -q -m source-version
  source_head="$(git -C "$source_clone" rev-parse HEAD)"
  (cd "$source_clone/web" && NPM_CONFIG_CACHE="$work/npm-cache-source" npm ci --ignore-scripts >/dev/null && env -u ASHIATO_BUILD_VERSION npm run build >/dev/null)
  (cd "$source_clone/collector-android" && env -u ASHIATO_BUILD_VERSION ./gradlew -q :app:assembleRelease)
  source_web="$(exact_stamp_in "出所変更後の画面の印" "0\\.9\\.0\\+([0-9a-f]{40}|unknown)(\\.dirty)?(\\.debug)?" "$source_clone/web/dist")"
  [ "$source_web" = "0.9.0+$source_head" ] || fail "出所変更後の画面の印が 0.9.0 でない: $source_web"
  source_apk="$(find "$source_clone/collector-android/app/build/outputs/apk/release" -maxdepth 1 -name '*.apk' -print -quit)"
  source_apk_info="$($root/tools/release/check-artifacts.sh apk-version "$source_apk")" || fail "出所変更後の携帯の版番号を読めない"
  grep -Fq "versionName='0.9.0'" <<<"$source_apk_info" || fail "出所変更後の携帯の版が 0.9.0 でない: $source_apk_info"
  source_build_config="$source_clone/collector-android/app/build/generated/source/buildConfig/release/dev/ashiato/collector/BuildConfig.java"
  source_android="$(exact_stamp_in "出所変更後の携帯の印" "0\\.9\\.0\\+([0-9a-f]{40}|unknown)(\\.dirty)?(\\.debug)?" "$source_build_config")"
  [ "$source_android" = "0.9.0+$source_head" ] || fail "出所変更後の携帯の印が 0.9.0 でない: $source_android"
  echo "== Scenario: 携帯と画面の印の版番号も、根の出所の版から読む"

  dirty_clone="$work/dirty-source"
  git clone -q --local --no-hardlinks "$root" "$dirty_clone"
  sed -i 's/^version = "[^"]*"/version = "0.9.0"/' "$dirty_clone/Cargo.toml"
  dirty_head="$(git -C "$dirty_clone" rev-parse HEAD)"
  (cd "$dirty_clone/web" && NPM_CONFIG_CACHE="$work/npm-cache-dirty" npm ci --ignore-scripts >/dev/null && env -u ASHIATO_BUILD_VERSION npm run build >/dev/null)
  (cd "$dirty_clone/collector-android" && env -u ASHIATO_BUILD_VERSION ./gradlew -q :app:assembleRelease)
  dirty_web="$(exact_stamp_in "未 commit の出所変更後の画面の印" "0\\.9\\.0\\+([0-9a-f]{40}|unknown)\\.dirty" "$dirty_clone/web/dist")"
  [ "$dirty_web" = "0.9.0+$dirty_head.dirty" ] || fail "未 commit の出所変更で画面の印が .dirty でない: $dirty_web"
  dirty_build_config="$dirty_clone/collector-android/app/build/generated/source/buildConfig/release/dev/ashiato/collector/BuildConfig.java"
  dirty_android="$(exact_stamp_in "未 commit の出所変更後の携帯の印" "0\\.9\\.0\\+([0-9a-f]{40}|unknown)\\.dirty" "$dirty_build_config")"
  [ "$dirty_android" = "0.9.0+$dirty_head.dirty" ] || fail "未 commit の出所変更で携帯の印が .dirty でない: $dirty_android"
  echo "== Scenario: 根の Cargo.toml を書き換えて commit していなければ、携帯と画面の印は .dirty を持つ"
  exit 0
fi

# ---- server ----
clone="$work/clone"
git clone -q --local --no-hardlinks "$root" "$clone"

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

echo "-- 部品のソースの道: 画面の木の変更は .dirty にせず、migrations/・crates/build-stamp の変更は .dirty にする（design D5）"
git -C "$clone" commit -q -am "stamp check dirty"
head_c="$(git -C "$clone" rev-parse HEAD)"
echo "<!-- stamp check -->" >> "$clone/web/index.html"
d="$(one_stamp ほかの部品の変更後の組み立て)"
case "$d" in "$cargo_version+$head_c.debug") echo "OK: web/ の変更 -> $d";; *) fail "ほかの部品（web/）の変更で印が変わった: $d";; esac
echo "# stamp check" >> "$clone/migrations/ORDER"
e="$(one_stamp migrationsの変更後の組み立て)"
case "$e" in "$cargo_version+$head_c.dirty.debug") echo "OK: migrations/ の変更 -> $e";; *) fail "migrations/ の変更の後の印が .dirty.debug でない: $e";; esac
git -C "$clone" checkout -q -- migrations/ORDER
f="$(one_stamp migrationsを戻した組み立て)"
case "$f" in "$cargo_version+$head_c.debug") ;; *) fail "migrations/ を戻した後の印が .dirty のまま: $f";; esac
echo "// stamp check" >> "$clone/crates/build-stamp/src/lib.rs"
g="$(one_stamp build-stampの変更後の組み立て)"
case "$g" in "$cargo_version+$head_c.dirty.debug") echo "OK: crates/build-stamp の変更 -> $g";; *) fail "crates/build-stamp の変更の後の印が .dirty.debug でない: $g";; esac
