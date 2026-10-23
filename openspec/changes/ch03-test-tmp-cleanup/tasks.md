# CH03 実装タスク — テストの一時ファイルの後始末

読む順: `deep.md`（**本人の答え Q1〜Q8**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/test-hygiene/spec.md` → `design.md` → `premises.md`（末尾の C1〜C4）→
`docs/changes/CH03.md` → `tools/check-log-private.sh`（検査の前例）→ `CLAUDE.md`。

**製品のコードは変えない**（依頼の「触らない」。`temp_dir()` を持つ製品のコードは 0 か所）。**試験の期待値・`// Scenario:` の印・試験の名前は変えない**（design D7）。
書き換えるのは一時フォルダの作り方・持ち方・後始末だけ。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin はコメント（`// Scenario: Rust の試験が panic しても、試験の一時フォルダは消える`）、bash は `echo "Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 16 本**（`test-hygiene` の ADDED 3 つ: 3 本・8 本・5 本）。**16 本とも、このファイルのどこかの Task が名指しする**
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
    `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` が、その項目に書いた本数以上
  - **`CW <名前>`** は同じ形で `-p ashiato-collector-windows --lib`（DB は要らない）
  - **`LEAK <引数…>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run leak -- tools/tmp-leak-check.sh <引数…>'` rc=0
  - **`KT <絞り込み>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --rerun --tests '<絞り込み>'` rc=0
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので `-- --list` の下限を必ず見る）
- **新しく足す試験の名前の頭は `ch03_`**（design D7）
- **Rust**: 一時フォルダは `tempfile::TempDir`（名前の頭は `tempfile::Builder::new().prefix("ashiato-…-").tempdir()` で残す）。手伝いは `TempDir`（か、それを持つ型）を返し、
  呼び出し側は**束縛して試験の最後まで持つ**。`tmp_dir("x").path().to_path_buf()` のように一時の値から取り出さない（その行で消え、書き出しが置き場を作り直して漏れる。design D1）。
  通ったときだけ消す `remove_dir_all(…)` は消す
- **Kotlin**: `@get:Rule val tmp = TemporaryFolder()` と `tmp.newFolder()`。共通の手伝いはフォルダを引数で受け取る（既定の引数で作らない）。property の初期化では作らない（`by lazy` か `@Before`。design D2）
- **本人の決定（下流は変えない）**: Q1 検査は Rust の `crates/` 全体と Kotlin の `app/src/test` / Q2 作る書き方の兄弟と後始末を外す口も落とす / Q3 bash と `/tmp` の固定名は直さない /
  Q4 木の全体に当てる / Q5 CI の既存の試験の段で一時フォルダを空のフォルダへ向けて測る / Q6 今ある残りは消さない / Q7 殺された試験の残りは扱わない / Q8 失敗した試験の中身を残す口は持たない。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **`/tmp` を名前の頭で消す台本を書かない**（Q6。試験でないファイルも消しうる）。`tools/` の bash の `mktemp` と固定名には触らない（Q3）
- 走っている st09・st13 のブランチには触らない（rebase の後にそれぞれの gate → fix が直す。Q4）

## Task 1: 足場 —— 依存・試験の JVM の一時フォルダ・残りを測る台本

- [ ] 1.1 `tempfile = "3"` を `crates/server/Cargo.toml` と `crates/collector-windows/Cargo.toml` の `[dev-dependencies]` に足す（collector-windows は `[target.'cfg(windows)'.dev-dependencies]` でなく、全ての target の `[dev-dependencies]` を新しく置く）。
  検証: `bash -c 'cargo tree -p ashiato-server -e dev --depth 1 | grep -q "tempfile v3" && cargo tree -p ashiato-collector-windows -e dev --depth 1 | grep -q "tempfile v3"'` rc=0
