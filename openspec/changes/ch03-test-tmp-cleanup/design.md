# CH03 design — テストの一時ファイルの後始末

> 第 1 回の深掘りの推奨の答え（Q1〜Q8）と、聞かずに決めたもの（C1〜C4。`premises.md` の末尾）で先に書いた。
> 本人の答えが推奨と違えば、その Q を `refs` に持つ D を書き直す（各 D の反転条件）。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-10-23 の repo と `/tmp`）:

- Rust: `temp_dir()` は 20 ファイル・46 か所で、全部が試験の側（`*_tests.rs`・`tests/`・`#[cfg(test)]` の中）。漏れ方は 3 通り ——
  一度も消さない（書き出しの `tmp_dir()`。`export_tests.rs:16`、呼び出し 57 か所）/ 通ったときだけ消す（`archive_tests.rs` の `remove_dir_all(root).unwrap()`、
  `outbox.rs`・`outage_notice.rs`・`history/locate.rs` の `.ok()`）/ もう Drop で消す（`archive_flow_tests.rs:36` の `Inbox`、`tests/runtime_windows.rs` の自前の `TempDir`・`Cleanup`）
- Rust の手伝いは `PathBuf` を返す（`export_tests.rs` の `tmp_dir()`・`runtime.rs:855` の `cfg()`・`history/ledger.rs:183` の `temp_dir()`）。書き出しは置き場が無ければ作り直す（`export.rs` の `ensure_dir`）
- Kotlin: `app/src/test` の 10 ファイル。共通の手伝い（`TestOutbox.kt` の `testOutbox()`・`testHeartbeatOutbox()`・`TestStores` の既定の引数、`AppUsageFixtures.kt`）は関数の中で作り、
  呼び出しは 26 ファイル・60 行（`TestStores(`・`AppUsageWorld(`・`St04World(` を含む）。4 つの試験（`SegmentStoreTest`・`OutboxStoreTest`・`ElapsedClockTest`・`SegmentMigrationTest`）は
  property の初期化で直に作り、10 ファイル（`AppUsageFixtures.kt`・`DropReportSendTest.kt`・`UnreadableLineTest.kt`・`SegmentMigrationTest.kt`・`RetentionNotifierTest.kt`・`RetentionTest.kt`・
  `RollupImportTest.kt`・`WriteFailedTest.kt`・`DropReportTest.kt`・`DrainTest.kt`）は手伝いを property の初期化で呼ぶ
- 検査の前例: `tools/check-log-private.sh`（書き方を止める・`// log-ok: <理由>`・`--self-test`・CI の chain job と `tools/pre-commit.sh`）

## Goals / Non-Goals

**Goals:**
- 試験の一時フォルダが、通っても落ちても消える（spec の 1 つ目の Requirement）
- 手書きの一時フォルダが main に入る道を、検査で塞ぐ（2 つ目）
- 早すぎる drop の漏れのように、検査の grep では拾えない漏れを、CI が試験のたびに測る（3 つ目）

**Non-Goals:**
- `tools/` の bash の `mktemp` と、`/tmp` の固定名（`tools/stack.sh:102`・`tools/smoke.sh:118`・`tools/check-licenses.sh:13`、tasks の `tee /tmp/ct.log`）の直しと検査（Q3。固定名の取り合いは別の漏れ方。tasks の書き方は harness2 側）
- いま `/tmp` にある残りの掃除（Q6。試験の名前の頭は汎用の語で、試験でないファイルも消しうる）。harness2 の試験の古い残り（約 1.6G）も CH03 の外
- 殺された試験（SIGKILL）の残り（Q7）
- 失敗した試験の一時フォルダを残す口（Q8）
- 携帯の計測試験（`app/src/androidTest`）と、本物の Windows の `runtime_windows` の残りを測ること（どちらもホストの `/tmp` を使わない。後者は書き換えるが、測らない）
- web の試験（手書きの一時フォルダが無い）

## Decisions

### D1. Rust は `tempfile::TempDir` を試験の最後まで持つ。手伝いは `TempDir` を持つ型を返す（C1）

