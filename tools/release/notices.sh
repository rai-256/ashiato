#!/usr/bin/env bash
set -euo pipefail

root="${ASHIATO_NOTICES_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
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
  find "$dir" -maxdepth 2 -type f \( -iname 'license*' -o -iname 'copying*' -o -iname 'notice*' -o -path '*/LICENSES/*' \) -print 2>/dev/null | sort | head -n1
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
  done < <(grep -Eo '[A-Za-z0-9][A-Za-z0-9.+-]*( WITH [A-Za-z0-9.+-]+)?' <<<"$expr" | grep -Ev '^(AND|OR)$' | sort -u)
}

emit_rust() {
  local name="$1" version="$2" license="$3" authors="$4" manifest="$5" dir lic
  [[ "$license" == - ]] && license=""
  [[ "$authors" == - ]] && authors=""
  dir="$(dirname "$manifest")"
  [[ -d "$dir" ]] || die "許諾の全文も著作権の表示も無い部品: $name"
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
    author=v.get('author','-')
    if isinstance(author,dict): author=author.get('name','-')
    print('\t'.join([name,str(v.get('version','')),str(v.get('license') or '-'),str(author or '-'),os.path.join(w,k)]))
PY
}

emit_node() {
  local name="$1" version="$2" license="$3" author="$4" dir="$5" lic body
  [[ "$license" == - ]] && license=""
  [[ "$author" == - ]] && author=""
  lic="$(first_license_file "$dir")"
  printf '== %s %s\nLicense: %s\n' "$name" "$version" "$license"
  if [[ -n "$lic" ]]; then cat "$lic"; printf '\n\n'; return; fi
  [[ -n "$license" && -n "$author" ]] || die "許諾の全文も著作権の表示も無い部品: $name"
  body="$(spdx_text "$license" 2>/dev/null || true)"
  [[ -n "$body" ]] || die "許諾の全文が無い部品: $name（$license）"
  printf '%s\nCopyright: %s\n\n' "$body" "$author"
}

rust_notice() {
  local kind="$1" out="$2" target="${3:-}" crate
  [[ "$kind" == server ]] && crate=ashiato-server || crate=ashiato-collector-windows
  local tmp rows nodes raw; tmp="$(mktemp "${TMPDIR:-/tmp}/notices.XXXXXX")"; rows="$(mktemp "${TMPDIR:-/tmp}/notices-rows.XXXXXX")"; nodes="$(mktemp "${TMPDIR:-/tmp}/notices-nodes.XXXXXX")"; raw="$(mktemp "${TMPDIR:-/tmp}/notices-raw.XXXXXX")"
  NOTICE_TMP="$tmp"; NOTICE_ROWS="$rows"; NOTICE_NODES="$nodes"; NOTICE_RAW="$raw"
  trap 'rm -f "${NOTICE_TMP:-}" "${NOTICE_ROWS:-}" "${NOTICE_NODES:-}" "${NOTICE_RAW:-}"' EXIT
  rust_rows "$crate" "$target" | awk -F $'\t' -v c="$crate" '$1 != c' | sort -t $'\t' -k1,1 -k2,2 > "$rows"
  [[ "$kind" != server ]] || node_rows | sort -t $'\t' -k1,1 > "$nodes"
  { while IFS=$'\t' read -r n v l a m; do emit_rust "$n" "$v" "$l" "$a" "$m"; done < "$rows"
    std_entry
    if [[ "$kind" == server ]]; then while IFS=$'\t' read -r n v l a d; do emit_node "$n" "$v" "$l" "$a" "$d"; done < "$nodes"; fi
  } > "$raw"
  python3 -c 'import re,sys; s=open(sys.argv[1]).read(); print("\n".join(sorted(re.split(r"(?=^== )", s, flags=re.M), key=lambda x: x.splitlines()[0] if x.strip() else "")).strip())' "$raw" > "$tmp"
  mv "$tmp" "$out"
  rm -f "$tmp" "$rows" "$nodes" "$raw"; NOTICE_TMP=; NOTICE_ROWS=; NOTICE_NODES=; NOTICE_RAW=; trap - EXIT
}