- [ ] 1.2 `collector-android/app/build.gradle.kts` の `unitTests.all` に、環境変数 `ASHIATO_TEST_TMPDIR` があるときだけ `test.systemProperty("java.io.tmpdir", it)` を渡す行を足す
  （`ASHIATO_ROBOLECTRIC_JARS` と同じ形。無ければ何も変えない旨をコメントに書く。design D6）。
  検証: `bash -c 'grep -q "ASHIATO_TEST_TMPDIR" collector-android/app/build.gradle.kts && grep -q "java.io.tmpdir" collector-android/app/build.gradle.kts'` rc=0
- [ ] 1.3 `tools/tmp-leak-check.sh` を design D6 のとおりに書く: `rust [cargo test の引数…]`（引数が無ければ `--workspace`。先に `--no-run` で組んでから `TMPDIR=<根>` で走らせる）/
  `android [gradle の引数…]`（`ASHIATO_TEST_TMPDIR=<根>`・`--rerun`・`-p collector-android`。本数は `collector-android/app/build/test-results/testDebugUnitTest/*.xml` の `tests` の合計）/ `--self-test`。
  根は `target/tmp-leak/<rust|android>-<日時>-<pid>`。判定は試験の rc → 本数 0 → 残り、の順。rc=0 なら根を消し、rc=1 なら残す。試験の命令は `TMP_LEAK_CMD` で差し替えられる。
  `--self-test` は偽の命令で次の 4 つを見て、それぞれの前に印を出す:
  `echo "Scenario: 試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる"`（1 つ残して rc=0 → rc=1 と名前）/
  `echo "Scenario: 試験が 1 本も走らなければ、一時フォルダが空でも落ちる"`（本数 0 → rc=1 と「0 本」）/
  `echo "Scenario: 試験が落ちれば、台本も落ちる"`（rc=1 → 0 でない）/
  `echo "Scenario: 試験が通り、何も残さなければ、台本は通る"`（本数 3・残り無し → rc=0 と本数）。冒頭のコメントに使い方・rc・なぜ要るか（CH03）を書く。
  検証: `tools/tmp-leak-check.sh --self-test` rc=0
- [ ] 1.4 前提の確かめ（`premises.md` の確かさ 1 の行。**Task 6 で `SegmentStoreTest` を直す前に**走らせる）: いまの木で、台本が試験の JVM の一時フォルダを向け直せていて、`SegmentStoreTest` の残り（`seg` の頭）を拾うことを見る。
  あわせて、Robolectric か Gradle の worker が根に自分のものを残すかを、出力の残りの名前で見て、残すなら design D6 の反転条件に当たると tasks のこの行の下に書く（直しは Task 6 の終わりまでに）。
  検証: `bash -c 'mkdir -p target; scripts/quiet-run leak -- tools/tmp-leak-check.sh android --tests "*SegmentStoreTest*" > target/ch03-1.4.log 2>&1; rc=$?; [ $rc -eq 1 ] && grep -q "seg" target/ch03-1.4.log'` rc=0

## Task 2: サーバの書き出しの試験と、panic しても消えることの試験

- [ ] 2.1 `crates/server/src/export_tests.rs` の `tmp_dir()` を `TempDir` を返す形にし（名前の頭 `ashiato-export-<name>-` は残す）、呼び出し 57 か所（`export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs`）を
  束縛して最後まで持つ形に直す。`export_file_tests.rs:619` の `std::env::temp_dir()`（一時フォルダを**読むだけ**）は、同じ行か 1 行上に `// tmp-ok: 書き出しが一時フォルダに漏れていないかを読むだけ` を書く（design D4）。
  `crates/server/src/migrate_backup.rs:207` の試験の手伝いも同じ形に直す。
  検証: `CT export_` rc=0（`-- --list` で 99 本以上）/ `CT migrate_backup` rc=0（`-- --list` で 4 本以上）
- [ ] 2.2 `ch03_tmp_dir_is_removed_even_if_the_test_panics` を `export_tests.rs` に足す —— `// Scenario: Rust の試験が panic しても、試験の一時フォルダは消える`。
  `std::panic::catch_unwind` の中で `tmp_dir("ch03")` を受け取り、中にファイルを書き、その path を外へ写してから panic し、外で `catch_unwind` が `Err` であることと、写した path が `exists()` でないことを見る（DB は使わない）。
  検証: `CT ch03_tmp_dir` rc=0（`-- --list` で 1 本以上）