- `tempfile` を `crates/server` と `crates/collector-windows` の `[dev-dependencies]` に足す（ワークスペースの依存にはしない。製品は使わない）
- 手伝いは `PathBuf` でなく `TempDir`（か、それを持つ型）を返す。呼び出し側は `let dir = tmp_dir("x");` と束縛して試験の最後まで持ち、`dir.path()` を渡す。
  `tmp_dir("x").path().to_path_buf()` のように一時の値から取り出すと、その行で消える。書き出しは置き場を作り直す（`ensure_dir`）ので、試験は落ちずに確かめる経路が変わり、
  作り直した置き場が漏れる —— **この形は検査の grep では拾えず、D6 の台本だけが拾う**
- `runtime.rs` の `cfg()` のように設定の構造体へ置き場を入れる手伝いは、`(Config, TempDir)` か、両方を持つ足場の型を返す。どちらにするかは下流が呼び出しの形で決める
- `archive_flow_tests.rs` の `Inbox` は、`Inbox::new(name)` の形と `root` などの `PathBuf` の欄を変えずに `TempDir` の欄を足す（走っている st13 が `Inbox::new(` を 15 か所・欄を 16 か所使う。
  形を変えると rebase の後にコンパイルが通らない）。`TempDir` の欄は `workers` より後ろに置き、取り込み器が止まってから置き場が消える順にする（いまは自前の Drop が置き場を先に消す）
- もう Drop で消している `archive_flow_tests.rs` の `Inbox` と `tests/runtime_windows.rs` の自前の `TempDir`・`Cleanup` は、`tempfile::TempDir` を持つ形に寄せる（同じ手段に揃え、検査の逃げ道を増やさない）
- 通ったときだけ消す `remove_dir_all(…)` は消す（`TempDir` が消す）
- `TempDir` は Unix で 0700 のフォルダを作る（`create_dir_all` は 0755）。権限を見る試験は自分で権限を付け直す（いま当たる試験は無い。`export_tests.rs:142` は自分で 0755 にしている）
- 名前の頭は残す: `tempfile::Builder::new().prefix("ashiato-export-").tempdir()`。残ったときに出所が分かる
- 退けた案: 試験ごとの `Drop` の型を自前で書く —— 型が試験の数だけ増え、検査がどれを許すかを持てない

### D2. Kotlin は JUnit4 の `@get:Rule` の `TemporaryFolder`。手伝いはフォルダを引数で受け取る（C2）

- 共通の手伝いは `testOutbox(dir: File)`・`testHeartbeatOutbox(dir: File)`・`TestStores(dir: File, …)`（既定の引数を外す）・`AppUsageFixtures` の同じ形にし、
  呼び出し側の試験が `@get:Rule val tmp = TemporaryFolder()` を持って `tmp.newFolder()` を渡す
- property の初期化で作っている 4 つの試験と、手伝いを property の初期化で呼ぶ 10 ファイル（`private val st = TestStores()` など）は、`private val dir by lazy { tmp.newFolder() }`・
  `private val st by lazy { TestStores(tmp.newFolder()) }` か `@Before` に移す。`TemporaryFolder.newFolder()` は規則の `before()` より前に呼ぶと `IllegalStateException` になる（`premises.md`）
- 手伝いの引数が変わるので、走っている st09（`testOutbox()`・`testHeartbeatOutbox()`・`TestStores()` の既定の引数）と st13（`testHeartbeatOutbox()`）は rebase の後にコンパイルが通らない。
  引数を足す以上避けられない（Rust の `Inbox` のように形を残す道が無い）。直しはそれぞれの gate → fix（Q4）
- 退けた案: JVM ごとに 1 つの根を作り、手伝いはその下に作り、JVM の終わりに根を消す（呼び出し側を書き換えずに済む）—— 落ちた試験の後始末が JVM の終わり（shutdown hook）に寄り、
  Gradle の worker が殺されると全部が残る。手伝いの中に `createTempDirectory` が残り、検査の逃げ道になる
- `@After` での削除は採らない（例外の経路を試験ごとに書くことになる。規則なら JUnit が持つ）

### D3（仮）. 検査はファイルの置き場で範囲を決め、`#[cfg(test)]` を読まない（Q1 の推奨）

- 範囲: `git ls-files 'crates/**/*.rs'` の全部と `git ls-files 'collector-android/app/src/test/**/*.kt'`。追跡されていないファイルは見ない（`check-log-private.sh` と同じ。spec に無い実装の決め）
- 製品のコードも縛る。後の Story が製品のコードで一時ファイルを使うときは `tmp-ok: <理由>` を書く
- 反転条件: Q1 の答えが「androidTest も」なら範囲に `app/src/androidTest` を足す（st09 の 2 か所が落ちる）。「試験のコードだけ」なら、Rust のファイルの `#[cfg(test)]` の行より後ろだけを見る
  （`check-log-private.sh` の「`#[cfg(test)]` 以降は見ない」の逆）

