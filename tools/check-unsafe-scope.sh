#!/usr/bin/env bash
# `unsafe` を許す場所を 1 つに保つ。作業場の lint は `unsafe_code = "deny"`（collector-windows の DPAPI のため
# `forbid` から緩めた）で、`deny` はどのファイルの `#[allow(unsafe_code)]` も止めない。許してよいのは
# `crates/collector-windows/src/secret.rs`（review/code.md R32）と、インストーラの窓の口
# `crates/installer/src/console.rs`（CH14 design D4・D8。deep.md Q5 で窓へ直に `WriteConsoleW` で書くと決めた）だけ。
set -euo pipefail

ROOT="${UNSAFE_SCOPE_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
ALLOWED=(crates/collector-windows/src/secret.rs crates/installer/src/console.rs)

check() {
    local root="$1" hits toml path excludes=()
    for path in "${ALLOWED[@]}"; do excludes+=(-not -path "$path"); done
    # 属性は行をまたいでも見る（`#[allow(\n    unsafe_code)]`。review/code.md R45）
    hits="$(cd "$root" && find crates -name '*.rs' "${excludes[@]}" -print0 |
        xargs -0 -r perl -0ne 'print "$ARGV\n" if /\b(allow|expect|warn)\s*\(([^)]*,)?\s*unsafe_code/s' || true)"
    if [ -n "$hits" ]; then
        echo "error: unsafe_code を許す属性は ${ALLOWED[*]} の外に置かない:" >&2
        echo "$hits" >&2
        return 1
    fi
    # crate が作業場の lint を受けずに unsafe_code を緩めることも止める（属性を書かずに素通りできる。R45）
    for toml in "$root"/crates/*/Cargo.toml; do
        [ -f "$toml" ] || continue
        if ! perl -0ne 'exit(/^\[lints\]\s*\n\s*workspace\s*=\s*true/m ? 0 : 1)' "$toml" \
            || grep -qE '^\s*unsafe_code\s*=' "$toml"; then
            echo "error: ${toml#"$root"/} は [lints] workspace = true を持ち、unsafe_code を上書きしない" >&2
            return 1
        fi
    done
}

if [ "${1:-}" = "--self-test" ]; then
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    mkdir -p "$work/crates/collector-windows/src" "$work/crates/installer/src" "$work/crates/server/src"
    for path in "${ALLOWED[@]}"; do printf '#![allow(unsafe_code)]\n' > "$work/$path"; done
    printf '[package]\nname = "x"\n\n[lints]\nworkspace = true\n' > "$work/crates/server/Cargo.toml"
    cp "$work/crates/server/Cargo.toml" "$work/crates/collector-windows/Cargo.toml"
    printf 'fn main() {}\n' > "$work/crates/server/src/lib.rs"
    printf '#![allow(unsafe_code)]\n' > "$work/crates/installer/src/main.rs"
    if check "$work" 2>/dev/null; then echo "error: self-test: インストーラの console.rs の外の allow(unsafe_code) を見逃した" >&2; exit 1; fi
    rm "$work/crates/installer/src/main.rs"
    check "$work" || { echo "error: self-test: 許した場所だけで落ちた" >&2; exit 1; }
    printf '#[allow(unsafe_code)]\nfn f() {}\n' >> "$work/crates/server/src/lib.rs"
    if check "$work" 2>/dev/null; then echo "error: self-test: サーバの allow(unsafe_code) を見逃した" >&2; exit 1; fi
    printf 'fn main() {}\n#[allow(\n    unsafe_code\n)]\nfn f() {}\n' > "$work/crates/server/src/lib.rs"
    if check "$work" 2>/dev/null; then echo "error: self-test: 行をまたぐ allow を見逃した" >&2; exit 1; fi
    printf 'fn main() {}\n' > "$work/crates/server/src/lib.rs"
    printf '[package]\nname = "x"\n\n[lints.rust]\nunsafe_code = "allow"\n' > "$work/crates/server/Cargo.toml"
    if check "$work" 2>/dev/null; then echo "error: self-test: Cargo.toml の上書きを見逃した" >&2; exit 1; fi
    echo "unsafe scope self-test: PASS"
    exit 0
fi

check "$ROOT"
echo "unsafe scope: PASS"
