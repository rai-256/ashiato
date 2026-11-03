#!/usr/bin/env bash
set -euo pipefail
# Windows の Python の既定の符号化（cp1252 など）で UTF-8 の許諾の本文を読み書きしないよう、入出力を UTF-8 にそろえる
export PYTHONUTF8=1

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
# Windows の Python は改行を \r\n に変える。bash の read が最後の列に \r を残さないよう \n にそろえる
sys.stdout.reconfigure(newline='\n')
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
    # Windows の cargo はバックスラッシュの置き場を返す。そのままでは bash の dirname が . になり、置き場を取り違える
    print('\t'.join([p['name'],p['version'],p.get('license') or '-', '; '.join(p.get('authors') or []) or '-',p['manifest_path'].replace('\\','/')]))
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
sys.stdout.reconfigure(newline='\n')
w=sys.argv[1]; x=json.load(open(os.path.join(w,'package-lock.json'),encoding='utf-8'))
for k,v in x.get('packages',{}).items():
    if not k or v.get('dev') is True: continue
    # 入れ子の部品（a/node_modules/b）も、名前は最後の node_modules/ の後ろ
    if not k.startswith('node_modules/'): continue
    name=k.rsplit('node_modules/',1)[-1]
    author=v.get('author','-')
    if isinstance(author,dict): author=author.get('name','-')
    print('\t'.join([name,str(v.get('version','')),str(v.get('license') or '-'),str(author or '-'),(w.rstrip('/\\')+'/'+k).replace('\\','/')]))
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
  python3 -c 'import re,sys; sys.stdout.reconfigure(newline="\n"); s=open(sys.argv[1],encoding="utf-8").read(); print("\n".join(sorted(re.split(r"(?=^== )", s, flags=re.M), key=lambda x: x.splitlines()[0] if x.strip() else "")).strip())' "$raw" > "$tmp"
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
import io, json, pathlib, re, sys, zipfile
import xml.etree.ElementTree as ET
list_path, out_path, gradle_home, texts, names_path = sys.argv[1:]
cache=pathlib.Path(gradle_home)/'caches/modules-2/files-2.1'
named={line.rstrip('\n').split('\t',1)[0]:line.rstrip('\n').split('\t',1)[1] for line in open(names_path, encoding='utf-8') if '\t' in line}
# 部品の中の許諾のファイル: META-INF の直下と、その下の置き場（androidx は META-INF/androidx/<group>/<name>/LICENSE.txt）
LICENSE_FILE=re.compile(r'META-INF/(?:.*/)?(LICENSE|COPYING|NOTICE)[^/]*$', re.I)
# 著作権の行に残る雛形。Apache-2.0 の付録の「Copyright [yyyy] [name of copyright owner]」は許諾の本文そのものなので数えない
TEMPLATE=re.compile(r'\[copyright holders?\]|<copyright holders?>|<year>|<owner>|\[year\]|\[fullname\]', re.I)
rows=[]
for line in open(list_path, encoding='utf-8'):
    f=line.rstrip('\n').split('\t')
    if len(f)!=4: raise SystemExit(f'Task 4.1 の一覧の列数が不正: {line.rstrip()}')
    rows.append(f)

def pom_root(group, module, version):
    poms=sorted((cache/group/module/version).glob('*/*.pom'))
    if not poms: return None
    text=poms[0].read_text(encoding='utf-8')
    text=re.sub(r'\sxmlns="[^"]*"', '', text, count=1)
    return ET.fromstring(text)

def pom_holders(group, module, version):
    """POM の developers / organization の名前。自分に無ければ親の POM をたどる（出所の座標も返す）"""
    seen=set()
    while (group, module, version) not in seen:
        seen.add((group, module, version))
        r=pom_root(group, module, version)
        if r is None: return [], ''
        names=[]
        for d in r.findall('developers/developer'):
            for tag in ('name', 'organization'):
                v=(d.findtext(tag) or '').strip()
                if v and v not in names: names.append(v)
        v=(r.findtext('organization/name') or '').strip()
        if v and v not in names: names.append(v)
        if names: return names, f'{group}:{module}:{version}'
        parent=r.find('parent')
        if parent is None: return [], ''
        group, module, version=((parent.findtext(t) or '').strip() for t in ('groupId', 'artifactId', 'version'))
        if not (group and module and version): return [], ''
    return [], ''

def license_files(z):
    found=[]
    for n in sorted(z.namelist()):
        if LICENSE_FILE.match(n): found.append(z.read(n).decode('utf-8','replace'))
    if 'classes.jar' in z.namelist():
        with zipfile.ZipFile(io.BytesIO(z.read('classes.jar'))) as inner:
            for n in sorted(inner.namelist()):
                if LICENSE_FILE.match(n): found.append(inner.read(n).decode('utf-8','replace'))
    return found

