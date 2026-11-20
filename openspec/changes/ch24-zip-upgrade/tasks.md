# CH24 実装タスク — zip の部品を 2 から 9 に上げる

読む順: `proposal.md` → `design.md` → このファイル → `docs/changes/CH24.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `premises.md` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）で始め、深掘り gate の Q-G1（許す許諾の一覧に `bzip2-1.0.6` を足す。本人の答え 2026-11-20）で通常の路へ昇格した。
予定のパスは `lane.json` の予定のパスに `tools/check-licenses.sh` を足したもので、4.1 が実際の diff で見る。

## Global Constraints

- **触ってよいのは `lane.json` の予定のパスと `tools/check-licenses.sh`（Q-G1）とこの change の中だけ**: `crates/server/Cargo.toml`・`crates/collector-windows/Cargo.toml`・`Cargo.lock`・
  `crates/server/src/archive/open.rs`・`crates/server/src/export/dump.rs`・`crates/server/src/export/ledger.rs`・`crates/collector-windows/src/update_apply.rs`・
  `crates/server/src/{archive_tests,archive_flow_tests,media_export_tests,export_job_tests,export_tests,export_file_tests}.rs`・`crates/collector-windows/tests/st58_pc_update.rs`・
  `osv-scanner.toml`・`tools/licenses/**`・`tools/check-licenses.sh`・`docs/changes/CH24.md`・`openspec/changes/ch24-zip-upgrade/**`（グラフが置く `docs/briefs/CH24*` と `.superpowers/` は数えない）。
  試験の材料のファイル（`crates/server/tests/fixtures/`）・CI の workflow・`docs/testing.md` に触れない
- **`tools/check-licenses.sh` で変えるのは、許す一覧に `bzip2-1.0.6` を足すことだけ**（design D1・`specs/distribution`）。直すのは既定の一覧（`ALLOW`）と、自己試験が一覧の文字列を丸ごと比べる行の 2 か所。
  ほかの許諾を足さない・外さない・台本のほかの行を変えない（3.2 が機械で見る）
- **振る舞いを変えない**（`docs/changes/CH24.md` の「壊してはいけないもの」）。zip を使う 4 か所の直しは新しい API に合わせるだけ。ただし design D6（PC の更新の展開の名前の確かめ）と
  D7（Takeout の項目の名前の読み方）は、2.6.1 の振る舞いを残すために自前のコードを足す（9 の `enclosed_name()`・`name()` の読み方には揃えない）。`update_apply.rs` の展開を zip の `extract` に置き換えない・権限の写し方のコードを変えない（design D5）。
  zip の features を絞らない（design D1）
- **試験のファイルは新しい API に合わせる直しだけ**（Task 1 で足す試験を除く）。Scenario の印・試験の名前・1 行の assert の期待値を変えない（2.3 が機械で見る）
- **試験の材料は zip の部品の外で作ったバイト列を埋め込む**（名前の試験。design の Risks）。書き出しの埋め込みだけは今の版の書き出しで作る（design D2）。
  埋め込みのバイト列と、それを 2.6.1 で読んだ期待値（1.1 の名前・SHA-256・id、1.3 の名前の文字列、1.4 の名前ごとの受け付ける / 断る）は `// CH24-FIXTURE-BEGIN` と `// CH24-FIXTURE-END` の行で囲む
  （1 つのファイルに 1 組。1.5 が上げた後に書き直していないことを見る。Task 2 で直してよい期待値は無い）
- **例外の表で変えるのは zip の 7 行を消すことだけ**（design D3）。ほかの行の理由・期限を変えない・行を足さない。全体の検査を落としている zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）は範囲の外
- **上げきれないときは切り替えない**（design D4）。Task 2 の検証が予定のパスの中の直しで通らなければ、Task 2 を `[x]` にせず、落ちたもの（試験の名前・組み立ての誤り・
  許諾・新しい勧告）と、本人の決めた代わり（7 行の理由を `上げる予定: 2 → 9 の上げの作業が残る` に直し期限を 90 日以内に置く）を添えて A として返す。
  新しく引く依存の許諾が `bzip2-1.0.6` を足した一覧に無いときも同じ（`bzip2-1.0.6` のほかに一覧を足さない）
- **Task 1 は zip を上げる前に終えて commit する**（埋め込みと名前の試験の期待値は zip 2.6.1 の結果でなければ意味が無い。1.5 が git で見る）
- 長いコマンド（`cargo test`）は `scripts/quiet-run` で走らせ、1 回で待つ。試験の本数は検証の中で数えて下限を割ったら落とす（`--list` は 0 本でも rc=0）

## Task 1: 上げる前の版の振る舞いを試験に固定する

- [ ] 1.1 zip 2.6.1 のままの木で、`crates/server/src/export_file_tests.rs` に `specs/data-durability` の Scenario のうち 3 本の試験を足す
  （`ch24_previous_export_reads_same` が「以前の版で書いた書き出しの zip を、同じ中身で読める」、`ch24_previous_export_wrong_password_fails` が
  「以前の版で書いた書き出しの zip は、違う合言葉では読めない」、`ch24_export_aes_shape_matches_previous` が「新しく書いた書き出しの zip の項目は、以前の版と同じ AES の形で書かれる」。
  `// Scenario: <名前>` の印を逐語で）。
  埋め込み（design D2）: 試験の DB に決まった記録（数件。緯度経度・原文を含み、0 行の `.jsonl` と 20 バイト以上の項目の両方が出るように）を置いて今の版で書き出しを 1 回走らせ、
  得たファイルのバイト列・項目の名前の並び・項目ごとの中身の SHA-256・書き出しの id・合言葉を定数（`CH24_PREVIOUS_EXPORT` など）に置き、囲みの行で囲む。
  記録を置く手順を試験の補助関数にして、3 本目の試験と 1.2 が同じ記録から書き出せるようにする。局所ヘッダと中央ディレクトリの圧縮方式・汎用の印・追加の欄 `0x9901` は
  zip の部品を使わずバイト列から読む小さな関数で取る。3 本目の試験は、環境変数 `CH24_DUMP_DIR` があれば書いた zip をそこに `ch24.zip` として写す（2.2 の python3 の確かめが読む）。
  検証: `scripts/quiet-run ch24a -- bash -c 'cargo test -p ashiato-server --lib ch24_ && n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_: $n" && test "$n" -ge 3'` rc=0
- [ ] 1.2 zip 2.6.1 のままの木で、`crates/server/src/export_job_tests.rs` に Scenario「以前の版で書いた書き出しは、次の書き出しを書き終えると消える」の試験
  `ch24_previous_export_replaced_after_next_export` を足す（印を逐語で）。1.1 の埋め込みを置き場に置き、台帳の done の行にその大きさと SHA-256 と書き出しの id（埋め込みの注記の id と同じ）を持たせ、書き出しの仕事を 1 回走らせて、
  置き場に新しい 1 つだけが残ることを見る（design D8）。
  検証: `scripts/quiet-run ch24b -- bash -c 'cargo test -p ashiato-server --lib ch24_ && n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_: $n" && test "$n" -ge 4'` rc=0
- [ ] 1.3 zip 2.6.1 のままの木で、`crates/server/src/archive_tests.rs` に Takeout の項目の名前の読み方の試験を 2 本足す（design D7）:
  `ch24_takeout_unflagged_name_reads_as_cp437`（`Takeout/マイ アクティビティ/a.json` を UTF-8 のバイトで書き、汎用の印の bit 11 を消した zip を `open_archive` で読むと、
  名前が 2.6.1 の CP437 の読み方の文字列になる。同じ書庫に、0x80〜0xFF の全部のバイトを名前に持つ印の無い項目も入れ、CP437 の表の 128 字を全部見る）・
  `ch24_takeout_flagged_name_reads_as_utf8`（bit 11 のある同じ名前は UTF-8 のまま読める）。
  zip は python3 の `zipfile` で作ってから bit 11 を書き換えたバイト列を埋め込む（中央ディレクトリと局所ヘッダの両方の印を消す）。期待値は 2.6.1 で読んだ値をそのまま書き、バイト列と一緒に囲みの中に置く。
  検証: `scripts/quiet-run ch24c -- bash -c 'cargo test -p ashiato-server --lib ch24_takeout_ && n=$(cargo test -p ashiato-server --lib ch24_takeout_ -- --list 2>/dev/null | grep -c ": test$") && echo "ch24_takeout_: $n" && test "$n" -ge 2'` rc=0
- [ ] 1.4 zip 2.6.1 のままの木で、`crates/collector-windows/src/update_apply.rs` の `#[cfg(test)]` に PC の更新の展開の名前の試験を 2 本足す（design D6）:
  `ch24_extract_zip_name_rules` は、名前を 1 つずつ持つ zip（`/etc/passwd`・`C:\x`・`\x`・`a:b`・`../x`・`..\x`・`NUL`・`a.`・`ashiato-collector.exe`・`sub/a.txt`）を `extract_zip` に渡し、
  どれで「zip の道が不正」になるかを 2.6.1 の結果のとおりに見る（Windows と Windows 以外で期待値を `cfg` で分ける。Windows の値は 1.4 の検証の Windows 向けの走りで得る。
  `a\..\..\x`・`ab:c` も並びに入れる。名前と OS ごとの期待値の表は囲みの中に置く）。
  `ch24_extract_zip_reads_info_zip` は、Info-ZIP の `zip -j` で作った小さな zip（配る zip と同じ作り方。`tools/release/build.sh`）を展開し、中身が同じであることを見る。
  zip は python3 の `zipfile` と `/usr/bin/zip` で作ったバイト列を埋め込む。
  検証: `scripts/quiet-run ch24d -- bash -c 'for t in "" "--target x86_64-pc-windows-gnu"; do cargo test -p ashiato-collector-windows --lib $t ch24_ || exit 1; n=$(cargo test -p ashiato-collector-windows --lib $t ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_ ${t:-linux}: $n"; test "$n" -ge 2 || exit 1; done'` rc=0
- [ ] 1.5 埋め込みを入れた commit の `Cargo.lock` の zip が 2.6.1 で、HEAD の囲みの中がその commit と同じであることを確かめる（上げた後に作り直していないこと。3 つのファイルそれぞれ）。
  検証: `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'` rc=0

## Task 2: zip を 9 に上げ、4 か所と試験を新しい API に合わせる

- [ ] 2.1 2 つの `Cargo.toml` の zip を `"9"` にし（既定の features のまま）、`cargo update -p zip` で `Cargo.lock` を更新する。zip を使う 4 か所と 7 つの試験のファイルを、
  新しい API に合わせて直す（意味を変えない）。あわせて、`open.rs` は `name()` を使わず `name_raw()` と `flags().as_u16() & 0x0800` から 2.6.1 と同じ規則で名前を作り（CP437 の表を置く。design D7）、
  `extract_zip` は 9 の `enclosed_name()` を使わず、2.6.1 と同じ名前の確かめを自前の関数で当て、展開先は `destination.join(PathBuf::from(名前))` のままにする（design D6）。
  Task 1 で足した試験の期待値は変えない。
  `Cargo.lock` の zip が 1 つの版だけで 9.0.5 以上であることと、サーバと PC の収集アプリの zip の依存が既定の features のまま（`default-features` を切らない。design D1・Q-G1）であることと、
  Linux と Windows 向けの clippy が通ることを確かめる。
  検証: `bash -c 'set -o pipefail; cargo metadata --format-version 1 --no-deps | python3 -c "import sys,json;m=json.load(sys.stdin);d={p[\"name\"]:[x for x in p[\"dependencies\"] if x[\"name\"]==\"zip\"] for p in m[\"packages\"] if p[\"name\"] in (\"ashiato-server\",\"ashiato-collector-windows\")};print(d);assert len(d)==2 and all(z and all(x[\"uses_default_features\"] for x in z) for z in d.values()),\"zip の既定の features が切られた\"" && python3 -c "import re;t=open(\"Cargo.lock\").read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert len(v)==1 and tuple(map(int,v[0].split(\".\")))>=(9,0,5),v" && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings && cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings'` rc=0
- [ ] 2.2 上げた後の版で、Task 1 のサーバの 6 本（書き出しの 4 本と Takeout の名前の 2 本）が通り、上げた後の書き出しを zip の部品の外の実装（python3 の `zipfile`）で開くと、
  全部の項目が `compress_type == 99` かつ暗号化の bit を持つ（design D2）。
  検証: `scripts/quiet-run ch24e -- bash -c 'd="$TMPDIR/ch24dump"; rm -rf "$d"; mkdir -p "$d"; CH24_DUMP_DIR="$d" cargo test -p ashiato-server --lib ch24_ || exit 1; n=$(cargo test -p ashiato-server --lib ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_: $n"; test "$n" -ge 6 || exit 1; python3 -c "import zipfile,sys;il=zipfile.ZipFile(sys.argv[1]).infolist();print(len(il));assert il and all(i.compress_type==99 and i.flag_bits&1 for i in il),[(i.filename,i.compress_type,i.flag_bits) for i in il]" "$d/ch24.zip"'` rc=0
- [ ] 2.3 書き出し・Takeout の取り込み・PC の更新の既存の試験が全部通り、試験の印・名前・1 行の期待値が base から減っていない。
  検証: `scripts/quiet-run ch24f -- bash -c 'tools/check-tests-kept.sh origin/main crates/server && tools/check-tests-kept.sh origin/main crates/collector-windows && cargo test -p ashiato-server --lib -- export archive media_export || exit 1; n=$(cargo test -p ashiato-server --lib -- export archive media_export --list 2>/dev/null | grep -c ": test$"); echo "export archive media_export: $n（base で 447 本 + Task 1 の 6 本）"; test "$n" -ge 453'` rc=0
- [ ] 2.4 PC の収集アプリの試験（更新の展開を含む）が Linux で全部通る。
  検証: `scripts/quiet-run ch24g -- bash -c 'cargo test -p ashiato-collector-windows || exit 1; n=$(cargo test -p ashiato-collector-windows --test st58_pc_update -- --list 2>/dev/null | grep -c ": test$"); echo "st58_pc_update: $n"; test "$n" -ge 16 || exit 1; m=$(cargo test -p ashiato-collector-windows --lib ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_: $m"; test "$m" -ge 2'` rc=0
- [ ] 2.5 PC の収集アプリの `--lib` の試験（1.4 の名前の試験を含む）が Windows 向けに組んで走らせても全部通る（手元は WSL の interop。CI の Windows の job も同じ `--lib` を走らせる）。
  検証: `scripts/quiet-run ch24h -- bash -c 'cargo test -p ashiato-collector-windows --lib --target x86_64-pc-windows-gnu || exit 1; n=$(cargo test -p ashiato-collector-windows --lib --target x86_64-pc-windows-gnu ch24_ -- --list 2>/dev/null | grep -c ": test$"); echo "ch24_ windows: $n"; test "$n" -ge 2'` rc=0

## Task 3: 例外の表から zip の行を消し、検査で確かめる

- [ ] 3.1 `osv-scanner.toml` から zip の 7 行（GHSA-m665-v7pm-7grj・GHSA-q7v9-x93c-xqv3・GHSA-8hxf-qr8p-62x4・RUSTSEC-2026-0367・RUSTSEC-2026-0368・RUSTSEC-2026-0369・RUSTSEC-2026-0429）を消す。
  ほかの行は変えない。PR の脆弱性の検査（新しく持ち込んだ勧告が 0 件・表の行の検査が通る・足した・変えた行が `なし`）を走らせる。検査が落ちたときも出力を残す。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; python3 -c "import tomllib,subprocess as s;b=tomllib.loads(s.run([\"git\",\"show\",\"$base:osv-scanner.toml\"],capture_output=True,text=True,check=True).stdout)[\"IgnoredVulns\"];n=tomllib.load(open(\"osv-scanner.toml\",\"rb\"))[\"IgnoredVulns\"];z={\"GHSA-m665-v7pm-7grj\",\"GHSA-q7v9-x93c-xqv3\",\"GHSA-8hxf-qr8p-62x4\",\"RUSTSEC-2026-0367\",\"RUSTSEC-2026-0368\",\"RUSTSEC-2026-0369\",\"RUSTSEC-2026-0429\"};assert [r for r in b if r[\"id\"] not in z]==n,\"zip の 7 行の外が変わった\"" || exit 1; out=$(tools/check-vulns.sh --base "$base"); rc=$?; printf "%s\n" "$out"; test $rc -eq 0 && printf "%s\n" "$out" | grep -qx "新しく持ち込んだ: 0 件" && printf "%s\n" "$out" | grep -A1 -x "== 例外の表で足した・変えた行" | grep -qx "なし"'` rc=0
- [ ] 3.2 `tools/check-licenses.sh` の許す一覧の末尾に `bzip2-1.0.6` を足す（既定の一覧 `ALLOW` と、自己試験が一覧の文字列を丸ごと比べる行の 2 か所。design D1・`specs/distribution` の
  Scenario「許可の一覧は、AGPL-3.0-only を外したほかは前と同じ」）。台本のほかの行は変えない。そのうえで、新しく引いた依存の許諾が許す一覧に収まり、許諾の検査の自己試験が通り、
  更新係と PC の収集アプリの依存の境界が守られている（許諾の全文が要れば `tools/licenses/texts/` に置く。`libbz2-rs-sys` は crate に `LICENSE` を持つので要らない見込み）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; git show "$base:tools/check-licenses.sh" | python3 -c "import sys;b=sys.stdin.read();n=open(\"tools/check-licenses.sh\").read();o=\",CC-BY-4.0,Python-2.0\";assert b.count(o)==2,\"base の一覧の形が違う\";assert b.replace(o,o+\",bzip2-1.0.6\")==n,\"一覧に bzip2-1.0.6 を足すほかの変更がある\"" || exit 1; tools/check-licenses.sh rust && tools/check-licenses.sh --self-test && tools/check-updater-deps.sh'` rc=0

## Task 4: 触ったのが予定のパスだけであることを確かめる

- [ ] 4.1 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。完了の判定 5）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(crates/server/Cargo\.toml|crates/collector-windows/Cargo\.toml|Cargo\.lock|crates/server/src/archive/open\.rs|crates/server/src/export/(dump|ledger)\.rs|crates/collector-windows/src/update_apply\.rs|crates/server/src/(archive_tests|archive_flow_tests|media_export_tests|export_job_tests|export_tests|export_file_tests)\.rs|crates/collector-windows/tests/st58_pc_update\.rs|osv-scanner\.toml|tools/licenses/.*|tools/check-licenses\.sh|docs/changes/CH24\.md|openspec/changes/ch24-zip-upgrade/.*|docs/briefs/CH24.*|\.superpowers/.*|)$"'` rc=0
- [ ] 4.2 全部の Task の後に、1.5 と同じ確かめをもう一度走らせる（上げた後の commit で囲みの中が書き直されていないこと。1.5 は上げる前に走るので、上げた後の書き直しはここで見る。完了の判定 2）。
  検証: `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'` rc=0
