# CH24 実装タスク — zip の部品を 2 から 9 に上げる

読む順: `proposal.md` → `design.md` → このファイル → `docs/changes/CH24.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `premises.md` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）。実装の後に、実際の diff が予定のパスの中にあることを機械が見る（外に触れると通常の路へ昇格する）。

## Global Constraints

- **触ってよいのは `lane.json` の予定のパスとこの change の中だけ**: `crates/server/Cargo.toml`・`crates/collector-windows/Cargo.toml`・`Cargo.lock`・
  `crates/server/src/archive/open.rs`・`crates/server/src/export/dump.rs`・`crates/server/src/export/ledger.rs`・`crates/collector-windows/src/update_apply.rs`・
  `crates/server/src/{archive_tests,archive_flow_tests,media_export_tests,export_job_tests,export_tests,export_file_tests}.rs`・`crates/collector-windows/tests/st58_pc_update.rs`・
  `osv-scanner.toml`・`tools/licenses/**`・`docs/changes/CH24.md`・`openspec/changes/ch24-zip-upgrade/**`（グラフが置く `docs/briefs/CH24*` と `.superpowers/` は数えない）。
  試験の材料のファイル（`crates/server/tests/fixtures/`）・`tools/check-licenses.sh`（許す許諾の一覧）・CI の workflow・`docs/testing.md` に触れない
- **振る舞いを変えない**（`docs/changes/CH24.md` の「壊してはいけないもの」）。zip を使う 4 か所の直しは新しい API に合わせるだけ。
  `update_apply.rs` の展開を zip の `extract` に置き換えない・権限の写し方を変えない（design D5）。zip の features を絞らない（design D1）
- **試験のファイルは新しい API に合わせる直しだけ**（Task 1 で足す試験を除く）。Scenario の印・試験の名前・1 行の assert の期待値を変えない（2.3 が機械で見る）
- **例外の表で変えるのは zip の 7 行を消すことだけ**（design D3）。ほかの行の理由・期限を変えない・行を足さない。全体の検査を落としている zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）は範囲の外
- **上げきれないときは切り替えない**（design D4）。Task 2 の検証が予定のパスの中の直しで通らなければ、Task 2 を `[x]` にせず、落ちたもの（試験の名前・組み立ての誤り・
  許諾・新しい勧告）と、本人の決めた代わり（7 行の理由を `上げる予定: 2 → 9 の上げの作業が残る` に直し期限を 90 日以内に置く）を添えて A として返す。
  新しく引く依存の許諾が許す一覧に無いときも同じ（一覧を直さない）
- **Task 1 は zip を上げる前に終えて commit する**（埋め込みは zip 2.6.1 で書いたものでなければ意味が無い。1.2 が git で見る）
- 長いコマンド（`cargo test`）は `scripts/quiet-run` で走らせ、1 回で待つ。`cargo test <名前>` は `-- --list` で本数を確かめる

## Task 1: 上げる前の版で書いた書き出しの zip を試験に固定する

- [ ] 1.1 zip 2.6.1 のままの木で、`crates/server/src/export_file_tests.rs` に `specs/data-durability` の 2 つの Scenario の試験を足す
  （名前は `ch24_` で始める: `ch24_previous_zip_export_reads_same`・`ch24_export_aes_fields_match_previous_zip`。`// Scenario: <名前>` の印を逐語で）。
  埋め込み（design D2）: 試験の DB に決まった記録（数件。緯度経度・原文を含む）を置いて今の版で書き出しを 1 回走らせ、得たファイルのバイト列・項目の名前の並び・
  項目ごとの中身の SHA-256・書き出しの id・合言葉を定数（`CH24_PREVIOUS_EXPORT` など）に置く。埋め込みを作る手順（どの記録を置いたか）を試験の補助関数にして、
  2 本目の試験が同じ記録から書き出せるようにする。AES の追加の欄 `0x9901` は zip の部品を使わず、中央ディレクトリ（`PK\x01\x02`）の追加の欄をバイト列から読む小さな関数で取る。
  zip 2.6.1 のまま 2 本とも通ることを確かめて commit する（違う合言葉で読めないことも 1 本目に含める）。
  検証: `scripts/quiet-run ch24a -- cargo test -p ashiato-server --lib ch24_` rc=0（`-- --list` で 2 本以上）
- [ ] 1.2 埋め込みを入れた commit の `Cargo.lock` の zip が 2.6.1 であることを確かめる（上げた後に作り直していないこと）。
  検証: `bash -c 'set -o pipefail; c=$(git log --format=%H -S CH24_PREVIOUS_EXPORT -- crates/server/src/export_file_tests.rs | tail -1); test -n "$c" && git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v"'` rc=0

## Task 2: zip を 9 に上げ、4 か所と試験を新しい API に合わせる

- [ ] 2.1 2 つの `Cargo.toml` の zip を `"9"` にし（既定の features のまま）、`cargo update -p zip` で `Cargo.lock` を更新する。zip を使う 4 か所と 7 つの試験のファイルを、
  新しい API に合わせて直す（意味を変えない）。`Cargo.lock` の zip が 1 つの版だけで 9.0.5 以上であることと、Linux と Windows 向けの clippy が通ることを確かめる。
  検証: `bash -c 'python3 -c "import re;t=open(\"Cargo.lock\").read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert len(v)==1 and tuple(map(int,v[0].split(\".\")))>=(9,0,5),v" && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings && cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings'` rc=0
- [ ] 2.2 上げた後の版で、Task 1 の 2 本（前の版で書いた書き出しを同じ中身に読む・同じ AES の形で書く）が通る。
  検証: `scripts/quiet-run ch24b -- cargo test -p ashiato-server --lib ch24_` rc=0（`-- --list` で 2 本以上）
- [ ] 2.3 書き出し・Takeout の取り込み・PC の更新の既存の試験が全部通り、試験の印・名前・1 行の期待値が base から減っていない。
  検証: `scripts/quiet-run ch24c -- bash -c 'tools/check-tests-kept.sh origin/main crates/server && tools/check-tests-kept.sh origin/main crates/collector-windows && cargo test -p ashiato-server --lib -- export archive media_export'` rc=0（`cargo test -p ashiato-server --lib -- export archive media_export --list` で 449 本以上。base で 447 本（2026-11-20）+ Task 1 の 2 本）
- [ ] 2.4 PC の収集アプリの試験（更新の展開を含む）が全部通る。Windows の実地（`tests/runtime_windows.rs`）は PR の CI の Windows の job が見る（PR 本文にその job の結果を 1 行載せる）。
  検証: `scripts/quiet-run ch24d -- cargo test -p ashiato-collector-windows` rc=0（`cargo test -p ashiato-collector-windows --test st58_pc_update -- --list` で 16 本以上）

## Task 3: 例外の表から zip の行を消し、検査で確かめる

- [ ] 3.1 `osv-scanner.toml` から zip の 7 行（GHSA-m665-v7pm-7grj・GHSA-q7v9-x93c-xqv3・GHSA-8hxf-qr8p-62x4・RUSTSEC-2026-0367・RUSTSEC-2026-0368・RUSTSEC-2026-0369・RUSTSEC-2026-0429）を消す。
  ほかの行は変えない。PR の脆弱性の検査（新しく持ち込んだ勧告が 0 件・表の行の検査が通る・足した・変えた行が `なし`）を走らせる。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; python3 -c "import tomllib,subprocess as s;b=tomllib.loads(s.run([\"git\",\"show\",\"$base:osv-scanner.toml\"],capture_output=True,text=True,check=True).stdout)[\"IgnoredVulns\"];n=tomllib.load(open(\"osv-scanner.toml\",\"rb\"))[\"IgnoredVulns\"];z={\"GHSA-m665-v7pm-7grj\",\"GHSA-q7v9-x93c-xqv3\",\"GHSA-8hxf-qr8p-62x4\",\"RUSTSEC-2026-0367\",\"RUSTSEC-2026-0368\",\"RUSTSEC-2026-0369\",\"RUSTSEC-2026-0429\"};assert [r for r in b if r[\"id\"] not in z]==n,\"zip の 7 行の外が変わった\"" && out=$(tools/check-vulns.sh --base "$base") && printf "%s\n" "$out" && printf "%s\n" "$out" | grep -qx "新しく持ち込んだ: 0 件" && printf "%s\n" "$out" | grep -A1 -x "== 例外の表で足した・変えた行" | grep -qx "なし"'` rc=0
- [ ] 3.2 新しく引いた依存の許諾が許す一覧に収まり、更新係と PC の収集アプリの依存の境界が守られている（許諾の全文が要れば `tools/licenses/texts/` に置く）。
  検証: `bash -c 'tools/check-licenses.sh rust && tools/check-updater-deps.sh'` rc=0

## Task 4: 触ったのが予定のパスだけであることを確かめる

- [ ] 4.1 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。完了の判定 5）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(crates/server/Cargo\.toml|crates/collector-windows/Cargo\.toml|Cargo\.lock|crates/server/src/archive/open\.rs|crates/server/src/export/(dump|ledger)\.rs|crates/collector-windows/src/update_apply\.rs|crates/server/src/(archive_tests|archive_flow_tests|media_export_tests|export_job_tests|export_tests|export_file_tests)\.rs|crates/collector-windows/tests/st58_pc_update\.rs|osv-scanner\.toml|tools/licenses/.*|docs/changes/CH24\.md|openspec/changes/ch24-zip-upgrade/.*|docs/briefs/CH24.*|\.superpowers/.*|)$"'` rc=0