by_coordinate={}
for coordinate, license_name, url, _ in sorted(rows):
    by_coordinate.setdefault(coordinate, []).append((license_name, url))

blocks=[]
for coordinate, licenses in by_coordinate.items():
    group,module,version=coordinate.split(':',2)
    names=[n for n,_ in licenses]
    candidates=sorted((cache/group/module/version).glob('*/*.aar')) + sorted(p for p in (cache/group/module/version).glob('*/*.jar') if not re.search(r'-(sources|javadoc)\.jar$', p.name))
    aar=candidates[0] if candidates else None
    if aar is None and module.startswith('play-services-'):
        raise SystemExit(f'AAR が無い: {coordinate}')
    z=zipfile.ZipFile(aar) if aar else None
    body='\n'.join(license_files(z)) if z else ''
    copyright_line=''
    if not body.strip():
        chunks=[]
        need_holders=False
        for license_name, url in licenses:
            spdx=named.get(license_name, license_name)
            if spdx == 'PROPRIETARY':
                # 本文は作文しない。POM が示す許諾の URL を出所つきで書く
                if not url: raise SystemExit(f'許諾の全文が無い: {coordinate} ({license_name}。POM に許諾の URL も無い)')
                chunks.append(f'本文: {url}（出所: {coordinate} の POM の licenses）')
                continue
            need_holders=True
            for part in dict.fromkeys(p for p in re.findall(r'[A-Za-z0-9][A-Za-z0-9.+-]*', spdx) if p not in ('AND','OR')):
                p=pathlib.Path(texts)/(part+'.txt')
                if not p.is_file(): raise SystemExit(f'許諾の全文が無い: {coordinate} ({license_name})')
                chunks.append(p.read_text(encoding='utf-8'))
        body='\n\n'.join(chunks)
        if need_holders:
            holders, origin=pom_holders(group, module, version)
            if not holders:
                raise SystemExit(f'許諾の全文も著作権の表示も無い部品: {coordinate}（部品の中に許諾のファイルが無く、POM と親の POM に developers / organization が無い）')
            copyright_line=f'Copyright: {"; ".join(holders)}（出所: {origin} の POM の developers / organization）\n'
    for n,line in enumerate(body.splitlines(), 1):
        if TEMPLATE.search(line):
            raise SystemExit(f'雛形が残る許諾の本文: {coordinate}（{line.strip()}）')
    display_name=f'{group}:{module}'
    block=f'== {display_name} {version}\nLicense: {" AND ".join(names)}\n{body.strip()}\n{copyright_line}'
    if coordinate in {f'com.google.android.gms:{x}:{version}' for x in ('play-services-location','play-services-base','play-services-basement','play-services-tasks')}:
        if not z or 'third_party_licenses.txt' not in z.namelist() or 'third_party_licenses.json' not in z.namelist():
            raise SystemExit(f'gms の第三者許諾資料が無い: {coordinate}')
        raw=z.read('third_party_licenses.txt').decode('utf-8','replace')
        index=json.loads(z.read('third_party_licenses.json').decode('utf-8'))
        pieces=[raw[v['start']:v['start']+v['length']] for v in index.values()]
        block += '\n(gms の中の第三者)\n' + '\n'.join(pieces).rstrip() + '\n'
    if z: z.close()
    blocks.append(block)
with open(out_path,'w',encoding='utf-8',newline='\n') as out:
    out.write('\n'.join(blocks)+'\n')
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
  apk_self_test "$work"
  rm -rf "$work"; trap - EXIT
}