### D4（仮）. 落とす書き方と印の形（Q2 の推奨）

| 言語 | 落とす書き方（正規表現の形） | 文言で示すもの |
|---|---|---|
| Rust | `temp_dir\(\)`・`"/tmp`・`\.keep\(\)`・`\.into_path\(\)`・`\.persist\(` | `tempfile::TempDir`（`tempfile::Builder::new().prefix(…).tempdir()`）を試験の最後まで持つ |
| Kotlin | `createTempDirectory`・`createTempFile`・`createTempDir`・`java\.io\.tmpdir`・`"/tmp`・`TemporaryFolder\(` で、同じ行か 1 行上に `@Rule` / `@get:Rule` の無いもの | `@get:Rule val tmp = TemporaryFolder()` と `tmp.newFolder()` |

- 印: 同じ行か 1 行上の `tmp-ok: <理由>`（`//` のコメント）。理由が空の印は数えない。2 行以上離れた印は数えない（`log-ok` の 3 行より狭い。一時フォルダの行は 1 行で済む）。Kotlin の `@Rule` / `@get:Rule` も同じ行か 1 行上まで（spec の Scenario）
- コメントの中の文字列も拾う（doc コメントの中の例も `tmp-ok` が要る。誤検知は印で逃がす。コメントを除く解析を持たない）
- いまの木で印が要る所: `export_file_tests.rs:619`（一時フォルダを**読むだけ**。書き出しが一時フォルダに漏れていないことを見る試験）。
  `config.rs:157`・`autostart.rs:75`（絶対パスの文字列を作るだけ）と `archive_tests.rs:121`（`"/tmp/failing.zip"`。開かない文字列）は、`TempDir` の path に書き換えて印を要らなくする
- `history/ledger.rs` の試験の手伝い `fn temp_dir()`（呼び出し 2 か所）は `temp_dir\(\)` に当たるので、名前を変える（例 `scratch()`）
- 反転条件: Q2 の答えが「依頼の 3 つだけ」なら、Rust は `std::env::temp_dir\(\)`、Kotlin は `createTempDirectory`・`createTempFile` だけにする。誤検知で `tmp-ok` が 1 Story に 3 つを超えて要るなら、その書き方を表から外すかを見直す

### D5（仮）. 検査は CI の chain job と commit 前に掛ける

- `tools/check-tmp-hygiene.sh` は python3 の標準だけで書く（`check-log-private.sh` と同じ形。ツールチェインが要らない）。chain job は出し分けの対象外で毎回走る
- `tools/pre-commit.sh` に `check-log-private.sh` と同じ形で足す（追跡ファイル全体。1 秒かからない）。Actions の予算が尽きている間も効く
- 追跡ファイル全体を見るので、rebase の後の st09・st13 は、漏れを全部直すまでどの commit も commit 前の検査に止まる（直しは 1 つの commit にまとめる。コンパイルの直しも同じ commit に入る）。
  退けた案: staged のファイルだけを見る（`check-private.sh --staged` の形）—— 直さないまま別のファイルを commit でき、CI まで気づかない
- `--self-test` は、一時の木（`mktemp -d` + `trap`）に違反の断片を置いて rc=1、正しい形で rc=0 を見る。台本の中に spec の Scenario の印（`echo "Scenario: …"`）を置く
- 反転条件: commit 前の検査が rebase の後の直しを 2 回以上妨げたら、commit 前は staged だけを見る形に替える。chain job で落ちた PR が、rust / android の job より後に気づくことで直しが遅れる、が 2 回起きたら、rust job にも掛ける

### D6（仮）. 残りの測り方: 一時フォルダを `/tmp` の下の専用の空のフォルダへ向け（落ちた回は `target/tmp-leak/` へ移す）、試験の数も見る（Q5 の推奨）

