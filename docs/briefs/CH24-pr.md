# CH24 —— zip の部品を 2 から 9 に上げ、例外の表の理由を事実に合わせる

change: `openspec/changes/ch24-zip-upgrade/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D2（仮）. 前の版で書いた書き出しは、試験のファイルにバイト列として埋め込む
- D7（仮）. Takeout の書庫の項目の名前は、2.6.1 と同じ規則で読む

全 Task の ledger に `Ruling:` の行は無い。

## Task

4 Task のうち、埋めた Task は 4 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `scripts/quiet-run ch24a -- bash -c 'cargo test -p ashiato-server --lib ch24_ && n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_: $n" && test "$n" -ge 3'` → rc=0（PASS・head 1ec847a）
- 1.2 `scripts/quiet-run ch24b -- bash -c 'cargo test -p ashiato-server --lib ch24_ && n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_: $n" && test "$n" -ge 4'` → rc=0（PASS・head 1ec847a）
- 1.3 `scripts/quiet-run ch24c -- bash -c 'cargo test -p ashiato-server --lib ch24_takeout_ && n=$(cargo test -p ashiato-server --lib ch24_takeout_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_takeout_: $n" && test "$n" -ge 2'` → rc=0（PASS・head 4bd5e92）
- 1.4 `scripts/quiet-run ch24d -- bash -c 'for t in "" "--target x86_64-pc-windows-gnu"; do cargo test -p ashiato-collector-windows --lib $t ch24_ || exit 1; n=$(cargo test -p ashiato-collector-windows --lib $t ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_ ${t:-linux}: $n"; test "$n" -ge 2 || exit 1; done'` → rc=0（PASS・head 4bd5e92）
- 1.5 `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'` → rc=1（FAIL・head 1ec847a）
- 1.5 `./openspec/changes/ch24-zip-upgrade/check-fixtures.sh` → rc=0（PASS・head 1ec847a）
- 2.1 `bash -c 'set -o pipefail; cargo metadata --format-version 1 --no-deps | python3 -c "import sys,json;m=json.load(sys.stdin);d={p[\"name\"]:[x for x in p[\"dependencies\"] if x[\"name\"]==\"zip\"] for p in m[\"packages\"] if p[\"name\"] in (\"ashiato-server\",\"ashiato-collector-windows\")};print(d);assert len(d)==2 and all(z and all(x[\"uses_default_features\"] for x in z) for z in d.values()),\"zip の既定の features が切られた\"" && python3 -c "import re;t=open(\"Cargo.lock\").read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert len(v)==1 and tuple(map(int,v[0].split(\".\")))>=(9,0,5),v" && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings && cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings'` → rc=0（PASS・head 4bd5e92）
- 2.2 `scripts/quiet-run ch24e -- bash -c 'd="$TMPDIR/ch24dump"; rm -rf "$d"; mkdir -p "$d"; CH24_DUMP_DIR="$d" cargo test -p ashiato-server --lib ch24_ || exit 1; n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_: $n"; test "$n" -ge 6 || exit 1; python3 -c "import zipfile,sys;il=zipfile.ZipFile(sys.argv[1]).infolist();print(len(il));assert il and all(i.compress_type==99 and i.flag_bits&1 for i in il),[(i.filename,i.compress_type,i.flag_bits) for i in il]" "$d/ch24.zip"'` → rc=0（PASS・head a8eb371）
- 2.2 `scripts/quiet-run ch24e -- bash -c 'd="$TMPDIR/ch24dump"; rm -rf "$d"; mkdir -p "$d"; CH24_DUMP_DIR="$d" cargo test -p ashiato-server --lib ch24_ || exit 1; n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_: $n"; test "$n" -ge 6 || exit 1; python3 -c "import zipfile,sys;il=zipfile.ZipFile(sys.argv[1]).infolist();print(len(il));assert il and all(i.compress_type==99 and i.flag_bits&1 and (lambda p:i.extra[p+6:p+8]==b\"AE\" and i.extra[p+8]==3 and i.extra[p+9:p+11]==b\"\\x08\\x00\")(i.extra.index(b\"\\x01\\x99\\x07\\x00\")) for i in il),[(i.filename,i.compress_type,i.flag_bits) for i in il]" "$d/ch24.zip"'` → rc=0（PASS・head 4bd5e92）
- 2.3 `scripts/quiet-run ch24f -- bash -c 'tools/check-tests-kept.sh origin/main crates/server && tools/check-tests-kept.sh origin/main crates/collector-windows && cargo test -p ashiato-server --lib -- export archive media_export || exit 1; n=$(cargo test -p ashiato-server --lib -- export archive media_export --list 2>/dev/null | grep -c ": test$"); echo "export archive media_export: $n（base で 447 本 + Task 1 の 6 本）"; test "$n" -ge 453'` → rc=0（PASS・head a8eb371）
- 2.4 `scripts/quiet-run ch24g -- bash -c 'cargo test -p ashiato-collector-windows || exit 1; n=$(cargo test -p ashiato-collector-windows --test st58_pc_update -- --list 2>/dev/null | grep -c ": test$"); echo "st58_pc_update: $n"; test "$n" -ge 16 || exit 1; m=$(cargo test -p ashiato-collector-windows --lib ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_: $m"; test "$m" -ge 2'` → rc=0（PASS・head 4bd5e92）
- 2.5 `scripts/quiet-run ch24h -- bash -c 'cargo test -p ashiato-collector-windows --lib --target x86_64-pc-windows-gnu || exit 1; n=$(cargo test -p ashiato-collector-windows --lib --target x86_64-pc-windows-gnu ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_ windows: $n"; test "$n" -ge 2'` → rc=0（PASS・head 4bd5e92）
- 3.1 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; python3 -c "import tomllib,subprocess as s;b=tomllib.loads(s.run([\"git\",\"show\",\"$base:osv-scanner.toml\"],capture_output=True,text=True,check=True).stdout)[\"IgnoredVulns\"];n=tomllib.load(open(\"osv-scanner.toml\",\"rb\"))[\"IgnoredVulns\"];z={\"GHSA-m665-v7pm-7grj\",\"GHSA-q7v9-x93c-xqv3\",\"GHSA-8hxf-qr8p-62x4\",\"RUSTSEC-2026-0367\",\"RUSTSEC-2026-0368\",\"RUSTSEC-2026-0369\",\"RUSTSEC-2026-0429\"};assert [r for r in b if r[\"id\"] not in z]==n,\"zip の 7 行の外が変わった\"" || exit 1; out=$(tools/check-vulns.sh --base "$base"); rc=$?; printf "%s\n" "$out"; test $rc -eq 0 && printf "%s\n" "$out" | grep -qx "新しく持ち込んだ: 0 件" && printf "%s\n" "$out" | grep -A1 -x "== 例外の表で足した・変えた行" | grep -qx "なし"'` → rc=0（PASS・head 8a3f055）
- 3.2 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; git show "$base:tools/check-licenses.sh" | python3 -c "import sys;b=sys.stdin.read();n=open(\"tools/check-licenses.sh\").read();o=\",CC-BY-4.0,Python-2.0\";assert b.count(o)==2,\"base の一覧の形が違う\";assert b.replace(o,o+\",bzip2-1.0.6\")==n,\"一覧に bzip2-1.0.6 を足すほかの変更がある\"" || exit 1; tools/check-licenses.sh rust && tools/check-licenses.sh --self-test && tools/check-updater-deps.sh'` → rc=0（PASS・head 8a3f055）
- 4.1 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(crates/server/Cargo\.toml|crates/collector-windows/Cargo\.toml|Cargo\.lock|crates/server/src/archive/open\.rs|crates/server/src/export/(dump|ledger)\.rs|crates/collector-windows/src/update_apply\.rs|crates/server/src/(archive_tests|archive_flow_tests|media_export_tests|export_job_tests|export_tests|export_file_tests)\.rs|crates/collector-windows/tests/st58_pc_update\.rs|osv-scanner\.toml|tools/licenses/.*|tools/check-licenses\.sh|docs/changes/CH24\.md|openspec/changes/ch24-zip-upgrade/.*|docs/briefs/CH24.*|\.superpowers/.*|)$"'` → rc=0（PASS・head 3964d1d）
- 4.2 `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'` → rc=1（FAIL・head 3964d1d）
- 4.2 `./openspec/changes/ch24-zip-upgrade/check-fixtures.sh` → rc=0（PASS・head 3964d1d）

## レビュー（`review/code.md`）

- 指摘 19 件（deferred 1・fixed 15・rejected 3）
- 処置の re-review: 3 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

- R5. collector の `clock::tests` の 2 本が、windows-gnu の `--lib`（tasks 2.5・CI の Windows の job と同じ）で 1 回落ち、走らせ直すと通った —— deferred ST05 — CH24 が触れていない `clock::tests` の揺れであり、閉じた ST05 の時計試験を新しい change で原因から扱う。CH24 の Windows `--lib` は再実行で通り、CH24 の名前試験 2 本も通った。