- [ ] 2.3 書き出しの試験が一時フォルダに残さないこと。
  検証: `LEAK rust -p ashiato-server export_` rc=0

## Task 3: サーバの書庫の試験

- [ ] 3.1 `crates/server/src/archive_tests.rs` の 14 か所を `TempDir` に替え、通ったときだけ消す `remove_dir_all(root).unwrap()` を消す。`archive_tests.rs:121` の `"/tmp/failing.zip"`（開かない文字列）は
  `TempDir` の path の下の名前に書き換えて印を要らなくする（design D4）。
  検証: `CT archive_tests` rc=0（`-- --list` で 73 本以上）
- [ ] 3.2 `crates/server/src/archive_flow_tests.rs` の `Inbox` を `tempfile::TempDir` を持つ形に寄せ、自前の `impl Drop for Inbox` の `remove_dir_all` を外す（取り込み器を握る `workers` の Drop の順は変えない。
  `TempDir` は最後に落ちる欄に置く）。
  検証: `CT archive_flow_tests` rc=0（`-- --list` で 105 本以上）
- [ ] 3.3 書庫の試験が一時フォルダに残さないこと。
  検証: `LEAK rust -p ashiato-server archive` rc=0

## Task 4: PC の収集アプリの試験

- [ ] 4.1 `crates/collector-windows/src` の 14 ファイル（`autostart.rs`・`config.rs`・`exclusion.rs`・`fsutil.rs`・`heartbeat.rs`・`history/collect.rs`・`history/ledger.rs`・`history/locate.rs`・
  `history/read.rs`・`marker.rs`・`outage_notice.rs`・`outbox.rs`・`runtime.rs`・`sender.rs`）の `temp_dir()` を `TempDir` に替える。`runtime.rs` の `cfg()` は `Config` と `TempDir` を一緒に返す形
  （`(Config, TempDir)` か足場の型）、`history/ledger.rs` の `temp_dir()` は `TempDir` を返す形にする。`config.rs` の `abs_dir()`・`autostart.rs:75` の絶対パスの文字列は `TempDir` の path から作る。
  `.ok()` / `.unwrap()` の `remove_dir_all` は消す。
  検証: `CW` を絞り込み無しで rc=0（`-- --list` で 283 本以上）/ `LEAK rust -p ashiato-collector-windows --lib` rc=0