- `tools/tmp-leak-check.sh rust [cargo test の引数…]`（引数が無ければ `--workspace`、あればその引数だけ）/ `tools/tmp-leak-check.sh android [gradle の引数…]` / `--self-test`。
  一時フォルダの根は `/tmp/ashiato-tmp-leak-<rust|android>-<日時>-<乱数>`（tmpfs）。`mktemp -d` で新しく作る（既にある名前・symlink を使わず 0700。
  決まった名前を `mkdir -p` すると、同じ機械の別の利用者が先に置いた symlink の先へ試験が書く。review R23）。
  通れば消し、落ちた・残りがあった・中断した（INT / TERM。R24）ときだけ `target/tmp-leak/<rust|android>-<日時>-<乱数>` へ移して残す
  （`target/` は gitignore 済み。/tmp を食わない）。残すのは種類ごとに新しい方から 5 個まで（際限なく育てない。R33）。
  **ディスクに根を置かない**（2026-10-27 実測で訂正。初めは `target/tmp-leak/` に置いていた）: 置き場に書く試験が 10 倍以上遅くなる
  （collector-windows の 283 本が /tmp で 5.7 秒、`target/` で 63〜90 秒）。その負荷で、見回り 1 回が 1 秒未満を見る
  `history_large_fetch_keeps_observing_foreground` が毎回落ち、8.2 が通らなかった
- **Rust**: `cargo test --no-run <引数>` で先に組み（rustc の一時ファイルを根に入れない）、`TMPDIR=<根> cargo test <引数>` で走らせる。
  走った本数は出力の `test result: ok. <N> passed` の N の合計
- **Kotlin**: `ASHIATO_TEST_TMPDIR=<根> ./gradlew :app:testDebugUnitTest --rerun "$@"`。`build.gradle.kts` の `unitTests.all` に
  `System.getenv("ASHIATO_TEST_TMPDIR")?.let { test.systemProperty("java.io.tmpdir", it) }` を足す（環境変数が無ければ何も変えない。`ASHIATO_ROBOLECTRIC_JARS` と同じ形）。
  OpenJDK の Unix の `java.io.tmpdir` は `TMPDIR` を読まない（`premises.md`、確かさ 1 → Task 1 で走らせて確かめる）。`--rerun` は `UP-TO-DATE` で走らないことを防ぐ。
  走った本数は `app/build/test-results/testDebugUnitTest/*.xml` の `tests` の合計
- 判定: 試験の命令の rc が 0 でなければその rc で終わる。0 なら、走った本数が 0 → rc=1（「走った試験が 0 本」）、根に何か残る → rc=1（残った名前を最大 20 件と総数）、どちらでもない → rc=0（本数を出す）。
  rc=0 のときは根を消す。rc≠0 のときは `target/tmp-leak/` へ移して残す（中身を見て直すため。`/tmp` を食わない）。試験の rc をそのまま返すことは spec の Scenario。名前の件数・根を残すかは spec に無い実装の決め
- 試験の命令は環境変数 `TMP_LEAK_CMD` で差し替えられる（`--self-test` が偽の命令を渡す。本数は偽の命令が出す同じ形の行から数える）
- CI: rust job の `- run: cargo test --workspace` を `- run: ./tools/tmp-leak-check.sh rust` に、android job の `- run: ./gradlew :app:testDebugUnitTest` を
  `- run: ../tools/tmp-leak-check.sh android` に替える。試験を 2 回走らせない。chain job で `tools/tmp-leak-check.sh --self-test` も走らせる
- 反転条件: Robolectric か Gradle の worker が、ashiato の試験と関係なく `java.io.tmpdir` に自分のものを残すと分かったら（Task 1 で見る）、台本に「道具が残すものの名前の頭」の一覧を
  理由つきで持ち、それだけを判定から外す（ashiato の試験の頭とは被らない名前に限る）。Rust で `cargo` か試験の道具が根に残すと分かったときも同じ。
  CI の時間が 10% を超えて伸びたら（`--rerun` の分）、android は `--rerun` をやめて試験の結果の XML が今回の時刻であることで代える

### D7. 試験の期待値・`Scenario:` の印・試験の名前は変えない（C4）

- 書き換えるのは一時フォルダの作り方・持ち方・後始末だけ。assert の行・`// Scenario:` の行・`fn` / `fun` の試験の名前は差分に出さない
- 機械で見る（tasks の Task 8.3）: `git diff origin/main...HEAD` の消した行にある `Scenario:` の印と `fn` / `fun` の名前が全部足した行にもあり（消してよいのは `ledger.rs` の `fn temp_dir` と、
  `runtime_windows.rs` の自前の型の `fn new`・`fn drop`・`fn path` だけ）、`#[test]`・`#[tokio::test]` と `@Test` の数が `origin/main` より減っていない（CH03 が足す試験の分だけ増える）