# 携帯の一覧の自己試験。偽の gradle の cache（aar・jar・POM）と偽の許諾の全文の置き場で組む
apk_self_test() {
  local work="$1/apk" fake_root cache
  fake_root="$work/root"; cache="$work/gradle/caches/modules-2/files-2.1"
  mkdir -p "$fake_root/tools/licenses/texts"
  cp "$root/tools/licenses/texts/Apache-2.0.txt" "$fake_root/tools/licenses/texts/"
  printf 'Fake BSD\n\nCopyright (c) [copyright holder]\nAll rights reserved.\n' > "$fake_root/tools/licenses/texts/BSD-3-Clause.txt"
  printf 'Apache-2.0\tApache-2.0\nBSD-3-Clause\tBSD-3-Clause\n' > "$fake_root/tools/licenses/pom-license-names.tsv"
  python3 - "$cache" <<'PY'
import io, pathlib, sys, zipfile
cache=pathlib.Path(sys.argv[1])
def pom(g, m, v, developers=''):
    d=cache/g/m/v/'0'; d.mkdir(parents=True, exist_ok=True)
    (d/f'{m}-{v}.pom').write_text(f'<project xmlns="http://maven.apache.org/POM/4.0.0"><groupId>{g}</groupId><artifactId>{m}</artifactId><version>{v}</version>{developers}</project>', encoding='utf-8')
    return d
dev='<developers><developer><name>Fake Author</name></developer></developers>'
# classes.jar の中にだけ LICENSE を持つ aar
d=pom('fake', 'inner', '1.0')
classes=io.BytesIO()
with zipfile.ZipFile(classes, 'w') as z: z.writestr('META-INF/LICENSE.txt', 'INNER-LICENSE-MARKER\n')
with zipfile.ZipFile(d/'inner-1.0.aar', 'w') as z: z.writestr('classes.jar', classes.getvalue())
# 許諾のファイルを持たず、POM に作者がある Apache-2.0 の jar
d=pom('fake', 'apache', '1.0', dev)
with zipfile.ZipFile(d/'apache-1.0.jar', 'w') as z: z.writestr('a/A.class', b'')
# 許諾のファイルを持たず、全文が雛形の BSD-3-Clause の jar
d=pom('fake', 'template', '1.0', dev)
with zipfile.ZipFile(d/'template-1.0.jar', 'w') as z: z.writestr('a/A.class', b'')
# 許諾のファイルも POM の作者も無い jar
d=pom('fake', 'nobody', '1.0')
with zipfile.ZipFile(d/'nobody-1.0.jar', 'w') as z: z.writestr('a/A.class', b'')
PY
  local failure
  run_fake_apk() { printf '%s\n' "$@" > "$work/list"; ASHIATO_NOTICES_ROOT="$fake_root" GRADLE_USER_HOME="$work/gradle" ASHIATO_LICENSES_POM_LIST="$work/list" "$0" apk "$work/out"; }
  echo '== Scenario: 携帯の部品の aar の classes.jar の中の許諾のファイルを、依存の一覧に写す'
  rm -f "$work/out"
  run_fake_apk $'fake:inner:1.0\tApache-2.0\thttps://example.invalid/l\tself' >/dev/null || die 'classes.jar の中の許諾のファイルを読めなかった'
  grep -q '^INNER-LICENSE-MARKER$' "$work/out" || die 'classes.jar の中の LICENSE が一覧に無い'
  echo '== Scenario: 許諾のファイルを持たない携帯の部品は、SPDX の全文に POM の作者を添える（Apache-2.0 の付録は雛形とみなさない）'
  rm -f "$work/out"
  run_fake_apk $'fake:apache:1.0\tApache-2.0\thttps://example.invalid/l\tself' >/dev/null || die 'Apache-2.0 の全文の部品を落とした'
  grep -q '^Copyright: Fake Author（出所: fake:apache:1.0 の POM の developers / organization）$' "$work/out" || die 'POM の作者が一覧に無い'
  echo '== Scenario: 雛形の残る許諾の本文の部品があれば、組み立てが落ちる'
  rm -f "$work/out"
  if failure="$(run_fake_apk $'fake:template:1.0\tBSD-3-Clause\thttps://example.invalid/l\tself' 2>&1)"; then die '雛形の残る本文を受け入れた'; fi
  grep -qF '雛形が残る許諾の本文: fake:template:1.0' <<<"$failure" || die "雛形の失敗の理由が無い: $failure"
  [[ ! -e "$work/out" ]] || die '失敗時に一覧を作った'
  echo '== Scenario: 許諾のファイルも POM の作者も無い携帯の部品があれば、組み立てが落ちる'
  if failure="$(run_fake_apk $'fake:nobody:1.0\tApache-2.0\thttps://example.invalid/l\tself' 2>&1)"; then die '著作権の表示の無い部品を受け入れた'; fi
  grep -qF '許諾の全文も著作権の表示も無い部品: fake:nobody:1.0' <<<"$failure" || die "著作権の表示の失敗の理由が無い: $failure"
  [[ ! -e "$work/out" ]] || die '失敗時に一覧を作った'
}

case "${1:-}" in
  --self-test) self_test;;
  commit) commit_value;;
  source) shift; source_notice "$@";;
  server|collector-windows) kind="$1"; out="${2:-}"; [[ -n "$out" ]] || usage; shift 2; target=""; if [[ "${1:-}" == --target ]]; then target="${2:-}"; [[ -n "$target" ]] || usage; shift 2; fi; [[ $# == 0 ]] || usage; rust_notice "$kind" "$out" "$target";;
  apk) shift; apk_notice "$@";;
  *) usage;;
esac