- [ ] 4.2 `crates/collector-windows/tests/runtime_windows.rs` の自前の `TempDir`・`Cleanup` の型を `tempfile::TempDir` に寄せる（Windows でだけ組まれる。ホストの `/tmp` は使わないので測らない。design Non-Goals）。
  検証: `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0

## Task 5: 携帯の共通の手伝いと、例外で落ちても消えることの試験

- [ ] 5.1 `collector-android/app/src/test/kotlin/dev/ashiato/collector/TestOutbox.kt` の `testOutbox()`・`testHeartbeatOutbox()` をフォルダを引数で受け取る形にし、`TestStores` の `dir` の既定の引数
  （`Files.createTempDirectory("st04")`）を外す。`AppUsageFixtures.kt`（`UsageTestEnv`）も同じ形にする。呼び出し側の試験（`testOutbox(`・`testHeartbeatOutbox(`・`TestStores(`・`AppUsageWorld(`・`St04World(` を持つ 26 ファイル）に
  `@get:Rule val tmp = TemporaryFolder()` を置いて `tmp.newFolder()` を渡す（design D2）。
  検証: `bash -c '! grep -rnE "createTempDirectory|createTempFile" collector-android/app/src/test/kotlin/dev/ashiato/collector/TestOutbox.kt collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt'` rc=0 /
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --rerun` rc=0
- [ ] 5.2 `Ch03TmpRuleTest.kt` を `app/src/test/kotlin/dev/ashiato/collector/` に足す: `ch03_folder_is_removed_even_if_the_test_throws` —— `// Scenario: 携帯の単体試験が例外で落ちても、試験の一時フォルダは消える`。
  `TemporaryFolder()` を規則として当てた `Statement`（`rule.apply(statement, Description.EMPTY).evaluate()`）の中で `newFolder()` にファイルを書き、そのフォルダを外へ写してから例外を投げ、
  外で例外を受けたことと、写したフォルダが `exists()` でないことを見る。規則を手で当てる行には `// tmp-ok: 規則の後始末を手で当てて確かめる試験` を書く（design D4）。
  検証: `KT *Ch03TmpRuleTest*` rc=0 / `bash -c 'ls collector-android/app/build/test-results/testDebugUnitTest/ | grep -q Ch03TmpRuleTest'` rc=0

## Task 6: 携帯の残りの試験

- [ ] 6.1 property の初期化で作る 4 つ（`SegmentStoreTest`・`OutboxStoreTest`・`ElapsedClockTest`・`SegmentMigrationTest`）を `@get:Rule` の `TemporaryFolder` と `by lazy { tmp.newFolder() }`（か `@Before`）に替える。
  残りの `HeartbeatCountersTest`（4 か所）・`HeartbeatOutboxTest`（2）・`RollupImportTest`・`UnreadableLineTest` も同じにする。
  検証: `bash -c '! grep -rnE "createTempDirectory|createTempFile|createTempDir\(" collector-android/app/src/test'` rc=0 / `KT *SegmentStoreTest*` rc=0
- [ ] 6.2 携帯の単体試験の全部が一時フォルダに残さないこと（Task 1.4 で Robolectric か Gradle が残すと分かっていたら、design D6 の反転条件の一覧を台本に入れてから）。
  検証: `LEAK android` rc=0

## Task 7: 手書きの一時フォルダを落とす検査

- [ ] 7.1 `tools/check-tmp-hygiene.sh` を design D3〜D5 のとおりに書く（python3 の標準だけ・`git ls-files` の `crates/**/*.rs` と `collector-android/app/src/test/**/*.kt`・D4 の表の書き方・
  同じ行か 1 行上の `tmp-ok: <理由>`・落ちたときはファイルと行番号と、言語ごとに何を使えばよいか）。冒頭のコメントに使い方・rc・なぜ要るか（CH03 の実測）を `check-log-private.sh` の形で書く。
  `--self-test` は `mktemp -d` + `trap` の一時の木に断片を置き、次の 7 つを見て、それぞれの前に印を出す:
  `echo "Scenario: Rust の試験で一時フォルダを手で作ると、検査が落ち、TempDir を示す"` /
  `echo "Scenario: 書き方を変えても、検査は素通りさせない"`（5 つの書き方を 1 つずつ）/
  `echo "Scenario: 製品のコードの一時フォルダも、検査が落とす"` /
  `echo "Scenario: 携帯の単体試験で一時フォルダを手で作ると、検査が落ち、TemporaryFolder を示す"`（6 つの書き方を 1 つずつ）/
  `echo "Scenario: 規則の付いた TemporaryFolder と tempfile は、検査を通る"` /
  `echo "Scenario: 理由を書いた tmp-ok の印のある行は通り、理由の無い印は通らない"` /
  `echo "Scenario: 携帯の main と androidTest は、検査の外"`。
  検証: `tools/check-tmp-hygiene.sh --self-test` rc=0
- [ ] 7.2 いまの木が検査を通ること（Task 2〜6 の後）。`--self-test` の最後の段で本物の木に対して本体を走らせ、その前に `echo "Scenario: CH03 の後の木は、検査を通る"` を出す。
  検証: `tools/check-tmp-hygiene.sh` rc=0 / `tools/check-tmp-hygiene.sh --self-test` rc=0
- [ ] 7.3 `tools/pre-commit.sh` に `check-log-private.sh` と同じ形で足し、`.github/workflows/ci.yml` の chain job の `check-log-private.sh` の 2 行の後に
  `- run: ./tools/check-tmp-hygiene.sh --self-test` と `- run: ./tools/check-tmp-hygiene.sh` を足す（コメントに CH03 と何を止めるか 1 行）。
  検証: `bash -c 'grep -q "check-tmp-hygiene.sh" tools/pre-commit.sh && grep -q "./tools/check-tmp-hygiene.sh --self-test" .github/workflows/ci.yml'` rc=0
- [ ] 7.4 `docs/testing.md` に「一時フォルダ」の節（design D8: D1・D2 の形・`tmp-ok:`・`tools/tmp-leak-check.sh` の走らせ方）を足し、§8 の一覧に `./tools/check-tmp-hygiene.sh --self-test && ./tools/check-tmp-hygiene.sh` と
  `./tools/tmp-leak-check.sh --self-test` を足す。§1 の CI の表の `chain` の行に検査を足す。`docs/flow-gates.md` の検査の表に `tools/check-tmp-hygiene.sh` の行を足す。
  検証: `bash -c 'grep -q "TemporaryFolder" docs/testing.md && grep -q "tmp-ok" docs/testing.md && grep -q "tmp-leak-check.sh" docs/testing.md && grep -q "check-tmp-hygiene.sh" docs/flow-gates.md'` rc=0

## Task 8: CI が試験のたびに残りを測る・全体の確かめ

- [ ] 8.1 `.github/workflows/ci.yml` の rust job の `- run: cargo test --workspace` を `- run: ./tools/tmp-leak-check.sh rust` に、android job の `- run: ./gradlew :app:testDebugUnitTest` を
  `- run: ../tools/tmp-leak-check.sh android` に替え（コメントに CH03 と、試験を 2 回走らせない旨）、chain job に `- run: ./tools/tmp-leak-check.sh --self-test` を足す。
  `tools/tmp-leak-check.sh --self-test` の最後に、`ci.yml` の rust job と android job に裸の `cargo test --workspace` / `gradlew :app:testDebugUnitTest` の `run:` が無く、
  `tmp-leak-check.sh rust` / `tmp-leak-check.sh android` があり、chain job に `check-tmp-hygiene.sh --self-test` と `check-tmp-hygiene.sh` があることを見る段を足し、その前に
  `echo "Scenario: CI の試験の段は、残りを測る台本から走る"` を出す。
  検証: `tools/tmp-leak-check.sh --self-test` rc=0
- [ ] 8.2 全体: Rust の試験の全部が一時フォルダに残さないこと（CI の rust job と同じ形）。
  `echo "Scenario: 一時フォルダを空のフォルダへ向けて試験を一通り走らせると、終わった後も空のまま"` の印は `tools/tmp-leak-check.sh` の rust と android の本体の走りの前の出力に置く。
  検証: `LEAK rust` rc=0 / `LEAK android` rc=0
- [ ] 8.3 試験の期待値と印と本数が変わっていないこと（design D7）: `origin/main` からの差分で、消した行に `Scenario:` を含む行が 0、`#[test]`・`#[tokio::test]`・`@Test` の数が減っていない。
  検証: `bash -c 'git fetch -q origin main; [ "$(git diff origin/main...HEAD -- crates collector-android | grep -cE "^-.*Scenario:")" -eq 0 ] && b=$(git grep -hcE "#\[(tokio::)?test" origin/main -- crates | paste -sd+ | bc) && h=$(git grep -hcE "#\[(tokio::)?test" HEAD -- crates | paste -sd+ | bc) && [ "$h" -ge "$b" ] && kb=$(git grep -hc "@Test" origin/main -- collector-android/app/src/test | paste -sd+ | bc) && kh=$(git grep -hc "@Test" HEAD -- collector-android/app/src/test | paste -sd+ | bc) && [ "$kh" -ge "$kb" ]'` rc=0
- [ ] 8.4 Scenario の印と、CI と同じ検査の全部。
  検証: `python3 scripts/check_scenarios.py . ch03-test-tmp-cleanup` rc=0 / `bash -c 'cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings'` rc=0 /
  `scripts/quiet-run detekt -- ./collector-android/gradlew -p collector-android detekt` rc=0 / `bash -c './tools/check-log-private.sh && ./tools/check-shared-appends.sh'` rc=0