- spec の Scenario にしない: これは CH03 の差分の性質で、後も続く振る舞いではない（後の change は試験の期待値を変えてよい）。CH03.md の「壊してはいけないもの」に書き、Task 8.3 が固定する
- 「試験が使う一時フォルダは、その試験が最後に使うまで在り続ける」（D1 の早すぎる drop）も Scenario にしない。書き出しが置き場を作り直す経路は spec の 1 本目の Scenario（空のまま）で落ち、
  作り直さない経路は試験そのものが落ちる（置き場が無い）
- 新しく足す試験（spec の panic と例外の Scenario）は名前の頭を `ch03_` にする

### D8. 何を使えばよいかを `docs/testing.md` に 1 節で書く（C3）

- §1 の後ろに「一時フォルダ」の節を足し、D1・D2 の形・`tmp-ok:` の印・`tools/tmp-leak-check.sh` の走らせ方を書く（検査の文言と同じこと）。§8 の一覧に 2 つの台本を足す
- `docs/flow-gates.md` の検査の表に `tools/check-tmp-hygiene.sh` の行を足す（`check-log-private.sh` の行と同じ形）

### D9（仮）. 負荷で揺れる `archive_scan_interval_change_takes_effect_without_restart` は CH03 で直さない（review R21）

- CH03 の差分がこの試験に入れたのは一時フォルダの作り方と後始末の行だけ（`git diff origin/main...HEAD -- crates/server/src/archive_scan_interval_tests.rs`）で、
  時計・間隔・期待値は変えていない。単独（`tools/tmp-leak-check.sh rust -p ashiato-server --lib archive_scan_interval`）では 2 回とも通り、
  全体の走りで負荷（load average 3.9）の下だけ「走査が N 回に届かない」で落ちる。CH03 が無くても裸の `cargo test --workspace` で同じに落ちる
- だから CH03 の merge は止めず、8.2 の rust の走りの赤はこの 1 本のときに限り揺れとして扱う。直すのは別の change（PR 本文の申し送り。`hx change` で起票する）
- 反転条件: CH03 の分岐点（`origin/main`）の木で、同じ負荷の全体の走りが 3 回とも通るなら、CH03 の差分（根を tmpfs に置く・試験の並びが変わる）が原因として CH03 で直す

## Risks / Trade-offs

- [走っている st09・st13 が rebase の後に検査で落ちる] → 落ちたときの文言が何を使えばよいかを書く。直しはそれぞれの gate → fix（Q4 の推奨）。本文の衝突は proposal の Impact のファイルで起きうる。後から merge する側が追従する
- [手伝いの早すぎる drop が、試験を落とさずに確かめる経路を変える] → D6 の台本が CI で毎回拾う（作り直した置き場が根に残る）
- [`TempDir` の 0700 が、権限を見る試験を変える] → いま当たる試験は無い。当たったら試験が自分で付け直す（D1）
- [検査の誤検知（コメントや文字列の中）] → `tmp-ok: <理由>` で逃がす（D4）。逃がした数が増えたら D4 の反転条件
- [`java.io.tmpdir` を向けられない・Robolectric が残す] → Task 1 が最初に確かめる（D6 の反転条件）。外れたら Task 1 の検証が落ちる
- [殺された試験の残りは残る] → この change の外（Q7）。台本ごと殺されると根は `/tmp/ashiato-tmp-leak-*` に残る（落ちた回は `target/tmp-leak/` へ移すので `/tmp` を食わない）
- [失敗した試験の中身を後から覗けない] → もう一度走らせて再現する（Q8）。CI の台本は残りがあれば根を消さない（D6）が、それは漏れの調べ用で、落ちた試験の中身の保存ではない

## Migration Plan

移行も配布も無い。CI の 3 job と `tools/pre-commit.sh` が変わる。戻すときは `tools/tmp-leak-check.sh rust` / `android` の行を元の `cargo test --workspace` / `./gradlew :app:testDebugUnitTest` に戻し、
chain job と `pre-commit.sh` の検査の行を外す（試験の書き換えはそのまま残してよい）。