apk_notice() {
  [[ $# -ge 1 ]] || usage
  local out="$1"; shift; local list tmp generated_list=""
  tmp="$(mktemp "${TMPDIR:-/tmp}/notices-apk.XXXXXX")"; trap 'rm -f "${tmp:-}" "${generated_list:-}"' EXIT
  list="${ASHIATO_LICENSES_POM_LIST:-}"
  if [[ -z "$list" ]]; then
    if [[ $# -gt 0 && -f "$1" ]]; then list="$1"; shift
    else
      list="$(mktemp "${TMPDIR:-/tmp}/notices-apk-poms.XXXXXX")"
      generated_list="$list"
      "$root/collector-android/gradlew" -p "$root/collector-android" -q :app:ashiatoDependencyPoms "-Pashiato.pomOut=$list" >/dev/null
    fi
  fi
  [[ -s "$list" ]] || die "Task 4.1 の Android 依存一覧が無い: $list"
  python3 - "$list" "$tmp" "${GRADLE_USER_HOME:-$HOME/.gradle}" "$texts" "$root/tools/licenses/pom-license-names.tsv" <<'PY'
import json, os, pathlib, re, sys, zipfile
list_path, out_path, gradle_home, texts, names_path = sys.argv[1:]
cache=pathlib.Path(gradle_home)/'caches/modules-2/files-2.1'
named={line.rstrip('\n').split('\t',1)[0]:line.rstrip('\n').split('\t',1)[1] for line in open(names_path, encoding='utf-8') if '\t' in line}
rows=[]
for line in open(list_path, encoding='utf-8'):
    f=line.rstrip('\n').split('\t')
    if len(f)!=4: raise SystemExit(f'Task 4.1 の一覧の列数が不正: {line.rstrip()}')
    rows.append(f)
blocks=[]
for coordinate, license_name, _, _ in sorted(rows):
    group,module,version=coordinate.split(':',2)
    candidates=list((cache/group/module/version).glob('*/*.aar')) + list((cache/group/module/version).glob('*/*.jar'))
    aar=candidates[0] if candidates else None
    if aar is None and module.startswith('play-services-'):
        raise SystemExit(f'AAR が無い: {coordinate}')
    z=zipfile.ZipFile(aar) if aar else None
    body=''
    if z:
        lic_files=sorted(n for n in z.namelist() if re.match(r'META-INF/(LICENSE|COPYING|NOTICE)', n, re.I))
        body='\n'.join(z.read(n).decode('utf-8','replace') for n in lic_files)
    if not body:
        spdx=named.get(license_name, license_name)
        if spdx == 'PROPRIETARY':
            direct=pathlib.Path(texts)/(license_name+'.txt')
            if not direct.is_file(): raise SystemExit(f'許諾の全文が無い: {coordinate} ({license_name})')
            body=direct.read_text(encoding='utf-8')
            spdx=''
        if spdx:
            parts=re.findall(r'[A-Za-z0-9][A-Za-z0-9.+-]*', spdx)
            chunks=[]
            for part in dict.fromkeys(p for p in parts if p not in ('AND','OR')):
                p=pathlib.Path(texts)/(part+'.txt')
                if not p.is_file(): raise SystemExit(f'許諾の全文が無い: {coordinate} ({license_name})')
                chunks.append(p.read_text(encoding='utf-8'))
            body='\n\n'.join(chunks)
    display_name=f'{group}:{module}'
    block=f'== {display_name} {version}\nLicense: {license_name}\n{body.strip()}\n'
    if coordinate in {f'com.google.android.gms:{x}:{version}' for x in ('play-services-location','play-services-base','play-services-basement','play-services-tasks')}:
        if not z or 'third_party_licenses.txt' not in z.namelist() or 'third_party_licenses.json' not in z.namelist():
            raise SystemExit(f'gms の第三者許諾資料が無い: {coordinate}')
        raw=z.read('third_party_licenses.txt').decode('utf-8','replace')
        index=json.loads(z.read('third_party_licenses.json'))
        pieces=[raw[v['start']:v['start']+v['length']] for v in index.values()]
        block += '\n(gms の中の第三者)\n' + '\n'.join(pieces).rstrip() + '\n'
    if z: z.close()
    blocks.append(block)
open(out_path,'w',encoding='utf-8').write('\n'.join(blocks)+'\n')
PY
  mv "$tmp" "$out"; rm -f "$tmp" "$generated_list"; trap - EXIT
}

self_test() {
  local work; work="$(mktemp -d "${TMPDIR:-/tmp}/notices-self.XXXXXX")"; trap 'rm -rf "${work:-}"' EXIT
  echo '== Scenario: 許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる'
  printf 'fake\t1.0\tFake-1.0\t-\t%s/nope/Cargo.toml\n' "$work" > "$work/list"
  local failure
  if failure="$(ASHIATO_NOTICES_RUST_LIST="$work/list" "$0" collector-windows "$work/missing" 2>&1)"; then die '偽の部品を受け入れた'; fi
  grep -q 'fake' <<<"$failure" || die '失敗した部品の名前が無い'
  [[ ! -e "$work/missing" ]] || die '失敗時に一覧を作った'
  echo '== Scenario: サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない'
  "$0" server "$work/server" >/dev/null
  grep -q '^== react ' "$work/server" || die 'react が無い'; ! grep -q '^== vitest ' "$work/server" || die 'vitest が入った'
  echo '== Scenario: 正式版の配布物のソースの場所は、その版の印'; ASHIATO_SOURCE_REF=v0.4.0 "$0" source "$work/ref" 0.4.0 ""; grep -q '^Ref: v0.4.0$' "$work/ref"
  local main_tree="$work/main-tree" pr_tree="$work/pr-tree" main_commit pr_commit actual
  git init -q "$main_tree"; git -C "$main_tree" config user.email test@example.invalid; git -C "$main_tree" config user.name test; : > "$main_tree/file"; git -C "$main_tree" add file; git -C "$main_tree" commit -qm main
  main_commit="$(git -C "$main_tree" rev-parse HEAD)"
  git clone -q "$main_tree" "$pr_tree"; git -C "$pr_tree" config user.email test@example.invalid; git -C "$pr_tree" config user.name test; printf pr > "$pr_tree/file"; git -C "$pr_tree" commit -qam pr
  pr_commit="$(git -C "$pr_tree" rev-parse HEAD)"
  echo '== Scenario: 開発版と main への push の配布物のソースの場所は、組み立てた commit'; actual="$(ASHIATO_NOTICES_ROOT="$main_tree" "$0" commit)"; [[ "$actual" == "$main_commit" ]]; ASHIATO_NOTICES_ROOT="$main_tree" "$0" source "$work/main" 0.4.0 ""; grep -q "^Commit: $main_commit$" "$work/main"; if grep -q nightly "$work/main"; then die '開発版の SOURCE.txt に nightly が入った'; fi
  echo '== Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit'; actual="$(ASHIATO_NOTICES_ROOT="$pr_tree" "$0" commit)"; [[ "$actual" == "$pr_commit" ]]; ASHIATO_NOTICES_ROOT="$pr_tree" "$0" source "$work/pr" 0.4.0 ""; grep -q "^Commit: $pr_commit$" "$work/pr"
  echo '== Scenario: 組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる'; if ASHIATO_BUILD_COMMIT=main "$0" source "$work/bad" 0.4.0 "" >/dev/null 2>&1; then die '不正な commit を受け入れた'; fi; [[ ! -e "$work/bad" ]]
  echo '== Scenario: 印が版番号と違う公開の組み立ては、配布物を作らずに落ちる'; if ASHIATO_SOURCE_REF=v0.4.1 "$0" source "$work/wrong" 0.4.0 "$main_commit" >/dev/null 2>&1; then die '不一致の印を受け入れた'; fi; [[ ! -e "$work/wrong" ]]
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
