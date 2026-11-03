#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
texts="$root/tools/licenses/texts"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <server|collector-windows|apk|source|commit> ... | --self-test" >&2; exit 2; }

valid_commit() { [[ "$1" =~ ^[0-9a-fA-F]{40}$ ]]; }

commit_value() {
  local value="${ASHIATO_BUILD_COMMIT:-}"
  [[ -n "$value" ]] || value="$(git -C "$root" rev-parse HEAD 2>/dev/null || true)"
  valid_commit "$value" || die "組み立ての commit が 40 桁の 16 進でない: $value"
  printf '%s\n' "$value"
}

source_notice() {
  [[ $# == 3 ]] || usage
  local out="$1" version="$2" commit="$3" ref="${ASHIATO_SOURCE_REF:-}" repo="${ASHIATO_SOURCE_REPO:-https://github.com/rai-256/ashiato}" second
  if [[ -n "$ref" ]]; then
    [[ "$ref" == "v$version" ]] || die "印が版番号と違う: $ref（期待 v$version）"
    second="Ref: $ref"
  else
    [[ -n "$commit" ]] || commit="$(commit_value)"
    valid_commit "$commit" || die "組み立ての commit が 40 桁の 16 進でない: $commit"
    second="Commit: $commit"
  fi
  local tmp; tmp="$(mktemp "${TMPDIR:-/tmp}/source.XXXXXX")"
  trap 'rm -f "${tmp:-}"' EXIT
  {
    printf 'Repository: %s\n' "$repo"
    printf '%s\n' "$second"
    printf '取得: git clone %s && git checkout %s\n' "$repo" "${second#*: }"
  } > "$tmp"
  mv "$tmp" "$out"
  rm -f "$tmp"; trap - EXIT
}

rust_rows() {
  local crate="$1" target="${2:-}" list="${ASHIATO_NOTICES_RUST_LIST:-}"
  if [[ -n "$list" ]]; then
    [[ -f "$list" ]] || die "Rust の一覧が無い: $list"
    awk -F '\t' 'NF >= 5 {print}' "$list"
    return
  fi
  local args=(-p "$crate" -e normal)
  [[ -n "$target" ]] && args+=(--target "$target")
  local tree; tree="$(cd "$root" && cargo tree "${args[@]}" --prefix none --format '{p}')" || die "cargo tree に失敗した"
  CARGO_TREE="$tree" python3 - "$root" <<'PY'
import json, os, re, sys
root=sys.argv[1]
meta=json.loads(__import__('subprocess').check_output(['cargo','metadata','--format-version','1'],cwd=root,text=True))
by={(p['name'],p['version']):p for p in meta['packages']}
seen=set()
for raw in os.environ['CARGO_TREE'].splitlines():
    raw=re.sub(r' \(\*\)$','',raw.strip())
    m=re.match(r'(.+?) v([^ ]+)(?: \(.*\))?$',raw)
    if not m: continue
    key=(m.group(1),m.group(2)); p=by.get(key)
    if not p or key in seen: continue
    seen.add(key)
    print('\t'.join([p['name'],p['version'],p.get('license') or '-', '; '.join(p.get('authors') or []) or '-',p['manifest_path']]))
PY
}

first_license_file() {
  local dir="$1"
  find "$dir" -maxdepth 2 -type f \( -iname 'license*' -o -iname 'copying*' -o -iname 'notice*' -o -path '*/LICENSES/*' \) -print | sort | head -n1
}

all_license_files() {
  find "$1" -maxdepth 2 -type f \( -iname 'license*' -o -iname 'copying*' -o -iname 'notice*' -o -path '*/LICENSES/*' \) -print | sort
}

spdx_text() {
  local expr="$1" f
  [[ -d "$texts" ]] || return 1
  # Compound expressions are represented by their individual SPDX texts.
  while read -r name; do
    f="$texts/$name.txt"
    [[ -s "$f" ]] || return 1
    cat "$f"
    printf '\n'
  done < <(grep -Eo '[A-Za-z0-9.]+-[A-Za-z0-9.-]+' <<<"$expr" | sort -u)
}

emit_rust() {
  local name="$1" version="$2" license="$3" authors="$4" manifest="$5" dir lic
  [[ "$license" == - ]] && license=""
  [[ "$authors" == - ]] && authors=""
  dir="$(dirname "$manifest")"
  lic="$(first_license_file "$dir")"
  if [[ -n "$lic" ]]; then
    printf '== %s %s\nLicense: %s\n' "$name" "$version" "$license"
    while IFS= read -r f; do cat "$f"; printf '\n'; done < <(all_license_files "$dir"); printf '\n'
  else
    [[ -n "$license" ]] || die "許諾が無い部品: $name"
    [[ -n "$authors" ]] || die "許諾の全文も著作権の表示も無い部品: $name"
    local body; body="$(spdx_text "$license" 2>/dev/null || true)"
    [[ -n "$body" ]] || die "許諾の全文が無い部品: $name（$license）"
    printf '== %s %s\nLicense: %s\n%s\nCopyright: %s\n\n' "$name" "$version" "$license" "$body" "$authors"
  fi
}

std_entry() {
  local sysroot; sysroot="$(rustc --print sysroot 2>/dev/null)" || die "rustc が無い"
  local d="$sysroot/share/doc/rust" mit apache copy
  mit="$d/licenses/MIT.txt"; apache="$d/licenses/Apache-2.0.txt"; copy="$d/COPYRIGHT-library.html"
  [[ -s "$mit" && -s "$apache" && -s "$copy" ]] || die "Rust 標準ライブラリの許諾資料が無い: $d"
  printf '== rust-std %s\nLicense: MIT OR Apache-2.0\n' "$(rustc --version | sed 's/^rustc //')"
  cat "$mit"; printf '\n'; cat "$apache"; printf '\nCopyright:\n'; cat "$copy"; printf '\n\n'
}

node_rows() {
  local webroot="${ASHIATO_NOTICES_NODE_ROOT:-$root/web}"
  [[ -f "$webroot/package-lock.json" ]] || die "Node の package-lock.json が無い: $webroot"
  python3 - "$webroot" <<'PY'
import json,sys,os
w=sys.argv[1]; x=json.load(open(os.path.join(w,'package-lock.json')))
for k,v in x.get('packages',{}).items():
    if not k or v.get('dev') is True: continue
    name=k.removeprefix('node_modules/')
    if name == k: continue
    print('\t'.join([name,str(v.get('version','')),str(v.get('license') or '-'),os.path.join(w,k)]))
PY
}

emit_node() {
  local name="$1" version="$2" license="$3" dir="$4" lic
  [[ "$license" == - ]] && license=""
  lic="$(first_license_file "$dir")"
  [[ -n "$lic" ]] || die "Node 部品の許諾ファイルが無い: $name"
  printf '== %s %s\nLicense: %s\n' "$name" "$version" "$license"; cat "$lic"; printf '\n\n'
}

rust_notice() {
  local kind="$1" out="$2" target="${3:-}" crate
  [[ "$kind" == server ]] && crate=ashiato-server || crate=ashiato-collector-windows
  local tmp rows nodes; tmp="$(mktemp "${TMPDIR:-/tmp}/notices.XXXXXX")"; rows="$(mktemp "${TMPDIR:-/tmp}/notices-rows.XXXXXX")"; nodes="$(mktemp "${TMPDIR:-/tmp}/notices-nodes.XXXXXX")"
  NOTICE_TMP="$tmp"; NOTICE_ROWS="$rows"; NOTICE_NODES="$nodes"
  trap 'rm -f "${NOTICE_TMP:-}" "${NOTICE_ROWS:-}" "${NOTICE_NODES:-}"' EXIT
  rust_rows "$crate" "$target" | awk -F $'\t' -v c="$crate" '$1 != c' | sort -t $'\t' -k1,1 -k2,2 > "$rows"
  [[ "$kind" != server ]] || node_rows | sort -t $'\t' -k1,1 > "$nodes"
  { while IFS=$'\t' read -r n v l a m; do emit_rust "$n" "$v" "$l" "$a" "$m"; done < "$rows"
    std_entry
    if [[ "$kind" == server ]]; then while IFS=$'\t' read -r n v l d; do emit_node "$n" "$v" "$l" "$d"; done < "$nodes"; fi
  } > "$tmp"
  mv "$tmp" "$out"
  rm -f "$tmp" "$rows" "$nodes"; NOTICE_TMP=; NOTICE_ROWS=; NOTICE_NODES=; trap - EXIT
}

apk_notice() {
  [[ $# -ge 1 ]] || usage
  local out="$1"; shift; local tmp; tmp="$(mktemp "${TMPDIR:-/tmp}/notices-apk.XXXXXX")"; trap 'rm -f "${tmp:-}"' EXIT
  local -a aars=("$@")
  if [[ ${#aars[@]} == 0 ]]; then
    while IFS= read -r f; do aars+=("$f"); done < <(find "${GRADLE_USER_HOME:-$HOME/.gradle}/caches/modules-2/files-2.1/com.google.android.gms" -type f -name '*.aar' 2>/dev/null)
  fi
  local -a expanded=()
  for aar in "${aars[@]}"; do
    if [[ -d "$aar" ]]; then
      while IFS= read -r f; do expanded+=("$f"); done < <(find "$aar" -type f -name '*.aar' -print)
    else
      expanded+=("$aar")
    fi
  done
  aars=("${expanded[@]}")
  [[ ${#aars[@]} -gt 0 ]] || die "AAR が無い"
  : > "$tmp"
  for aar in "${aars[@]}"; do
    [[ -f "$aar" ]] || die "AAR が無い: $aar"
    local base coord version name; base="$(basename "$aar" .aar)"; version="$(basename "$(dirname "$(dirname "$aar")")")"; name="${base%-${version}}"; coord="com.google.android.gms:$name"
    printf '== %s %s\nLicense: Android Software Development Kit License\n' "$coord" "$version" >> "$tmp"
    unzip -p "$aar" 'META-INF/LICENSE*' 2>/dev/null >> "$tmp" || true; printf '\n' >> "$tmp"
    if [[ "$coord" == com.google.android.gms:play-services-location || "$coord" == com.google.android.gms:play-services-base || "$coord" == com.google.android.gms:play-services-basement || "$coord" == com.google.android.gms:play-services-tasks ]]; then
      printf '(gms の中の第三者)\n' >> "$tmp"
      unzip -p "$aar" 'third_party_licenses.txt' 2>/dev/null >> "$tmp" || true
      printf '\n' >> "$tmp"
    fi
  done
  mv "$tmp" "$out"
  rm -f "$tmp"; trap - EXIT
}

self_test() {
  local work; work="$(mktemp -d "${TMPDIR:-/tmp}/notices-self.XXXXXX")"; trap 'rm -rf "${work:-}"' EXIT
  echo '== Scenario: 許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる'
  printf 'fake\t1.0\tFake-1.0\t-\t%s/nope/Cargo.toml\n' "$work" > "$work/list"
  if ASHIATO_NOTICES_RUST_LIST="$work/list" "$0" collector-windows "$work/missing" >/dev/null 2>&1; then die '偽の部品を受け入れた'; fi
  [[ ! -e "$work/missing" ]] || die '失敗時に一覧を作った'
  echo '== Scenario: サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない'
  "$0" server "$work/server" >/dev/null
  grep -q '^== react ' "$work/server" || die 'react が無い'; ! grep -q '^== vitest ' "$work/server" || die 'vitest が入った'
  echo '== Scenario: 正式版の配布物のソースの場所は、その版の印'; ASHIATO_SOURCE_REF=v0.4.0 "$0" source "$work/ref" 0.4.0 ""; grep -q '^Ref: v0.4.0$' "$work/ref"
  echo '== Scenario: 開発版と main への push の配布物のソースの場所は、組み立てた commit'; local c; c="$(git -C "$root" rev-parse HEAD)"; "$0" source "$work/main" 0.4.0 "$c"; grep -q "^Commit: $c$" "$work/main"; ! grep -q nightly "$work/main"
  echo '== Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit'; "$0" source "$work/pr" 0.4.0 "$c"; grep -q "^Commit: $c$" "$work/pr"
  echo '== Scenario: 組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる'; if ASHIATO_BUILD_COMMIT=main "$0" source "$work/bad" 0.4.0 main >/dev/null 2>&1; then die '不正な commit を受け入れた'; fi; [[ ! -e "$work/bad" ]]
  echo '== Scenario: 印が版番号と違う公開の組み立ては、配布物を作らずに落ちる'; if ASHIATO_SOURCE_REF=v0.4.1 "$0" source "$work/wrong" 0.4.0 "$c" >/dev/null 2>&1; then die '不一致の印を受け入れた'; fi; [[ ! -e "$work/wrong" ]]
  rm -rf "$work"; trap - EXIT
}

case "${1:-}" in
  --self-test) self_test;;
  commit) commit_value;;
  source) shift; source_notice "$@";;
  server|collector-windows) kind="$1"; out="${2:-}"; [[ -n "$out" ]] || usage; shift 2; target=""; [[ "${1:-}" == --target ]] && target="${2:-}"; rust_notice "$kind" "$out" "$target";;
  apk) shift; apk_notice "$@";;
  *) usage;;
esac
