#!/usr/bin/env bash
# Windows の配布物の組み立ての鍵を 1 行で出す（CH17 design D1）。
#   windows-build-key.sh [<commit>]   既定は HEAD。鍵（sha256 の 16 進 64 桁）を標準出力へ
#   windows-build-key.sh --self-test  使い捨ての git の木で鍵の Scenario を確かめる
#
# 鍵 = sha256（ls-tree の行のうち除くパスを落としたもの ＋ 道具の版の行）。
# 道具の版の行は環境変数 WINDOWS_BUILD_KEY_TOOLS の文字列（CI は rustc -V・node -v・ImageOS・ImageVersion を並べて渡す）。
# 鍵を出せない（git の木でない・commit が引けない）ときは、何も出さずに 0 でない終了コードで終わる。
# Windows の Git Bash でも動くよう、sha256sum と git と bash の組み込みだけを使う。
set -euo pipefail

# 除くパス（bash の glob。`*` は `/` も越える）。理由: これらは組み立て（build.sh・package.sh など）が読まない。
# 試験の台本 3 つ（install-check.sh だけが conpty-run.ps1 を読む）と、changes.sh code が「コードでない」とするもの。
# 足すときは、組み立てがそれを読まないことを確かめる。Cargo.toml と LICENSE は外さない（design D1）。
EXCLUDE_PATHS=(
  'tools/release/install-check.sh'
  'tools/release/update-check-install.sh'
  'tools/release/conpty-run.ps1'
  'docs/*'
  'openspec/*'
  '*.md'
)

key() {
  local commit="${1:-HEAD}" root line path pat skip
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "windows-build-key: git の木でない" >&2; return 2; }
  git -C "$root" rev-parse --verify --quiet "${commit}^{commit}" >/dev/null \
    || { echo "windows-build-key: commit を引けない: $commit" >&2; return 2; }
  {
    printf 'tools:%s\n' "${WINDOWS_BUILD_KEY_TOOLS:-}"
    git -C "$root" ls-tree -r --full-tree "$commit" | while IFS= read -r line; do
      path="${line#*$'\t'}"
      skip=0
      for pat in "${EXCLUDE_PATHS[@]}"; do
        # shellcheck disable=SC2053
        if [[ "$path" == $pat ]]; then skip=1; break; fi
      done
      [ "$skip" = 1 ] || printf '%s\n' "$line"
    done
  } | sha256sum | cut -d' ' -f1
}

self_test() {
  local script work t keyfile base k alt
  script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  work="$(mktemp -d "${TMPDIR:-/tmp}/wbk.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  t="$work/tree"
  mkdir -p "$t"
  export WINDOWS_BUILD_KEY_TOOLS="rustc 1.90.0|node v24.0.0|ImageOS=win25|ImageVersion=20260101.1"
  local git_t=(git -C "$t" -c user.name=t -c user.email=t@example.invalid)
  "${git_t[@]}" init -q
  local f
  for f in tools/release/install-check.sh tools/release/update-check-install.sh tools/release/conpty-run.ps1 \
           docs/a.txt docs/y.md openspec/z.txt README.md \
           crates/server/src/lib.rs migrations/ORDER web/src/App.tsx tools/release/build.sh tools/release/pgsql.lock \
           tools/licenses/a.txt LICENSE Cargo.toml collector-android/app.kt Cargo.lock .github/workflows/ci.yml; do
    mkdir -p "$t/$(dirname "$f")"
    echo "base $f" >"$t/$f"
  done
  "${git_t[@]}" add -A
  "${git_t[@]}" commit -q -m base
  base="$("${git_t[@]}" rev-parse HEAD)"

  # 変えた木の鍵を出す。$1: 台本  残り: "パス=内容" の列。base の上に commit して、base へ戻る
  key_with() {
    local s="$1" kv p out
    shift
    "${git_t[@]}" checkout -q --detach "$base"
    for kv in "$@"; do
      p="${kv%%=*}"
      mkdir -p "$t/$(dirname "$p")"
      printf '%s\n' "${kv#*=}" >"$t/$p"
    done
    "${git_t[@]}" add -A
    "${git_t[@]}" commit -q -m variant
    out="$(cd "$t" && "$s" HEAD)"
    "${git_t[@]}" checkout -q --detach "$base"
    "${git_t[@]}" clean -qfd
    printf '%s' "$out"
  }
  local base_key
  base_key="$(cd "$t" && "$script" "$base")"
  [[ "$base_key" =~ ^[0-9a-f]{64}$ ]] || { echo "NG: 鍵が sha256 の形でない: $base_key"; return 1; }

  echo "== Scenario: 試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる"
  k="$(key_with "$script" \
    'tools/release/install-check.sh=changed' 'tools/release/update-check-install.sh=changed' \
    'tools/release/conpty-run.ps1=changed' 'docs/x.md=new doc')"
  [ "$k" = "$base_key" ] || { echo "NG: 台本と文書の違いで鍵が変わった"; return 1; }
  # 除くことが効いている確かめ: 除くパスの列挙から install-check.sh を外した台本では、同じ変更で鍵が変わる
  alt="$work/windows-build-key-noexclude.sh"
  grep -v "^  'tools/release/install-check.sh'$" "$script" >"$alt"
  chmod +x "$alt"
  k="$(key_with "$alt" 'tools/release/install-check.sh=changed')"
  [ "$k" != "$(cd "$t" && "$alt" "$base")" ] || { echo "NG: 除くパスを外しても鍵が変わらない（除くことが効いていない）"; return 1; }
  echo "ok"

  echo "== Scenario: 組み立てに入るファイルが 1 つでも違えば、鍵が変わる"
  for f in crates/server/src/lib.rs migrations/ORDER web/src/App.tsx tools/release/build.sh tools/release/pgsql.lock \
           tools/licenses/a.txt LICENSE Cargo.toml collector-android/app.kt Cargo.lock .github/workflows/ci.yml; do
    k="$(key_with "$script" "$f=changed")"
    [ "$k" != "$base_key" ] || { echo "NG: $f を変えても鍵が変わらない"; return 1; }
  done
  # repo の根でない cwd から走らせても同じ鍵になる
  k="$(mkdir -p "$t/crates" && cd "$t/crates" && "$script" "$base")"
  [ "$k" = "$base_key" ] || { echo "NG: cwd が違うと鍵が変わる"; return 1; }
  echo "ok"

  echo "== Scenario: 名指ししていないファイルが増えれば、鍵が変わる"
  k="$(key_with "$script" 'tools/release/new-script.sh=new')"
  [ "$k" != "$base_key" ] || { echo "NG: 新しい台本を足しても鍵が変わらない"; return 1; }
  echo "ok"

  echo "== Scenario: 道具の版が違えば、鍵が変わる"
  k="$(cd "$t" && WINDOWS_BUILD_KEY_TOOLS="rustc 1.91.0|node v24.0.0|ImageOS=win25|ImageVersion=20260101.1" "$script" "$base")"
  [ "$k" != "$base_key" ] || { echo "NG: Rust の版を変えても鍵が変わらない"; return 1; }
  echo "ok"

  echo "== Scenario: 鍵を出せない木では、鍵を出さずに落ちる"
  local plain="$work/plain" out rc=0
  mkdir -p "$plain"
  out="$(cd "$plain" && GIT_CEILING_DIRECTORIES="$work" "$script" HEAD 2>/dev/null)" || rc=$?
  [ "$rc" -ne 0 ] || { echo "NG: git の木でないのに rc=0"; return 1; }
  [ -z "$out" ] || { echo "NG: git の木でないのに出力がある: $out"; return 1; }
  rc=0
  out="$(cd "$t" && "$script" no-such-commit 2>/dev/null)" || rc=$?
  [ "$rc" -ne 0 ] && [ -z "$out" ] || { echo "NG: 引けない commit で鍵を出した"; return 1; }
  echo "ok"
}

case "${1:-}" in
  --self-test) self_test ;;
  *) key "${1:-HEAD}" ;;
esac
